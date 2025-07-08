//! Action representations and action spaces

use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Trait for actions in an RL environment
pub trait Action: Clone + Debug + Send + Sync {
    /// Convert action to a vector representation
    fn to_vec(&self) -> Vec<f64>;
}

/// Trait for defining action spaces
pub trait ActionSpace: Send + Sync {
    /// The type of actions in this space
    type Action: Action;
    
    /// Sample a random action from the space
    fn sample(&self) -> Self::Action;
    
    /// Check if an action is valid within this space
    fn contains(&self, action: &Self::Action) -> bool;
    
    /// Get the dimensionality of the action space
    fn dim(&self) -> Option<usize>;
}

/// Discrete action (e.g., for discrete action spaces)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiscreteAction(pub usize);

impl Action for DiscreteAction {
    fn to_vec(&self) -> Vec<f64> {
        vec![self.0 as f64]
    }
}

/// Continuous action (e.g., for continuous control)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContinuousAction(pub Vec<f64>);

impl Action for ContinuousAction {
    fn to_vec(&self) -> Vec<f64> {
        self.0.clone()
    }
}

/// Discrete action space
#[derive(Debug, Clone)]
pub struct DiscreteSpace {
    /// Number of discrete actions
    pub n: usize,
}

impl DiscreteSpace {
    /// Create a new discrete action space
    #[must_use]
    pub fn new(n: usize) -> Self {
        Self { n }
    }
}

impl ActionSpace for DiscreteSpace {
    type Action = DiscreteAction;
    
    fn sample(&self) -> Self::Action {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        DiscreteAction(rng.gen_range(0..self.n))
    }
    
    fn contains(&self, action: &Self::Action) -> bool {
        action.0 < self.n
    }
    
    fn dim(&self) -> Option<usize> {
        Some(1)
    }
}

/// Continuous action space (box)
#[derive(Debug, Clone)]
pub struct ContinuousSpace {
    /// Lower bounds for each dimension
    pub low: Vec<f64>,
    /// Upper bounds for each dimension
    pub high: Vec<f64>,
}

impl ContinuousSpace {
    /// Create a new continuous action space
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

impl ActionSpace for ContinuousSpace {
    type Action = ContinuousAction;
    
    fn sample(&self) -> Self::Action {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        let values: Vec<f64> = self.low.iter()
            .zip(&self.high)
            .map(|(l, h)| rng.gen_range(*l..*h))
            .collect();
            
        ContinuousAction(values)
    }
    
    fn contains(&self, action: &Self::Action) -> bool {
        action.0.len() == self.low.len() &&
        action.0.iter()
            .zip(&self.low)
            .zip(&self.high)
            .all(|((x, l), h)| x >= l && x <= h)
    }
    
    fn dim(&self) -> Option<usize> {
        Some(self.low.len())
    }
}