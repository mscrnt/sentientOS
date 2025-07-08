use super::SentientService;
use anyhow::{Result, Context};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration, Instant};
use log::{info, warn, error, debug};
use std::path::{Path, PathBuf};
use std::fs::{OpenOptions, create_dir_all};
use std::io::{Write, BufRead, BufReader};

/// Goal execution entry for activity loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityGoalEntry {
    pub goal: String,
    pub source: String,
    pub timestamp: DateTime<Utc>,
    #[serde(default)]
    pub processed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
}

/// Activity execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityResult {
    pub timestamp: DateTime<Utc>,
    pub goal: String,
    pub source: String,
    pub command: String,
    pub output: String,
    pub success: bool,
    pub reward: f32,
    pub execution_time: f32,
}

/// Activity loop service - processes goals every 5 seconds with heartbeat every 60 seconds
pub struct ActivityLoopService {
    name: String,
    check_interval: Duration,
    heartbeat_interval: Duration,
    last_heartbeat: Instant,
    logs_dir: PathBuf,
    processed_goals: Arc<RwLock<HashSet<String>>>,
}

impl ActivityLoopService {
    pub fn new() -> Self {
        Self {
            name: "activity-loop".to_string(),
            check_interval: Duration::from_secs(5),
            heartbeat_interval: Duration::from_secs(60),
            last_heartbeat: Instant::now(),
            logs_dir: PathBuf::from("logs"),
            processed_goals: Arc::new(RwLock::new(HashSet::new())),
        }
    }
    
    /// Convert goal to actual executable command
    fn goal_to_command(&self, goal: &str) -> String {
        let goal_lower = goal.to_lowercase();
        
        // Disk activity/IO
        if goal_lower.contains("disk") && (goal_lower.contains("activity") || 
            goal_lower.contains("i/o") || goal_lower.contains("io") || 
            goal_lower.contains("usage")) {
            return "df -h | grep -E '^/dev/' | head -3 && echo '---' && iostat -d 1 2 2>/dev/null | tail -n +4 | awk 'NR>1 {print $1 \": Read \" $3 \" KB/s, Write \" $4 \" KB/s\"}' || echo 'Disk stats: iostat not available'".to_string();
        }
        
        // Memory usage
        if goal_lower.contains("memory") && (goal_lower.contains("usage") || 
            goal_lower.contains("check") || goal_lower.contains("free")) {
            return "free -h | grep -E '^Mem:' | awk '{print \"Memory: Total \" $2 \", Used \" $3 \", Free \" $4 \", Available \" $7}'".to_string();
        }
        
        // Network activity
        if goal_lower.contains("network") && (goal_lower.contains("activity") || 
            goal_lower.contains("connections") || goal_lower.contains("traffic")) {
            return "netstat -tunl 2>/dev/null | grep LISTEN | wc -l | xargs -I {} echo 'Active listeners: {}' && ss -s 2>/dev/null | grep 'TCP:' || echo 'Network stats unavailable'".to_string();
        }
        
        // CPU usage
        if goal_lower.contains("cpu") && (goal_lower.contains("usage") || 
            goal_lower.contains("load") || goal_lower.contains("check")) {
            return "uptime | awk -F'load average:' '{print \"Load average:\" $2}' && top -bn1 | grep 'Cpu(s)' | head -1 || echo 'CPU stats unavailable'".to_string();
        }
        
        // Process count
        if goal_lower.contains("process") && (goal_lower.contains("count") || 
            goal_lower.contains("running") || goal_lower.contains("check")) {
            return "ps aux | wc -l | xargs -I {} echo 'Total processes: {}'".to_string();
        }
        
        // System health/uptime
        if goal_lower.contains("health") || goal_lower.contains("uptime") || 
            goal_lower.contains("status") {
            return "uptime -p && echo '---' && df -h / | tail -1 | awk '{print \"Root disk: \" $5 \" used\"}' && free -h | grep Mem | awk '{print \"Memory: \" $3 \"/\" $2 \" used\"}'".to_string();
        }
        
        // Log analysis
        if goal_lower.contains("log") && (goal_lower.contains("error") || 
            goal_lower.contains("check") || goal_lower.contains("analyze")) {
            return "find logs -name '*.log' -mtime -1 2>/dev/null | head -5 | xargs -I {} sh -c 'echo \"=== {} ===\" && tail -20 {} | grep -iE \"error|fail|critical\" | tail -5' || echo 'No recent errors in logs'".to_string();
        }
        
        // Service status
        if goal_lower.contains("service") && (goal_lower.contains("status") || 
            goal_lower.contains("check")) {
            return "ps aux | grep -E 'goal|llm|reflect' | grep -v grep | wc -l | xargs -I {} echo 'SentientOS services running: {}'".to_string();
        }
        
        // Default - echo the goal
        format!("echo 'Goal: {}'", goal)
    }
    
