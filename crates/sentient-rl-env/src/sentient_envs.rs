//! SentientOS-specific environments for RL training
//!
//! Provides environments that integrate with the SentientOS goal system,
//! allowing agents to learn from real system interactions.

use anyhow::{Result, Context};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ndarray::Array1;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::sync::RwLock;
use std::sync::Arc;

use sentient_rl_core::{
    Environment, EnvironmentConfig, StepInfo, 
    Observation, Action, Reward, Space, BoxSpace, DiscreteSpace,
};

/// Configuration for JSONL environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JSONLEnvConfig {
    /// Path to JSONL trace file
    pub trace_file: PathBuf,
    /// Maximum episode length
    pub max_episode_length: usize,
    /// Observation dimension
    pub observation_dim: usize,
    /// Action dimension
    pub action_dim: usize,
    /// Reward shaping parameters
    pub reward_config: RewardConfig,
}

/// Reward shaping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardConfig {
    /// Reward for successful goal completion
    pub success_reward: f32,
    /// Penalty for failed goals
    pub failure_penalty: f32,
    /// Penalty for system crashes
    pub crash_penalty: f32,
    /// Small penalty per timestep
    pub step_penalty: f32,
    /// Bonus for efficient completion
    pub efficiency_bonus: f32,
}

impl Default for RewardConfig {
    fn default() -> Self {
        Self {
            success_reward: 1.0,
            failure_penalty: -0.5,
            crash_penalty: -1.0,
            step_penalty: -0.01,
            efficiency_bonus: 0.2,
        }
    }
}

