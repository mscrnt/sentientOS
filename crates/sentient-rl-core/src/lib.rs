//! Core reinforcement learning traits and types for SentientOS
//!
//! This crate provides the foundational abstractions for building
//! reinforcement learning systems in a type-safe, modular way.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod action;
pub mod agent;
pub mod environment;
pub mod error;
pub mod observation;
pub mod policy;
pub mod reward;
pub mod state;
pub mod trajectory;
pub mod value;

// Re-export core traits and types
pub use action::{Action, ActionSpace, DiscreteAction, ContinuousAction};
pub use agent::{Agent, AgentConfig, Learning};
pub use environment::{Environment, EnvironmentConfig, Step, Episode};
pub use error::{RLError, Result};
pub use observation::{Observation, ObservationSpace};
pub use policy::{Policy, DeterministicPolicy, StochasticPolicy};
pub use reward::{Reward, RewardFunction};
pub use state::{State, StateSpace, Terminal};
pub use trajectory::{Trajectory, Transition, Experience};
pub use value::{ValueFunction, ActionValueFunction, Advantage};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        Action, ActionSpace, Agent, Environment, Observation, ObservationSpace,
        Policy, Reward, State, StateSpace, Step, Result,
    };
}