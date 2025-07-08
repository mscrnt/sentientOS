//! Observation representations and observation spaces

use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Trait for observations from an environment
pub trait Observation: Clone + Debug + Send + Sync {
    /// Convert observation to a feature vector
    fn to_vec(&self) -> Vec<f64>;
    
    /// Get the shape of the observation
    fn shape(&self) -> Vec<usize>;
}

/// Trait for defining observation spaces
pub trait ObservationSpace: Send + Sync {
    /// The type of observations in this space
    type Observation: Observation;
    
    /// Sample a random observation from the space
    fn sample(&self) -> Self::Observation;
    
    /// Check if an observation is valid within this space
    fn contains(&self, obs: &Self::Observation) -> bool;
    
    /// Get the shape of observations in this space
    fn shape(&self) -> Vec<usize>;
}

/// Vector observation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorObservation {
    /// The observation data
    pub data: Vec<f64>,
}

impl Observation for VectorObservation {
    fn to_vec(&self) -> Vec<f64> {
        self.data.clone()
    }
    
    fn shape(&self) -> Vec<usize> {
        vec![self.data.len()]
    }
}

/// Image observation (for visual environments)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageObservation {
    /// Image data (flattened)
    pub data: Vec<f64>,
    /// Height of the image
    pub height: usize,
    /// Width of the image
    pub width: usize,
    /// Number of channels
    pub channels: usize,
}

impl Observation for ImageObservation {
    fn to_vec(&self) -> Vec<f64> {
        self.data.clone()
    }
    
    fn shape(&self) -> Vec<usize> {
        vec![self.height, self.width, self.channels]
    }
}

/// Box observation space
#[derive(Debug, Clone)]
pub struct BoxObservationSpace {
    /// Lower bounds
    pub low: Vec<f64>,
    /// Upper bounds
    pub high: Vec<f64>,
    /// Shape of observations
    pub shape: Vec<usize>,
}

impl BoxObservationSpace {
    /// Create a new box observation space
    pub fn new(low: Vec<f64>, high: Vec<f64>, shape: Vec<usize>) -> crate::Result<Self> {
        let total_size: usize = shape.iter().product();
        if low.len() != total_size || high.len() != total_size {
            return Err(crate::RLError::DimensionMismatch {
                expected: total_size,
                actual: low.len(),
            });
        }
        Ok(Self { low, high, shape })
    }
}

impl ObservationSpace for BoxObservationSpace {
    type Observation = VectorObservation;
    
    fn sample(&self) -> Self::Observation {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        let data: Vec<f64> = self.low.iter()
            .zip(&self.high)
            .map(|(l, h)| rng.gen_range(*l..*h))
            .collect();
            
        VectorObservation { data }
    }
    
    fn contains(&self, obs: &Self::Observation) -> bool {
        obs.data.len() == self.low.len() &&
        obs.data.iter()
            .zip(&self.low)
            .zip(&self.high)
            .all(|((x, l), h)| x >= l && x <= h)
    }
    
    fn shape(&self) -> Vec<usize> {
        self.shape.clone()
    }
}