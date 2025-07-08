//! Error types for the RL core library

use thiserror::Error;

/// Core error type for RL operations
#[derive(Error, Debug)]
pub enum RLError {
    /// Environment-related errors
    #[error("Environment error: {0}")]
    Environment(String),
    
    /// Agent-related errors
    #[error("Agent error: {0}")]
    Agent(String),
    
    /// Policy-related errors
    #[error("Policy error: {0}")]
    Policy(String),
    
    /// Invalid action
    #[error("Invalid action: {0}")]
    InvalidAction(String),
    
    /// Invalid state
    #[error("Invalid state: {0}")]
    InvalidState(String),
    
    /// Dimension mismatch
    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },
    
    /// Computation error
    #[error("Computation error: {0}")]
    Computation(String),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Other errors
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

/// Result type alias for RL operations
pub type Result<T> = std::result::Result<T, RLError>;