use super::SentientService;
use anyhow::{Result, Context};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::time::{sleep, Duration};
use log::{info, warn, error};
use std::path::Path;
use std::fs::OpenOptions;
use std::io::Write;
use rand::seq::SliceRandom;

/// LLM response from Ollama
#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
    #[serde(default)]
    done: bool,
}

/// Goal injection entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalInjection {
    pub goal: String,
    pub source: String,
    pub timestamp: DateTime<Utc>,
    pub reasoning: String,
    pub priority: String,
    pub injected: bool,
    pub processed: bool,
}

/// LLM Observer Service - Periodically injects AI-generated goals
pub struct LlmObserverService {
    name: String,
    interval_ms: u64,
    ollama_url: String,
    logs_dir: String,
    fallback_goals: Vec<String>,
}

impl LlmObserverService {
    pub fn new() -> Self {
        Self {
            name: "llm-observer".to_string(),
            interval_ms: 30000, // 30 seconds default
            ollama_url: "http://192.168.69.197:11434".to_string(),
            logs_dir: "logs".to_string(),
            fallback_goals: vec![
                "Monitor disk I/O activity and report any anomalies".to_string(),
                "Check current memory usage and identify top consumers".to_string(),
                "Analyze network connections for unusual patterns".to_string(),
                "Review system logs for recent errors or warnings".to_string(),
                "Measure CPU load and identify resource-intensive processes".to_string(),
                "Verify all critical services are running properly".to_string(),
                "Check disk space usage across all mounted filesystems".to_string(),
                "Monitor system uptime and last reboot time".to_string(),
                "Analyze process count trends over time".to_string(),
                "Identify potential performance bottlenecks".to_string(),
            ],
        }
    }
    
    /// Query Ollama for goal generation
    async fn query_llm(&self, prompt: &str) -> Result<String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()?;
        
        let payload = json!({
            "model": "deepseek-v2:16b",
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": 0.7,
                "max_tokens": 100
            }
        });
        
        let response = client
            .post(format!("{}/api/generate", self.ollama_url))
            .json(&payload)
            .send()
            .await
            .context("Failed to connect to Ollama")?;
        
        if response.status().is_success() {
            let data: OllamaResponse = response.json().await?;
            Ok(data.response.trim().to_string())
        } else {
            Err(anyhow::anyhow!("Ollama returned status: {}", response.status()))
        }
    }
    
    /// Generate a new goal using LLM or fallback
    async fn generate_goal(&self) -> String {
        let prompt = r#"You are a system monitoring AI. Generate ONE specific, actionable goal for monitoring system health.
Focus on: disk usage, memory, CPU, network, processes, or logs.
Be specific and technical. Output only the goal, nothing else.
Example: "Check disk I/O patterns for the root filesystem"
Goal:"#;
        
        // Try LLM first
        match self.query_llm(prompt).await {
            Ok(goal) if goal.len() >= 10 => {
                info!("🤖 LLM generated goal: {}", goal);
                goal
            }
            Ok(_) => {
                warn!("LLM response too short, using fallback");
                self.use_fallback_goal()
            }
            Err(e) => {
                warn!("LLM query failed: {}, using fallback", e);
                self.use_fallback_goal()
            }
        }
    }
    
    /// Select a random fallback goal
    fn use_fallback_goal(&self) -> String {
        let mut rng = rand::thread_rng();
        let goal = self.fallback_goals
            .choose(&mut rng)
            .unwrap_or(&self.fallback_goals[0])
            .clone();
        
        info!("📋 Using fallback goal: {}", goal);
        goal
    }
    
    /// Inject a goal into the system
    fn inject_goal(&self, goal: &str) -> Result<()> {
        let injection = GoalInjection {
            goal: goal.to_string(),
            source: "llm_observer".to_string(),
            timestamp: Utc::now(),
            reasoning: "AI-generated monitoring goal".to_string(),
            priority: "medium".to_string(),
            injected: true,
            processed: false,
        };
        
        // Write to goal injection file
        let injection_file = Path::new(&self.logs_dir).join("goal_injections.jsonl");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&injection_file)
            .context("Failed to open injection file")?;
        
        writeln!(file, "{}", serde_json::to_string(&injection)?)?;
        
        info!("✓ Injected goal: {}...", &goal[..60.min(goal.len())]);
        
        // Also log to LLM activity
        self.log_activity(&injection)?;
        
        Ok(())
    }
    
    /// Log LLM activity
    fn log_activity(&self, injection: &GoalInjection) -> Result<()> {
        let log_file = Path::new(&self.logs_dir)
            .join(format!("llm_activity_{}.jsonl", Utc::now().format("%Y%m%d")));
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)?;
        
        writeln!(file, "{}", serde_json::to_string(injection)?)?;
        Ok(())
    }
}

#[async_trait]
impl SentientService for LlmObserverService {
    fn name(&self) -> &str {
        &self.name
    }
    
    async fn init(&mut self) -> Result<()> {
        info!("🤖 Initializing LLM Observer Service");
        
        // Create logs directory
        std::fs::create_dir_all(&self.logs_dir)?;
        
        // Load configuration from environment
        if let Ok(interval) = std::env::var("LLM_INTERVAL_MS") {
            self.interval_ms = interval.parse().unwrap_or(30000);
        }
        
        if let Ok(url) = std::env::var("OLLAMA_URL") {
            self.ollama_url = url;
        }
        
        info!("  Injection interval: {}ms", self.interval_ms);
        info!("  Ollama URL: {}", self.ollama_url);
        
        Ok(())
    }
    
    async fn run(&mut self) -> Result<()> {
        info!("✅ LLM Observer Service started");
        
        // Stagger start by 15 seconds to avoid collision with activity loop heartbeat
        sleep(Duration::from_secs(15)).await;
        
        loop {
            info!("🔮 Generating new goal at {}...", Utc::now().format("%H:%M:%S"));
            
            // Generate and inject goal
            let goal = self.generate_goal().await;
            if !goal.is_empty() {
                if let Err(e) = self.inject_goal(&goal) {
                    error!("Failed to inject goal: {}", e);
                }
            }
            
            // Wait for next cycle
            sleep(Duration::from_millis(self.interval_ms)).await;
        }
    }
    
    async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down LLM Observer Service");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_fallback_goal_selection() {
        let service = LlmObserverService::new();
        let goal = service.use_fallback_goal();
        assert!(!goal.is_empty());
        assert!(service.fallback_goals.contains(&goal));
    }
    
    #[test]
    fn test_goal_injection() {
        let temp_dir = TempDir::new().unwrap();
        let mut service = LlmObserverService::new();
        service.logs_dir = temp_dir.path().to_str().unwrap().to_string();
        
        let result = service.inject_goal("Test goal");
        assert!(result.is_ok());
        
        // Check file was created
        let injection_file = temp_dir.path().join("goal_injections.jsonl");
        assert!(injection_file.exists());
        
        // Check content
        let content = std::fs::read_to_string(injection_file).unwrap();
        assert!(content.contains("Test goal"));
        assert!(content.contains("llm_observer"));
    }
}