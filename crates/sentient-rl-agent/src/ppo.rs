//! Proximal Policy Optimization (PPO) agent implementation

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;

use sentient_rl_core::{Agent, Action, Observation, StepInfo, Experience};

/// PPO-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PPOConfig {
    /// Base agent configuration
    #[serde(flatten)]
    pub base: sentient_rl_core::AgentConfig,
    /// Clipping parameter for PPO
    pub clip_param: f64,
    /// Number of epochs for training
    pub ppo_epochs: usize,
    /// Number of minibatches
    pub num_minibatches: usize,
    /// Value loss coefficient
    pub value_loss_coef: f64,
    /// Entropy coefficient
    pub entropy_coef: f64,
    /// Maximum gradient norm
    pub max_grad_norm: f64,
    /// GAE lambda
    pub gae_lambda: f64,
    /// Use Generalized Advantage Estimation
    pub use_gae: bool,
    /// Normalize advantages
    pub normalize_advantages: bool,
}

impl Default for PPOConfig {
    fn default() -> Self {
        Self {
            base: sentient_rl_core::AgentConfig::default(),
            clip_param: 0.2,
            ppo_epochs: 4,
            num_minibatches: 4,
            value_loss_coef: 0.5,
            entropy_coef: 0.01,
            max_grad_norm: 0.5,
            gae_lambda: 0.95,
            use_gae: true,
            normalize_advantages: true,
        }
    }
}

/// PPO Agent - wrapper around full implementation
pub struct PPOAgent {
    inner: crate::ppo_full::PPOAgentFull,
}

impl PPOAgent {
    /// Create a new PPO agent
    pub async fn new(
        config: PPOConfig,
        observation_space: usize,
        action_space: usize,
    ) -> Result<Self> {
        let inner = crate::ppo_full::PPOAgentFull::new(
            config,
            observation_space,
            action_space,
        ).await?;
        Ok(Self { inner })
    }
    
    /// Collect rollout from environment
    pub async fn collect_rollout(
        &self,
        env: &mut dyn sentient_rl_core::Environment,
        n_steps: usize,
    ) -> Result<()> {
        self.inner.collect_rollout(env, n_steps).await
    }
    
    /// Train on collected rollout
    pub async fn train_on_rollout(&self) -> Result<crate::ppo_full::PPOTrainingStats> {
        self.inner.train().await
    }
}

#[async_trait]
impl Agent for PPOAgent {
    async fn act(&self, observation: &Observation) -> Result<Action> {
        self.inner.act(observation).await
    }
    
    async fn observe(&mut self, step_info: &StepInfo) -> Result<()> {
        self.inner.observe(step_info).await
    }
    
    async fn remember(&mut self, experience: Experience) -> Result<()> {
        self.inner.remember(experience).await
    }
    
    async fn train(&mut self) -> Result<()> {
        self.inner.train().await
    }
    
    async fn save(&self, path: &Path) -> Result<()> {
        self.inner.save(path).await
    }
    
    async fn load(&mut self, path: &Path) -> Result<()> {
        self.inner.load(path).await
    }
}