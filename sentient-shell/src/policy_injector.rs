// Policy Injector - Uses trained RL policies to propose system goals
// Integrates with the goal processing pipeline

use anyhow::{Result, Context};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{interval, Duration};
use serde_json::json;

/// Policy injection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyInjectorConfig {
    /// Path to policy checkpoint
    pub checkpoint_path: PathBuf,
    /// Interval between goal injections (seconds)
    pub injection_interval_secs: u64,
    /// Whether to automatically inject goals
    pub auto_inject: bool,
    /// Maximum goals to inject per interval
    pub max_goals_per_interval: usize,
    /// Minimum confidence threshold for injection
    pub confidence_threshold: f32,
    /// Goal priority for injected goals
    pub goal_priority: String,
}

impl Default for PolicyInjectorConfig {
    fn default() -> Self {
        Self {
            checkpoint_path: PathBuf::from("/var/rl_checkpoints/latest.bin"),
            injection_interval_secs: 30,
            auto_inject: false,
            max_goals_per_interval: 1,
            confidence_threshold: 0.7,
            goal_priority: "medium".to_string(),
        }
    }
}

/// System observation for policy
#[derive(Debug, Clone, Serialize)]
pub struct SystemObservation {
    /// CPU usage percentage
    pub cpu_usage: f32,
    /// Memory usage percentage
    pub memory_usage: f32,
    /// Disk usage percentage
    pub disk_usage: f32,
    /// Number of active processes
    pub process_count: usize,
    /// Recent goal success rate
    pub goal_success_rate: f32,
    /// Average goal execution time (ms)
    pub avg_execution_time: f32,
    /// Number of errors in last interval
    pub error_count: usize,
    /// Time since last goal (seconds)
    pub time_since_last_goal: f32,
    /// Current time of day (normalized)
    pub time_of_day: f32,
    /// Day of week (normalized)
    pub day_of_week: f32,
}

/// Goal suggestion from policy
#[derive(Debug, Clone, Serialize)]
pub struct GoalSuggestion {
    pub goal: String,
    pub confidence: f32,
    pub reasoning: String,
    pub expected_reward: f32,
    pub metadata: serde_json::Value,
}

/// Feedback for executed goals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalFeedback {
    pub goal_id: String,
    pub goal: String,
    pub success: bool,
    pub execution_time_ms: u64,
    pub output: Option<String>,
    pub error: Option<String>,
    pub reward: f32,
    pub timestamp: DateTime<Utc>,
}

/// Policy injector service
pub struct PolicyInjector {
    config: PolicyInjectorConfig,
    policy: Arc<RwLock<Option<Box<dyn Policy>>>>,
    is_running: Arc<RwLock<bool>>,
    injection_history: Arc<RwLock<Vec<InjectionRecord>>>,
    feedback_buffer: Arc<RwLock<Vec<GoalFeedback>>>,
}

#[derive(Debug, Clone, Serialize)]
struct InjectionRecord {
    timestamp: DateTime<Utc>,
    goal: String,
    confidence: f32,
    injected: bool,
    feedback: Option<GoalFeedback>,
}