    /// Calculate reward based on command output
    fn calculate_reward(&self, output: &str, success: bool) -> f32 {
        if !success {
            return 0.0;
        }
        
        let mut reward = 0.3; // Base reward for successful execution
        
        // Bonus for actual data
        if output.len() > 50 {
            reward += 0.2;
        }
        
        // Bonus for structured output
        if output.contains(':') || output.contains('|') {
            reward += 0.2;
        }
        
        // Bonus for numeric data
        if output.chars().any(|c| c.is_numeric()) {
            reward += 0.2;
        }
        
        // Penalty for errors
        if output.to_lowercase().contains("error") || output.to_lowercase().contains("unavailable") {
            reward -= 0.1;
        }
        
        reward.max(0.0).min(1.0)
    }
    
    /// Execute a command and return output, success, and execution time
    async fn execute_command(&self, command: &str) -> (String, bool, f32) {
        let start_time = Instant::now();
        
        match tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .await
        {
            Ok(output) => {
                let execution_time = start_time.elapsed().as_secs_f32();
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let output_str = if !stdout.is_empty() { 
                    stdout.to_string() 
                } else { 
                    stderr.to_string() 
                };
                
                (output_str.trim().to_string(), output.status.success(), execution_time)
            }
            Err(e) => {
                let execution_time = start_time.elapsed().as_secs_f32();
                (format!("Execution error: {}", e), false, execution_time)
            }
        }
    }
    
    /// Process a single goal and return results
    async fn process_goal(&self, goal_entry: ActivityGoalEntry) -> Result<ActivityResult> {
        let goal = &goal_entry.goal;
        let command = self.goal_to_command(goal);
        
        info!("🎯 Processing: {}...", &goal[..goal.len().min(80)]);
        info!("   Command: {}...", &command[..command.len().min(80)]);
        
        let (output, success, execution_time) = self.execute_command(&command).await;
        let reward = self.calculate_reward(&output, success);
        
        let result = ActivityResult {
            timestamp: Utc::now(),
            goal: goal.clone(),
            source: goal_entry.source,
            command,
            output: output[..output.len().min(500)].to_string(), // Limit output size
            success,
            reward,
            execution_time,
        };
        
        info!("   ✓ Success: {}, Reward: {:.2}", success, reward);
        
        Ok(result)
    }
    
    /// Load unprocessed goals from injection file
    async fn load_goals(&self) -> Result<Vec<ActivityGoalEntry>> {
        let injection_file = self.logs_dir.join("goal_injections.jsonl");
        if !injection_file.exists() {
            return Ok(Vec::new());
        }
        
        let mut goals = Vec::new();
        let file = OpenOptions::new()
            .read(true)
            .open(&injection_file)?;
        
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().collect::<std::io::Result<Vec<_>>>()?;
        
        // Clear the file and rewrite with processed flags
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&injection_file)?;
        
        for line in lines {
            if let Ok(mut entry) = serde_json::from_str::<ActivityGoalEntry>(&line) {
                if !entry.processed {
                    goals.push(entry.clone());
                    entry.processed = true;
                }
                writeln!(file, "{}", serde_json::to_string(&entry)?)?;
            }
        }
        
