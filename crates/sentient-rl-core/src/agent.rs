//! Agent traits and types

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{Action, Observation, Policy, Step, Environment};

/// Configuration for agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Learning rate
    pub learning_rate: f64,
    /// Discount factor
    pub gamma: f64,
    /// Batch size for training
    pub batch_size: usize,
    /// Buffer size for experience replay
    pub buffer_size: usize,
    /// Target network update frequency
    pub target_update_freq: Option<usize>,
    /// Additional parameters
    #[serde(flatten)]
    pub params: serde_json::Map<String, serde_json::Value>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            learning_rate: 1e-3,
            gamma: 0.99,
            batch_size: 32,
            buffer_size: 10000,
            target_update_freq: Some(100),
            params: serde_json::Map::new(),
        }
    }
}

/// Core agent trait
#[async_trait]
pub trait Agent: Send + Sync {
    /// Observation type
    type Observation: Observation;
    /// Action type
    type Action: Action;
    
    /// Get the agent's policy
    fn policy(&self) -> &dyn Policy<Observation = Self::Observation, Action = Self::Action>;
    
    /// Get mutable reference to the agent's policy
    fn policy_mut(&mut self) -> &mut dyn Policy<Observation = Self::Observation, Action = Self::Action>;
    
    /// Select an action given an observation
    async fn act(&self, observation: &Self::Observation) -> crate::Result<Self::Action> {
        self.policy().act(observation).await
    }
    
    /// Process a step from the environment (for learning)
    async fn observe(&mut self, step: &Step<Self::Observation, impl crate::State>) -> crate::Result<()> {
        Ok(()) // Default: no learning
    }
    
    /// Save the agent
    async fn save(&self, path: &std::path::Path) -> crate::Result<()>;
    
    /// Load the agent
    async fn load(&mut self, path: &std::path::Path) -> crate::Result<()>;
    
    /// Get agent metrics
    fn metrics(&self) -> AgentMetrics {
        AgentMetrics::default()
    }
}

/// Agent metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentMetrics {
    /// Total steps taken
    pub total_steps: usize,
    /// Total episodes
    pub total_episodes: usize,
    /// Average reward per episode
    pub avg_episode_reward: f64,
    /// Loss value
    pub loss: Option<f64>,
    /// Additional metrics
    #[serde(flatten)]
    pub custom: serde_json::Map<String, serde_json::Value>,
}

/// Trait for agents that can learn
#[async_trait]
pub trait Learning: Agent {
    /// Train the agent for one step
    async fn train_step(&mut self) -> crate::Result<f64>;
    
    /// Train the agent for multiple steps
    async fn train(&mut self, steps: usize) -> crate::Result<Vec<f64>> {
        let mut losses = Vec::with_capacity(steps);
        for _ in 0..steps {
            losses.push(self.train_step().await?);
        }
        Ok(losses)
    }
    
    /// Set training mode
    fn set_training(&mut self, training: bool);
    
    /// Check if in training mode
    fn is_training(&self) -> bool;
}

/// Base agent implementation with common functionality
pub struct BaseAgent<P> {
    /// Policy
    pub policy: P,
    /// Configuration
    pub config: AgentConfig,
    /// Metrics
    pub metrics: AgentMetrics,
    /// Training mode
    pub training: bool,
}

impl<P> BaseAgent<P> {
    /// Create a new base agent
    pub fn new(policy: P, config: AgentConfig) -> Self {
        Self {
            policy,
            config,
            metrics: AgentMetrics::default(),
            training: true,
        }
    }
}

#[async_trait]
impl<P> Agent for BaseAgent<P>
where
    P: Policy + Send + Sync + 'static,
{
    type Observation = P::Observation;
    type Action = P::Action;
    
    fn policy(&self) -> &dyn Policy<Observation = Self::Observation, Action = Self::Action> {
        &self.policy
    }
    
    fn policy_mut(&mut self) -> &mut dyn Policy<Observation = Self::Observation, Action = Self::Action> {
        &mut self.policy
    }
    
    async fn save(&self, path: &std::path::Path) -> crate::Result<()> {
        // Default implementation saves config and metrics
        let data = serde_json::json!({
            "config": self.config,
            "metrics": self.metrics,
        });
        
        let json = serde_json::to_string_pretty(&data)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }
    
    async fn load(&mut self, path: &std::path::Path) -> crate::Result<()> {
        let json = tokio::fs::read_to_string(path).await?;
        let data: serde_json::Value = serde_json::from_str(&json)?;
        
        if let Some(config) = data.get("config") {
            self.config = serde_json::from_value(config.clone())?;
        }
        
        if let Some(metrics) = data.get("metrics") {
            self.metrics = serde_json::from_value(metrics.clone())?;
        }
        
        Ok(())
    }
    
    fn metrics(&self) -> AgentMetrics {
        self.metrics.clone()
    }
}