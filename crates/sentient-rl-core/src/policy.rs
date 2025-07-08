//! Policy abstractions for action selection

use async_trait::async_trait;
use std::fmt::Debug;

use crate::{Action, ActionSpace, Observation};

/// Core policy trait for selecting actions
#[async_trait]
pub trait Policy: Send + Sync {
    /// Observation type
    type Observation: Observation;
    /// Action type  
    type Action: Action;
    
    /// Select an action given an observation
    async fn act(&self, observation: &Self::Observation) -> crate::Result<Self::Action>;
    
    /// Update the policy (for learnable policies)
    async fn update(&mut self) -> crate::Result<()> {
        Ok(())
    }
}

/// Deterministic policy that always returns the same action for a given observation
#[async_trait]
pub trait DeterministicPolicy: Policy {
    /// Get the deterministic action for an observation
    async fn deterministic_act(&self, observation: &Self::Observation) -> crate::Result<Self::Action>;
}

#[async_trait]
impl<P> Policy for P
where
    P: DeterministicPolicy,
{
    type Observation = P::Observation;
    type Action = P::Action;
    
    async fn act(&self, observation: &Self::Observation) -> crate::Result<Self::Action> {
        self.deterministic_act(observation).await
    }
}

/// Stochastic policy that samples actions from a distribution
#[async_trait]
pub trait StochasticPolicy: Policy {
    /// Sample an action from the policy distribution
    async fn sample(&self, observation: &Self::Observation) -> crate::Result<Self::Action>;
    
    /// Get the log probability of an action given an observation
    async fn log_prob(
        &self,
        observation: &Self::Observation,
        action: &Self::Action,
    ) -> crate::Result<f64>;
    
    /// Get the entropy of the policy distribution for an observation
    async fn entropy(&self, observation: &Self::Observation) -> crate::Result<f64> {
        Ok(0.0) // Default implementation
    }
}

#[async_trait]
impl<P> Policy for P
where
    P: StochasticPolicy,
{
    type Observation = P::Observation;
    type Action = P::Action;
    
    async fn act(&self, observation: &Self::Observation) -> crate::Result<Self::Action> {
        self.sample(observation).await
    }
}

/// Epsilon-greedy policy wrapper
pub struct EpsilonGreedy<P, A> {
    /// Base policy
    pub policy: P,
    /// Exploration rate
    pub epsilon: f64,
    /// Action space for random sampling
    pub action_space: A,
}

impl<P, A> EpsilonGreedy<P, A> {
    /// Create a new epsilon-greedy policy
    pub fn new(policy: P, epsilon: f64, action_space: A) -> Self {
        Self {
            policy,
            epsilon,
            action_space,
        }
    }
    
    /// Set the exploration rate
    pub fn set_epsilon(&mut self, epsilon: f64) {
        self.epsilon = epsilon.clamp(0.0, 1.0);
    }
}

#[async_trait]
impl<P, A> Policy for EpsilonGreedy<P, A>
where
    P: Policy,
    A: ActionSpace<Action = P::Action> + Send + Sync,
{
    type Observation = P::Observation;
    type Action = P::Action;
    
    async fn act(&self, observation: &Self::Observation) -> crate::Result<Self::Action> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        if rng.gen::<f64>() < self.epsilon {
            // Explore: random action
            Ok(self.action_space.sample())
        } else {
            // Exploit: use base policy
            self.policy.act(observation).await
        }
    }
    
    async fn update(&mut self) -> crate::Result<()> {
        self.policy.update().await
    }
}

/// Random policy that always selects random actions
pub struct RandomPolicy<A> {
    /// Action space
    pub action_space: A,
}

impl<A> RandomPolicy<A> {
    /// Create a new random policy
    pub fn new(action_space: A) -> Self {
        Self { action_space }
    }
}

#[async_trait]
impl<O, A> Policy for RandomPolicy<A>
where
    O: Observation,
    A: ActionSpace + Send + Sync,
{
    type Observation = O;
    type Action = A::Action;
    
    async fn act(&self, _observation: &Self::Observation) -> crate::Result<Self::Action> {
        Ok(self.action_space.sample())
    }
}