impl PolicyInjector {
    pub fn new(config: PolicyInjectorConfig) -> Self {
        Self {
            config,
            policy: Arc::new(RwLock::new(None)),
            is_running: Arc::new(RwLock::new(false)),
            injection_history: Arc::new(RwLock::new(Vec::new())),
            feedback_buffer: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Load policy from checkpoint
    pub async fn load_policy(&self) -> Result<()> {
        log::info!("Loading policy from: {:?}", self.config.checkpoint_path);
        
        // In real implementation, would deserialize the policy
        // For now, create a mock policy
        let policy = Box::new(MockPolicy::new());
        *self.policy.write().await = Some(policy);
        
        log::info!("Policy loaded successfully");
        Ok(())
    }
    
    /// Start the injector service
    pub async fn start(&self) -> Result<()> {
        if *self.is_running.read().await {
            return Err(anyhow::anyhow!("Policy injector already running"));
        }
        
        // Load policy if not loaded
        if self.policy.read().await.is_none() {
            self.load_policy().await?;
        }
        
        *self.is_running.write().await = true;
        log::info!("Starting policy injector service");
        
        // Start injection loop
        let injector = self.clone();
        tokio::spawn(async move {
            injector.injection_loop().await;
        });
        
        Ok(())
    }
    
    /// Stop the injector service
    pub async fn stop(&self) -> Result<()> {
        *self.is_running.write().await = false;
        log::info!("Stopping policy injector service");
        Ok(())
    }
    
    /// Main injection loop
    async fn injection_loop(&self) {
        let mut interval = interval(Duration::from_secs(self.config.injection_interval_secs));
        
        while *self.is_running.read().await {
            interval.tick().await;
            
            if !self.config.auto_inject {
                continue;
            }
            
            // Get system observation
            let observation = match self.get_system_observation().await {
                Ok(obs) => obs,
                Err(e) => {
                    log::error!("Failed to get system observation: {}", e);
                    continue;
                }
            };
            
            // Get goal suggestions from policy
            let suggestions = match self.get_goal_suggestions(&observation).await {
                Ok(sugg) => sugg,
                Err(e) => {
                    log::error!("Failed to get goal suggestions: {}", e);
                    continue;
                }
            };
            
            // Inject goals
            for (i, suggestion) in suggestions.iter().enumerate() {
                if i >= self.config.max_goals_per_interval {
                    break;
                }
                
                if suggestion.confidence >= self.config.confidence_threshold {
                    if let Err(e) = self.inject_goal(suggestion).await {
                        log::error!("Failed to inject goal: {}", e);
                    }
                }
            }
            
            // Process feedback
            self.process_feedback().await;
        }
    }
    
    /// Get current system observation
    async fn get_system_observation(&self) -> Result<SystemObservation> {
        use sysinfo::{System, SystemExt, CpuExt};
        
        let mut system = System::new_all();
        system.refresh_all();
        
        // Get system metrics
        let cpu_usage = system.global_cpu_info().cpu_usage();
        let total_mem = system.total_memory() as f32;
        let used_mem = system.used_memory() as f32;
        let memory_usage = (used_mem / total_mem) * 100.0;
        
        // Get disk usage (simplified - just root)
        let mut disk_usage = 0.0;
        for disk in system.disks() {
            if disk.mount_point() == Path::new("/") {
                let total = disk.total_space() as f32;
                let available = disk.available_space() as f32;
                disk_usage = ((total - available) / total) * 100.0;
                break;
            }
        }
        
        let process_count = system.processes().len();
        
        // Get goal execution metrics from history
        let history = self.injection_history.read().await;
        let recent_goals: Vec<_> = history.iter()
            .rev()
            .take(10)
            .filter_map(|r| r.feedback.as_ref())
            .collect();
        
        let goal_success_rate = if !recent_goals.is_empty() {
            let successes = recent_goals.iter().filter(|f| f.success).count();
            successes as f32 / recent_goals.len() as f32
        } else {
            0.5 // Default
        };
        
        let avg_execution_time = if !recent_goals.is_empty() {
            let total_time: u64 = recent_goals.iter().map(|f| f.execution_time_ms).sum();
            total_time as f32 / recent_goals.len() as f32
        } else {
            100.0 // Default
        };
        
        let error_count = recent_goals.iter().filter(|f| !f.success).count();
        
        // Time features
        let now = Utc::now();
        let time_since_last_goal = if let Some(last) = history.last() {
            (now - last.timestamp).num_seconds() as f32
        } else {
            300.0 // Default 5 minutes
        };
        
        let time_of_day = (now.hour() as f32 + now.minute() as f32 / 60.0) / 24.0;
        let day_of_week = now.weekday().num_days_from_monday() as f32 / 7.0;
        
        Ok(SystemObservation {
            cpu_usage,
            memory_usage,
            disk_usage,
            process_count,
            goal_success_rate,
            avg_execution_time,
            error_count,
            time_since_last_goal,
            time_of_day,
            day_of_week,
        })
    }
    
    /// Get goal suggestions from policy
    async fn get_goal_suggestions(&self, observation: &SystemObservation) -> Result<Vec<GoalSuggestion>> {
        let policy = self.policy.read().await;
        let policy = policy.as_ref().ok_or_else(|| anyhow::anyhow!("No policy loaded"))?;
        
        // Convert observation to tensor
        let obs_tensor = observation_to_tensor(observation);
        
        // Get action from policy
        let action = policy.predict(&obs_tensor).await?;
        
        // Convert action to goal suggestions
        let suggestions = action_to_goals(action, observation);
        
        Ok(suggestions)
    }
    
    /// Inject a goal into the system
    async fn inject_goal(&self, suggestion: &GoalSuggestion) -> Result<()> {
        let goal_id = uuid::Uuid::new_v4().to_string();
        
        let injection = json!({
            "goal_id": goal_id,
            "goal": suggestion.goal.clone(),
            "source": "rl_policy",
            "timestamp": Utc::now().to_rfc3339(),
            "reasoning": suggestion.reasoning.clone(),
            "priority": self.config.goal_priority,
            "confidence": suggestion.confidence,
            "expected_reward": suggestion.expected_reward,
            "metadata": suggestion.metadata,
            "injected": true,
            "processed": false,
        });
        
        // Write to goal injection file
        let logs_dir = Path::new("logs");
        let injection_file = logs_dir.join("goal_injections.jsonl");
        
        use tokio::fs::OpenOptions;
        use tokio::io::AsyncWriteExt;
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&injection_file)
            .await?;
        
        file.write_all(serde_json::to_string(&injection)?.as_bytes()).await?;
        file.write_all(b"\n").await?;
        
        // Record injection
        let record = InjectionRecord {
            timestamp: Utc::now(),
            goal: suggestion.goal.clone(),
            confidence: suggestion.confidence,
            injected: true,
            feedback: None,
        };
        
        self.injection_history.write().await.push(record);
        
        log::info!("Injected goal: {} (confidence: {:.2})", suggestion.goal, suggestion.confidence);
        
        Ok(())
    }
    
    /// Process feedback from executed goals
    async fn process_feedback(&self) {
        let mut buffer = self.feedback_buffer.write().await;
        let feedback: Vec<_> = buffer.drain(..).collect();
        drop(buffer);
        
        for fb in feedback {
            // Update injection history with feedback
            let mut history = self.injection_history.write().await;
            for record in history.iter_mut().rev() {
                if record.goal == fb.goal && record.feedback.is_none() {
                    record.feedback = Some(fb.clone());
                    break;
                }
            }
            
            // Log feedback for training
            if let Err(e) = self.log_feedback(&fb).await {
                log::error!("Failed to log feedback: {}", e);
            }
        }
    }
    
    /// Log feedback for future training
    async fn log_feedback(&self, feedback: &GoalFeedback) -> Result<()> {
        let feedback_file = Path::new("logs").join("rl_feedback.jsonl");
        
        use tokio::fs::OpenOptions;
        use tokio::io::AsyncWriteExt;
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&feedback_file)
            .await?;
        
        file.write_all(serde_json::to_string(feedback)?.as_bytes()).await?;
        file.write_all(b"\n").await?;
        
        Ok(())
    }
    
