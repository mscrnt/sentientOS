use super::SentientService;
use anyhow::{Result, Context};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use log::{info, warn, error};
use std::path::Path;
use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;

/// Goal execution entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalEntry {
    pub goal: String,
    pub source: String,
    pub timestamp: DateTime<Utc>,
    pub processed: bool,
    pub command: Option<String>,
    pub output: Option<String>,
    pub success: bool,
    pub reward: f32,
    pub execution_time: f32,
}

/// Goal processor service - executes goals every 5 seconds
pub struct GoalProcessorService {
    name: String,
    goals_queue: Arc<RwLock<Vec<GoalEntry>>>,
    logs_dir: String,
    goal_interval_ms: u64,
    heartbeat_interval_ms: u64,
    last_heartbeat: DateTime<Utc>,
}

impl GoalProcessorService {
    pub fn new() -> Self {
        Self {
            name: "goal-processor".to_string(),
            goals_queue: Arc::new(RwLock::new(Vec::new())),
            logs_dir: "logs".to_string(),
            goal_interval_ms: 5000,
            heartbeat_interval_ms: 60000,
            last_heartbeat: Utc::now(),
        }
    }
    
    /// Map a goal to an executable command
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
        format!("echo 'Goal: {}'", goal.replace("'", "\\'"))
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
        if output.to_lowercase().contains("error") || 
           output.to_lowercase().contains("unavailable") {
            reward -= 0.1;
        }
        
        reward.max(0.0).min(1.0)
    }
    
    /// Execute a single goal
    async fn execute_goal(&self, goal: &GoalEntry) -> (String, String, bool, f32, f32) {
        let start = std::time::Instant::now();
        let command = self.goal_to_command(&goal.goal);
        
        info!("🎯 Executing goal: {} -> {}", goal.goal, command);
        
        let output = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .output();
        
        let execution_time = start.elapsed().as_secs_f32();
        
        match output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                let stderr = String::from_utf8_lossy(&result.stderr);
                let output = if stdout.is_empty() { stderr.to_string() } else { stdout.to_string() };
                let success = result.status.success();
                let reward = self.calculate_reward(&output, success);
                
                (command, output.trim().to_string(), success, reward, execution_time)
            }
            Err(e) => {
                error!("Failed to execute command: {}", e);
                (command, format!("Execution error: {}", e), false, 0.0, execution_time)
            }
        }
    }
    
    /// Load goals from injection file
    async fn load_goals(&self) -> Result<Vec<GoalEntry>> {
        let injection_file = Path::new(&self.logs_dir).join("goal_injections.jsonl");
        if !injection_file.exists() {
            return Ok(Vec::new());
        }
        
        let content = tokio::fs::read_to_string(&injection_file).await?;
        let mut goals = Vec::new();
        
        for line in content.lines() {
            if let Ok(mut entry) = serde_json::from_str::<GoalEntry>(line) {
                if !entry.processed {
                    entry.processed = true;
                    goals.push(entry);
                }
            }
        }
        
        Ok(goals)
    }
    
    /// Write execution log
    async fn write_log(&self, entry: &GoalEntry) -> Result<()> {
        let log_file = Path::new(&self.logs_dir)
            .join(format!("goal_processor_{}.jsonl", Utc::now().format("%Y%m%d")));
        
        let json = serde_json::to_string(entry)?;
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)?;
        
        writeln!(file, "{}", json)?;
        Ok(())
    }
    
    /// System heartbeat
    async fn heartbeat(&mut self) {
        let now = Utc::now();
        if (now - self.last_heartbeat).num_milliseconds() as u64 >= self.heartbeat_interval_ms {
            info!("💓 Goal processor heartbeat - queue size: {}", 
                  self.goals_queue.read().await.len());
            self.last_heartbeat = now;
            
            // Could inject a system health check goal here
            let health_goal = GoalEntry {
                goal: "Check system health and resource usage".to_string(),
                source: "heartbeat".to_string(),
                timestamp: now,
                processed: false,
                command: None,
                output: None,
                success: false,
                reward: 0.0,
                execution_time: 0.0,
            };
            
            self.goals_queue.write().await.push(health_goal);
        }
    }
}

#[async_trait]
impl SentientService for GoalProcessorService {
    fn name(&self) -> &str {
        &self.name
    }
    
    async fn init(&mut self) -> Result<()> {
        info!("🚀 Initializing Goal Processor Service");
        
        // Create logs directory
        create_dir_all(&self.logs_dir)?;
        
        // Load configuration from environment
        if let Ok(interval) = std::env::var("GOAL_INTERVAL_MS") {
            self.goal_interval_ms = interval.parse().unwrap_or(5000);
        }
        
        if let Ok(heartbeat) = std::env::var("HEARTBEAT_INTERVAL_MS") {
            self.heartbeat_interval_ms = heartbeat.parse().unwrap_or(60000);
        }
        
        info!("  Goal interval: {}ms", self.goal_interval_ms);
        info!("  Heartbeat interval: {}ms", self.heartbeat_interval_ms);
        
        Ok(())
    }
    
    async fn run(&mut self) -> Result<()> {
        info!("✅ Goal Processor Service started");
        
        loop {
            // Load new goals
            match self.load_goals().await {
                Ok(new_goals) => {
                    if !new_goals.is_empty() {
                        info!("📥 Loaded {} new goals", new_goals.len());
                        self.goals_queue.write().await.extend(new_goals);
                    }
                }
                Err(e) => {
                    warn!("Failed to load goals: {}", e);
                }
            }
            
            // Process next goal
            let goal = self.goals_queue.write().await.pop();
            
            if let Some(mut goal) = goal {
                let (command, output, success, reward, exec_time) = 
                    self.execute_goal(&goal).await;
                
                // Update goal with results
                goal.command = Some(command);
                goal.output = Some(output.clone());
                goal.success = success;
                goal.reward = reward;
                goal.execution_time = exec_time;
                
                // Log execution
                if let Err(e) = self.write_log(&goal).await {
                    error!("Failed to write log: {}", e);
                }
                
                info!("✓ Goal processed: {} (reward: {:.2})", 
                      &goal.goal[..50.min(goal.goal.len())], reward);
            }
            
            // Heartbeat check
            self.heartbeat().await;
            
            // Sleep until next cycle
            sleep(Duration::from_millis(self.goal_interval_ms)).await;
        }
    }
    
    async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down Goal Processor Service");
        Ok(())
    }
}