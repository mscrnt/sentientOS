//! State representations and state spaces

use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Trait for states in an RL environment
pub trait State: Clone + Debug + Send + Sync {
    /// Get a feature representation of the state
    fn features(&self) -> Vec<f64>;
    
    /// Check if this is a terminal state
    fn is_terminal(&self) -> bool {
        false
    }
}

/// Trait for defining state spaces
pub trait StateSpace: Send + Sync {
    /// The type of states in this space
    type State: State;
    
    /// Sample a random state from the space
    fn sample(&self) -> Self::State;
    
    /// Check if a state is valid within this space
    fn contains(&self, state: &Self::State) -> bool;
    
    /// Get the dimensionality of the state space
    fn dim(&self) -> Option<usize>;
}

/// Terminal state indicator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Terminal {
    /// Not a terminal state
    No,
    /// Terminal state (episode ends)
    Yes,
    /// Truncated (time limit reached)
    Truncated,
}

impl Terminal {
    /// Check if the state is terminal (either Yes or Truncated)
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        !matches!(self, Self::No)
    }
}

/// A simple vector state implementation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorState {
    /// The state vector
    pub data: Vec<f64>,
    /// Terminal status
    pub terminal: Terminal,
}

impl State for VectorState {
    fn features(&self) -> Vec<f64> {
        self.data.clone()
    }
    
    fn is_terminal(&self) -> bool {
        self.terminal.is_terminal()
    }
}

/// Box state space (continuous bounded space)
#[derive(Debug, Clone)]
pub struct BoxSpace {
    /// Lower bounds for each dimension
    pub low: Vec<f64>,
    /// Upper bounds for each dimension
    pub high: Vec<f64>,
}

impl BoxSpace {
    /// Create a new box space
    pub fn new(low: Vec<f64>, high: Vec<f64>) -> crate::Result<Self> {
        if low.len() != high.len() {
            return Err(crate::RLError::DimensionMismatch {
                expected: low.len(),
                actual: high.len(),
            });
        }
        Ok(Self { low, high })
    }
}

impl StateSpace for BoxSpace {
    type State = VectorState;
    
    fn sample(&self) -> Self::State {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        let data: Vec<f64> = self.low.iter()
            .zip(&self.high)
            .map(|(l, h)| rng.gen_range(*l..*h))
            .collect();
            
        VectorState {
            data,
            terminal: Terminal::No,
        }
    }
    
    fn contains(&self, state: &Self::State) -> bool {
        state.data.len() == self.low.len() &&
        state.data.iter()
            .zip(&self.low)
            .zip(&self.high)
            .all(|((x, l), h)| x >= l && x <= h)
    }
    
    fn dim(&self) -> Option<usize> {
        Some(self.low.len())
    }
}