    /// Add feedback for an executed goal
    pub async fn add_feedback(&self, feedback: GoalFeedback) {
        self.feedback_buffer.write().await.push(feedback);
    }
    
    /// Get injection statistics
    pub async fn get_stats(&self) -> InjectorStats {
        let history = self.injection_history.read().await;
        let is_running = *self.is_running.read().await;
        
        let total_injections = history.len();
        let successful_injections = history.iter()
            .filter(|r| r.feedback.as_ref().map(|f| f.success).unwrap_or(false))
            .count();
        
        let recent_confidence: Vec<f32> = history.iter()
            .rev()
            .take(10)
            .map(|r| r.confidence)
            .collect();
        
        let avg_confidence = if !recent_confidence.is_empty() {
            recent_confidence.iter().sum::<f32>() / recent_confidence.len() as f32
        } else {
            0.0
        };
        
        InjectorStats {
            is_running,
            total_injections,
            successful_injections,
            success_rate: if total_injections > 0 {
                successful_injections as f32 / total_injections as f32
            } else {
                0.0
            },
            avg_confidence,
            last_injection: history.last().map(|r| r.timestamp),
        }
    }
}

// Make PolicyInjector cloneable for the async spawning
impl Clone for PolicyInjector {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            policy: self.policy.clone(),
            is_running: self.is_running.clone(),
            injection_history: self.injection_history.clone(),
            feedback_buffer: self.feedback_buffer.clone(),
        }
    }
}

/// Injector statistics
#[derive(Debug, Clone, Serialize)]
pub struct InjectorStats {
    pub is_running: bool,
    pub total_injections: usize,
    pub successful_injections: usize,
    pub success_rate: f32,
    pub avg_confidence: f32,
    pub last_injection: Option<DateTime<Utc>>,
}

/// Convert system observation to tensor
fn observation_to_tensor(obs: &SystemObservation) -> Vec<f32> {
    vec![
        obs.cpu_usage / 100.0,
        obs.memory_usage / 100.0,
        obs.disk_usage / 100.0,
        (obs.process_count as f32 / 1000.0).tanh(),
        obs.goal_success_rate,
        (obs.avg_execution_time / 1000.0).tanh(),
        (obs.error_count as f32 / 10.0).tanh(),
        (obs.time_since_last_goal / 300.0).tanh(),
        obs.time_of_day,
        obs.day_of_week,
    ]
}

