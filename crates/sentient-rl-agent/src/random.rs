//! Random agent for baseline comparisons

use async_trait::async_trait;
use sentient_rl_core::{
    Agent, AgentConfig, Policy, ActionSpace, Observation, Action, Step, State,
};

/// Random agent that selects actions uniformly at random
pub struct RandomAgent<A> {
    /// Action space
    action_space: A,
    /// Configuration
    config: AgentConfig,
    /// Random policy
    policy: RandomPolicy<A>,
}

/// Random policy wrapper
struct RandomPolicy<A> {
    action_space: A,
}

#[async_trait]
impl<O, A> Policy for RandomPolicy<A>
where
    O: Observation,
    A: ActionSpace + Clone + Send + Sync,
    A::Action: Send,
{
    type Observation = O;
    type Action = A::Action;
    
    async fn act(&self, _observation: &Self::Observation) -> sentient_rl_core::Result<Self::Action> {
        Ok(self.action_space.sample())
    }
}

impl<A> RandomAgent<A>
where
    A: ActionSpace + Clone,
{
    /// Create a new random agent
    pub fn new(action_space: A) -> Self {
        let policy = RandomPolicy {
            action_space: action_space.clone(),
        };
        
        Self {
            action_space,
            config: AgentConfig::default(),
            policy,
        }
    }
}

#[async_trait]
impl<O, A> Agent for RandomAgent<A>
where
    O: Observation,
    A: ActionSpace + Clone + Send + Sync + 'static,
    A::Action: Send,
{
    type Observation = O;
    type Action = A::Action;
    
    fn policy(&self) -> &dyn Policy<Observation = Self::Observation, Action = Self::Action> {
        &self.policy
    }
    
    fn policy_mut(&mut self) -> &mut dyn Policy<Observation = Self::Observation, Action = Self::Action> {
        &mut self.policy
    }
    
    async fn observe(&mut self, _step: &Step<Self::Observation, impl State>) -> sentient_rl_core::Result<()> {
        // Random agent doesn't learn from experience
        Ok(())
    }
    
    async fn save(&self, path: &std::path::Path) -> sentient_rl_core::Result<()> {
        // Save configuration
        let json = serde_json::to_string_pretty(&self.config)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }
    
    async fn load(&mut self, path: &std::path::Path) -> sentient_rl_core::Result<()> {
        // Load configuration
        let json = tokio::fs::read_to_string(path).await?;
        self.config = serde_json::from_str(&json)?;
        Ok(())
    }
}