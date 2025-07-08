//! Reinforcement learning agents implementation for SentientOS
//!
//! This crate provides various RL agent implementations including:
//! - Deep Q-Networks (DQN)
//! - Proximal Policy Optimization (PPO)
//! - Soft Actor-Critic (SAC)
//! - And more...

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod buffer;
pub mod dqn;
pub mod policy;
pub mod ppo;
pub mod ppo_full;
pub mod random;
pub mod utils;

// Re-export agents
pub use dqn::{DQNAgent, DQNConfig};
pub use ppo::{PPOAgent, PPOConfig};
pub use random::RandomAgent;

// Re-export utilities
pub use buffer::{ReplayBuffer, PrioritizedReplayBuffer, Experience};
pub use utils::{LinearSchedule, ExponentialSchedule, Schedule};

// Re-export policy components
pub use policy::{PolicyNetwork, MLPPolicy, MLPConfig, create_policy_network};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        DQNAgent, DQNConfig, PPOAgent, PPOConfig, RandomAgent,
        ReplayBuffer, Experience,
    };
    pub use sentient_rl_core::prelude::*;
}