/// Convert policy action to goal suggestions
fn action_to_goals(action: Vec<f32>, observation: &SystemObservation) -> Vec<GoalSuggestion> {
    let mut suggestions = Vec::new();
    
    // Goal templates based on action indices
    let goal_templates = vec![
        ("Monitor disk I/O activity", "High disk usage detected"),
        ("Check memory usage patterns", "Memory optimization needed"),
        ("Analyze CPU load distribution", "CPU usage requires attention"),
        ("Review network connections", "Network monitoring suggested"),
        ("Scan system logs for errors", "Error detection required"),
        ("Verify service health status", "Service health check needed"),
        ("Check disk space usage", "Disk space monitoring needed"),
        ("Monitor process count", "Process management suggested"),
        ("Analyze system performance", "Performance analysis needed"),
        ("Review security events", "Security monitoring suggested"),
    ];
    
    // Find highest confidence action
    if let Some((idx, &confidence)) = action.iter().enumerate().max_by(|(_, a), (_, b)| {
        a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
    }) {
        if idx < goal_templates.len() {
            let (goal, reasoning) = goal_templates[idx];
            
            // Adjust reasoning based on observation
            let reasoning = format!("{} (CPU: {:.1}%, Mem: {:.1}%, Disk: {:.1}%)",
                reasoning, observation.cpu_usage, observation.memory_usage, observation.disk_usage);
            
            suggestions.push(GoalSuggestion {
                goal: goal.to_string(),
                confidence,
                reasoning,
                expected_reward: confidence * observation.goal_success_rate,
                metadata: json!({
                    "action_idx": idx,
                    "observation": observation,
                }),
            });
        }
    }
    
    suggestions
}

/// Mock policy for testing
struct MockPolicy;

impl MockPolicy {
    fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Policy for MockPolicy {
    async fn predict(&self, observation: &[f32]) -> Result<Vec<f32>> {
        // Simple heuristic policy for testing
        let mut action = vec![0.1; 10];
        
        // Prioritize based on system state
        if observation[0] > 0.8 { // High CPU
            action[2] = 0.9; // Analyze CPU
        } else if observation[1] > 0.8 { // High memory
            action[1] = 0.9; // Check memory
        } else if observation[2] > 0.8 { // High disk
            action[0] = 0.9; // Monitor disk
        } else {
            // Default to system performance analysis
            action[8] = 0.7;
        }
        
        Ok(action)
    }
}

/// Policy trait
#[async_trait::async_trait]
trait Policy: Send + Sync {
    async fn predict(&self, observation: &[f32]) -> Result<Vec<f32>>;
}

/// Global policy injector instance
lazy_static::lazy_static! {
    static ref POLICY_INJECTOR: Arc<RwLock<Option<PolicyInjector>>> = 
        Arc::new(RwLock::new(None));
}

/// Initialize policy injector
pub async fn init_policy_injector(config: PolicyInjectorConfig) -> Result<()> {
    let injector = PolicyInjector::new(config);
    *POLICY_INJECTOR.write().await = Some(injector);
    Ok(())
}

/// Start policy injector
pub async fn start_policy_injector() -> Result<()> {
    let injector = POLICY_INJECTOR.read().await;
    if let Some(inj) = injector.as_ref() {
        inj.start().await?;
    } else {
        return Err(anyhow::anyhow!("Policy injector not initialized"));
    }
    Ok(())
}

/// Stop policy injector
pub async fn stop_policy_injector() -> Result<()> {
    let injector = POLICY_INJECTOR.read().await;
    if let Some(inj) = injector.as_ref() {
        inj.stop().await?;
    }
    Ok(())
}

/// Add goal feedback
pub async fn add_goal_feedback(feedback: GoalFeedback) -> Result<()> {
    let injector = POLICY_INJECTOR.read().await;
    if let Some(inj) = injector.as_ref() {
        inj.add_feedback(feedback).await;
    }
    Ok(())
}

/// Get injector statistics
pub async fn get_injector_stats() -> Option<InjectorStats> {
    let injector = POLICY_INJECTOR.read().await;
    if let Some(inj) = injector.as_ref() {
        Some(inj.get_stats().await)
    } else {
        None
    }
}