/// Trace entry from JSONL file
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TraceEntry {
    timestamp: DateTime<Utc>,
    goal: String,
    action: String,
    result: TraceResult,
    metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TraceResult {
    success: bool,
    output: Option<String>,
    error: Option<String>,
    execution_time_ms: Option<u64>,
}

/// Environment that replays JSONL traces
pub struct JSONLEnv {
    config: JSONLEnvConfig,
    traces: Arc<RwLock<Vec<TraceEntry>>>,
    current_episode: Arc<RwLock<Vec<TraceEntry>>>,
    current_step: Arc<RwLock<usize>>,
    observation_space: Box<dyn Space>,
    action_space: Box<dyn Space>,
}

impl JSONLEnv {
    /// Create new JSONL environment
    pub async fn new(config: JSONLEnvConfig) -> Result<Self> {
        // Load traces from file
        let traces = Self::load_traces(&config.trace_file).await?;
        
        // Create spaces
        let observation_space = Box::new(BoxSpace::new(
            vec![-1.0; config.observation_dim],
            vec![1.0; config.observation_dim],
        ));
        
        let action_space = Box::new(DiscreteSpace::new(config.action_dim));
        
        Ok(Self {
            config,
            traces: Arc::new(RwLock::new(traces)),
            current_episode: Arc::new(RwLock::new(Vec::new())),
            current_step: Arc::new(RwLock::new(0)),
            observation_space,
            action_space,
        })
    }
    
    /// Load traces from JSONL file
    async fn load_traces(path: &Path) -> Result<Vec<TraceEntry>> {
        let content = fs::read_to_string(path).await
            .context("Failed to read trace file")?;
        
        let mut traces = Vec::new();
        for line in content.lines() {
            if !line.trim().is_empty() {
                let entry: TraceEntry = serde_json::from_str(line)
                    .context("Failed to parse trace entry")?;
                traces.push(entry);
            }
        }
        
        Ok(traces)
    }
    
    /// Convert trace to observation
    fn trace_to_observation(&self, trace: &TraceEntry, step: usize) -> Array1<f32> {
        let mut obs = Array1::zeros(self.config.observation_dim);
        
        // Simple encoding: goal hash, action type, previous results, step counter
        // In practice, would use more sophisticated encoding (e.g., embeddings)
        
        // Goal features (simplified as hash)
        let goal_hash = trace.goal.chars().map(|c| c as u32).sum::<u32>() as f32;
        obs[0] = (goal_hash % 1000.0) / 1000.0;
        
        // Action features
        let action_hash = trace.action.chars().map(|c| c as u32).sum::<u32>() as f32;
        obs[1] = (action_hash % 1000.0) / 1000.0;
        
        // Previous result
        obs[2] = if trace.result.success { 1.0 } else { -1.0 };
        
        // Execution time (normalized)
        if let Some(time_ms) = trace.result.execution_time_ms {
            obs[3] = (time_ms as f32 / 1000.0).tanh();
        }
        
        // Step counter (normalized)
        obs[4] = (step as f32 / self.config.max_episode_length as f32).tanh();
        
        // Fill remaining with noise for now
        for i in 5..self.config.observation_dim {
            obs[i] = (i as f32 * 0.1).sin();
        }
        
        obs
    }
    
    /// Compute reward from trace result
    fn compute_reward(&self, trace: &TraceEntry) -> f32 {
        let mut reward = self.config.reward_config.step_penalty;
        
        if trace.result.success {
            reward += self.config.reward_config.success_reward;
            
            // Efficiency bonus based on execution time
            if let Some(time_ms) = trace.result.execution_time_ms {
                if time_ms < 100 {
                    reward += self.config.reward_config.efficiency_bonus;
                }
            }
        } else {
            reward += self.config.reward_config.failure_penalty;
            
            // Check for crashes
            if let Some(error) = &trace.result.error {
                if error.contains("crash") || error.contains("panic") {
                    reward += self.config.reward_config.crash_penalty;
                }
            }
        }
        
        reward
    }
}

#[async_trait]
impl Environment for JSONLEnv {
    fn observation_space(&self) -> &dyn Space {
        self.observation_space.as_ref()
    }
    
    fn action_space(&self) -> &dyn Space {
        self.action_space.as_ref()
    }
    
    async fn reset(&mut self) -> Result<Observation> {
        let traces = self.traces.read().await;
        
        // Sample random episode from traces
        if traces.is_empty() {
            return Err(anyhow::anyhow!("No traces available"));
        }
        
        // Create episode by sampling consecutive traces
        let start_idx = rand::random::<usize>() % traces.len();
        let episode_length = self.config.max_episode_length.min(traces.len() - start_idx);
        
        let episode: Vec<TraceEntry> = traces[start_idx..start_idx + episode_length]
            .iter()
            .cloned()
            .collect();
        
        *self.current_episode.write().await = episode;
        *self.current_step.write().await = 0;
        
        // Return initial observation
        let episode = self.current_episode.read().await;
        if let Some(first_trace) = episode.first() {
            let obs = self.trace_to_observation(first_trace, 0);
            Ok(Observation::new(obs.to_vec()))
        } else {
            Err(anyhow::anyhow!("Empty episode"))
        }
    }
    
    async fn step(&mut self, action: Action) -> Result<StepInfo> {
        let mut step = self.current_step.write().await;
        let episode = self.current_episode.read().await;
        
        if *step >= episode.len() {
            return Err(anyhow::anyhow!("Episode ended"));
        }
        
        // Get current trace
        let current_trace = &episode[*step];
        
        // Compute reward
        let reward = self.compute_reward(current_trace);
        
        // Increment step
        *step += 1;
        
        // Check if done
        let done = *step >= episode.len() || *step >= self.config.max_episode_length;
        
        // Get next observation
        let observation = if done || *step >= episode.len() {
            // Terminal state
            Observation::new(vec![0.0; self.config.observation_dim])
        } else {
            let next_trace = &episode[*step];
            let obs = self.trace_to_observation(next_trace, *step);
            Observation::new(obs.to_vec())
        };
        
        Ok(StepInfo {
            observation,
            reward: Reward(reward),
            done,
            truncated: *step >= self.config.max_episode_length,
            info: Default::default(),
        })
    }
    
    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Configuration for Goal Task environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalTaskEnvConfig {
    /// Goal templates to use
    pub goal_templates: Vec<String>,
    /// Maximum steps per episode
    pub max_steps: usize,
    /// Observation dimension
    pub observation_dim: usize,
    /// Whether to execute real commands
    pub execute_real_commands: bool,
    /// Command timeout in seconds
    pub command_timeout_secs: u64,
}

impl Default for GoalTaskEnvConfig {
    fn default() -> Self {
        Self {
            goal_templates: vec![
                "Monitor disk I/O activity".to_string(),
                "Check memory usage patterns".to_string(),
                "Analyze CPU load distribution".to_string(),
                "Review network connections".to_string(),
                "Scan system logs for errors".to_string(),
            ],
            max_steps: 50,
            observation_dim: 64,
            execute_real_commands: false,
            command_timeout_secs: 5,
        }
    }
}

/// Environment for goal-based tasks
pub struct GoalTaskEnv {
    config: GoalTaskEnvConfig,
    current_goal: Arc<RwLock<Option<String>>>,
    current_step: Arc<RwLock<usize>>,
    goal_history: Arc<RwLock<VecDeque<GoalExecution>>>,
    observation_space: Box<dyn Space>,
    action_space: Box<dyn Space>,
}

#[derive(Debug, Clone)]
struct GoalExecution {
    goal: String,
    command: String,
    success: bool,
    output: String,
    execution_time: std::time::Duration,
}

impl GoalTaskEnv {
    /// Create new goal task environment
    pub fn new(config: GoalTaskEnvConfig) -> Self {
        let observation_space = Box::new(BoxSpace::new(
            vec![-1.0; config.observation_dim],
            vec![1.0; config.observation_dim],
        ));
        
        let action_space = Box::new(DiscreteSpace::new(config.goal_templates.len()));
        
        Self {
            config,
            current_goal: Arc::new(RwLock::new(None)),
            current_step: Arc::new(RwLock::new(0)),
            goal_history: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
            observation_space,
            action_space,
        }
    }
    
    /// Convert state to observation
    async fn get_observation(&self) -> Array1<f32> {
        let mut obs = Array1::zeros(self.config.observation_dim);
        
        // Encode current goal
        if let Some(goal) = &*self.current_goal.read().await {
            let goal_hash = goal.chars().map(|c| c as u32).sum::<u32>() as f32;
            obs[0] = (goal_hash % 1000.0) / 1000.0;
        }
        
        // Step counter
        let step = *self.current_step.read().await;
        obs[1] = (step as f32 / self.config.max_steps as f32).tanh();
        
        // Recent goal history features
        let history = self.goal_history.read().await;
        let recent_success_rate = if history.len() > 0 {
            let successes = history.iter().filter(|g| g.success).count();
            successes as f32 / history.len() as f32
        } else {
            0.5
        };
        obs[2] = recent_success_rate;
        
        // Average execution time
        if history.len() > 0 {
            let avg_time: f32 = history.iter()
                .map(|g| g.execution_time.as_secs_f32())
                .sum::<f32>() / history.len() as f32;
            obs[3] = avg_time.tanh();
        }
        
        // Fill remaining with system metrics (simulated)
        for i in 4..self.config.observation_dim {
            obs[i] = ((i as f32) * 0.1).sin() * 0.5;
        }
        
        obs
    }
    
    /// Execute goal and return result
    async fn execute_goal(&self, goal: &str) -> Result<GoalExecution> {
        let start = std::time::Instant::now();
        
        // Map goal to command
        let command = self.goal_to_command(goal);
        
        let (success, output) = if self.config.execute_real_commands {
            // Execute real command
            match tokio::process::Command::new("sh")
                .arg("-c")
                .arg(&command)
                .output()
                .await
            {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let full_output = format!("{}\n{}", stdout, stderr);
                    (output.status.success(), full_output)
                }
                Err(e) => (false, format!("Command failed: {}", e)),
            }
        } else {
            // Simulate execution
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            let success = rand::random::<f32>() > 0.3;
            let output = if success {
                format!("Simulated success for: {}", command)
            } else {
                format!("Simulated failure for: {}", command)
            };
            (success, output)
        };
        
        Ok(GoalExecution {
            goal: goal.to_string(),
            command,
            success,
            output,
            execution_time: start.elapsed(),
        })
    }
    
    /// Convert goal to executable command
    fn goal_to_command(&self, goal: &str) -> String {
        let goal_lower = goal.to_lowercase();
        
        if goal_lower.contains("disk") && goal_lower.contains("i/o") {
            "df -h | head -5".to_string()
        } else if goal_lower.contains("memory") {
            "free -h".to_string()
        } else if goal_lower.contains("cpu") {
            "top -bn1 | head -10".to_string()
        } else if goal_lower.contains("network") {
            "netstat -an | head -10".to_string()
        } else if goal_lower.contains("logs") {
            "journalctl -n 10".to_string()
        } else {
            "echo 'Unknown goal'".to_string()
        }
    }
    
    /// Compute reward from execution
    fn compute_reward(&self, execution: &GoalExecution) -> f32 {
        let mut reward = -0.01; // Step penalty
        
        if execution.success {
            reward += 1.0;
            
            // Efficiency bonus
            if execution.execution_time.as_secs_f32() < 0.5 {
                reward += 0.2;
            }
        } else {
            reward -= 0.5;
        }
        
        reward
    }
}

#[async_trait]
impl Environment for GoalTaskEnv {
    fn observation_space(&self) -> &dyn Space {
        self.observation_space.as_ref()
    }
    
    fn action_space(&self) -> &dyn Space {
        self.action_space.as_ref()
    }
    
    async fn reset(&mut self) -> Result<Observation> {
        *self.current_step.write().await = 0;
        *self.current_goal.write().await = None;
        
        let obs = self.get_observation().await;
        Ok(Observation::new(obs.to_vec()))
    }
    
    async fn step(&mut self, action: Action) -> Result<StepInfo> {
        let action_idx = action.as_slice()[0] as usize;
        
        if action_idx >= self.config.goal_templates.len() {
            return Err(anyhow::anyhow!("Invalid action"));
        }
        
        // Select goal based on action
        let goal = self.config.goal_templates[action_idx].clone();
        *self.current_goal.write().await = Some(goal.clone());
        
        // Execute goal
        let execution = self.execute_goal(&goal).await?;
        let reward = self.compute_reward(&execution);
        
        // Update history
        let mut history = self.goal_history.write().await;
        if history.len() >= 100 {
            history.pop_front();
        }
        history.push_back(execution);
        
        // Update step counter
        let mut step = self.current_step.write().await;
        *step += 1;
        
        // Check if done
        let done = *step >= self.config.max_steps;
        
        // Get next observation
        let obs = self.get_observation().await;
        
        Ok(StepInfo {
            observation: Observation::new(obs.to_vec()),
            reward: Reward(reward),
            done,
            truncated: done,
            info: Default::default(),
        })
    }
    
    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}