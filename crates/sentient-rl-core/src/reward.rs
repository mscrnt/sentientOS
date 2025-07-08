//! Reward signals and reward functions

use serde::{Deserialize, Serialize};

/// Reward signal from the environment
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Reward(pub f64);

impl Reward {
    /// Create a new reward
    #[must_use]
    pub fn new(value: f64) -> Self {
        Self(value)
    }
    
    /// Get the reward value
    #[must_use]
    pub fn value(&self) -> f64 {
        self.0
    }
}

impl From<f64> for Reward {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl From<Reward> for f64 {
    fn from(reward: Reward) -> Self {
        reward.0
    }
}

impl std::ops::Add for Reward {
    type Output = Self;
    
    fn add(self, other: Self) -> Self::Output {
        Self(self.0 + other.0)
    }
}

impl std::ops::Mul<f64> for Reward {
    type Output = Self;
    
    fn mul(self, scalar: f64) -> Self::Output {
        Self(self.0 * scalar)
    }
}

/// Trait for reward functions
pub trait RewardFunction: Send + Sync {
    /// State type
    type State;
    /// Action type
    type Action;
    
    /// Compute reward for a state-action-next_state transition
    fn reward(
        &self,
        state: &Self::State,
        action: &Self::Action,
        next_state: &Self::State,
    ) -> Reward;
}

/// Shaped reward function that adds a potential-based shaping term
pub struct ShapedReward<R, F> {
    /// Base reward function
    pub base: R,
    /// Potential function
    pub potential: F,
    /// Shaping coefficient
    pub gamma: f64,
}

impl<R, F, S, A> RewardFunction for ShapedReward<R, F>
where
    R: RewardFunction<State = S, Action = A>,
    F: Fn(&S) -> f64 + Send + Sync,
    S: Send + Sync,
    A: Send + Sync,
{
    type State = S;
    type Action = A;
    
    fn reward(
        &self,
        state: &Self::State,
        action: &Self::Action,
        next_state: &Self::State,
    ) -> Reward {
        let base_reward = self.base.reward(state, action, next_state);
        let shaping = self.gamma * (self.potential)(next_state) - (self.potential)(state);
        Reward(base_reward.0 + shaping)
    }
}