        Ok(goals)
    }
    
    /// Write execution log
    async fn write_log(&self, entry: &ActivityResult) -> Result<()> {
        let log_file = self.logs_dir.join(format!("activity_loop_log_{}.jsonl", 
            Utc::now().format("%Y%m%d")));
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)?;
        
        writeln!(file, "{}", serde_json::to_string(entry)?)?;
        Ok(())
    }
    
    /// System heartbeat - inject health check goal
    async fn heartbeat(&mut self) -> Result<()> {
        let current_time = Instant::now();
        if current_time.duration_since(self.last_heartbeat) >= self.heartbeat_interval {
            info!("💓 Heartbeat - injecting system health check");
            
            let health_goal = ActivityGoalEntry {
                goal: "Check system health and resource usage".to_string(),
                source: "heartbeat".to_string(),
                timestamp: Utc::now(),
                processed: false,
                priority: Some("low".to_string()),
            };
            
            // Inject directly
            let injection_file = self.logs_dir.join("goal_injections.jsonl");
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&injection_file)?;
            
            writeln!(file, "{}", serde_json::to_string(&health_goal)?)?;
            
            self.last_heartbeat = current_time;
        }
        Ok(())
    }
}

#[async_trait]
impl SentientService for ActivityLoopService {
    fn name(&self) -> &str {
        &self.name
    }
    
    async fn init(&mut self) -> Result<()> {
        info!("🚀 Activity Loop Service initializing");
        
        // Create logs directory if it doesn't exist
        create_dir_all(&self.logs_dir)
            .context("Failed to create logs directory")?;
        
        info!("   Check interval: {:?}", self.check_interval);
        info!("   Heartbeat interval: {:?}", self.heartbeat_interval);
        info!("   Logs directory: {:?}", self.logs_dir);
        
        Ok(())
    }
    
    async fn run(&mut self) -> Result<()> {
        info!("🚀 Activity Loop Service started");
        
        loop {
            match self.run_iteration().await {
                Ok(_) => {},
                Err(e) => {
                    error!("❌ Error in activity loop: {}", e);
                }
            }
            
            // Sleep until next check
            sleep(self.check_interval).await;
        }
    }
    
    async fn shutdown(&mut self) -> Result<()> {
        info!("👋 Shutting down Activity Loop Service");
        Ok(())
    }
}

impl ActivityLoopService {
    /// Run one iteration of the activity loop
    async fn run_iteration(&mut self) -> Result<()> {
        // Load new goals
        let goals = self.load_goals().await?;
        
        if !goals.is_empty() {
            info!("📥 Found {} new goals", goals.len());
            
            // Process each goal
            for goal in goals {
                match self.process_goal(goal).await {
                    Ok(result) => {
                        self.write_log(&result).await?;
                    }
                    Err(e) => {
                        error!("Failed to process goal: {}", e);
                    }
                }
            }
        }
        
        // Heartbeat check
        self.heartbeat().await?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_goal_to_command() {
        let service = ActivityLoopService::new();
        
        // Test disk activity mapping
        let cmd = service.goal_to_command("Check disk activity and I/O");
        assert!(cmd.contains("df -h"));
        assert!(cmd.contains("iostat"));
        
        // Test memory mapping
        let cmd = service.goal_to_command("Check memory usage");
        assert!(cmd.contains("free -h"));
        
        // Test CPU mapping
        let cmd = service.goal_to_command("Monitor CPU load");
        assert!(cmd.contains("uptime"));
        
        // Test default mapping
        let cmd = service.goal_to_command("Unknown goal");
        assert_eq!(cmd, "echo 'Goal: Unknown goal'");
    }
    
    #[test]
    fn test_calculate_reward() {
        let service = ActivityLoopService::new();
        
        // Test failure
        assert_eq!(service.calculate_reward("", false), 0.0);
        
        // Test basic success
        assert_eq!(service.calculate_reward("short", true), 0.3);
        
        // Test with length bonus
        let long_output = "a".repeat(100);
        assert!(service.calculate_reward(&long_output, true) > 0.3);
        
        // Test with structured data
        assert!(service.calculate_reward("key: value | data", true) > 0.5);
        
        // Test with numeric data
        assert!(service.calculate_reward("Count: 42", true) > 0.5);
        
        // Test with error penalty
        assert!(service.calculate_reward("Error: something failed", true) < 0.5);
    }
}