//! Reinforcement learning environments for SentientOS
//!
//! This crate provides various RL environments including:
//! - Classic control environments
//! - LLM interaction environments
//! - Multi-agent environments
//! - And more...

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod classic;
pub mod llm;
pub mod registry;
pub mod sentient_envs;
pub mod wrappers;

// Re-export environments
pub use classic::{CartPoleEnv, MountainCarEnv};
pub use llm::{LLMEnv, LLMEnvConfig};
pub use sentient_envs::{JSONLEnv, JSONLEnvConfig, GoalTaskEnv, GoalTaskEnvConfig};
pub use registry::{EnvRegistry, register_env, make_env};
pub use wrappers::{
    RewardWrapper, ObservationWrapper, ActionWrapper,
    TimeLimit, FrameStack, Normalize,
};

// Re-export core types
pub use sentient_rl_core::{
    Environment, EnvironmentConfig, Step, Episode,
    Observation, ObservationSpace, Action, ActionSpace,
    State, StateSpace, Reward,
};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        CartPoleEnv, LLMEnv, EnvRegistry, make_env,
        TimeLimit, FrameStack,
    };
    pub use sentient_rl_core::prelude::*;
}