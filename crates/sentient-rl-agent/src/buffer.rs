//! Experience replay buffers for RL agents

use rand::seq::SliceRandom;
use std::collections::VecDeque;

use sentient_rl_core::{Observation, Action, State, Transition};

/// Experience type alias
pub type Experience<O, A, S> = sentient_rl_core::Experience<O, A, S>;

/// Basic replay buffer for experience replay
#[derive(Debug, Clone)]
pub struct ReplayBuffer<O, A, S> {
    /// Buffer storage
    buffer: VecDeque<Experience<O, A, S>>,
    /// Maximum capacity
    capacity: usize,
}

impl<O, A, S> ReplayBuffer<O, A, S>
where
    O: Observation + Clone,
    A: Action + Clone,
    S: State + Clone,
{
    /// Create a new replay buffer
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        }
    }
    
    /// Add an experience to the buffer
    pub fn push(&mut self, experience: Experience<O, A, S>) {
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(experience);
    }
    
    /// Add a transition to the buffer
    pub fn push_transition(&mut self, transition: Transition<O, A, S>) {
        self.push(Experience::from(transition));
    }
    
    /// Sample a batch of experiences
    pub fn sample(&self, batch_size: usize) -> Option<Vec<Experience<O, A, S>>> {
        if self.buffer.len() < batch_size {
            return None;
        }
        
        let mut rng = rand::thread_rng();
        let indices: Vec<usize> = (0..self.buffer.len()).collect();
        let sample_indices = indices.choose_multiple(&mut rng, batch_size);
        
        let batch: Vec<_> = sample_indices
            .map(|&i| self.buffer[i].clone())
            .collect();
            
        Some(batch)
    }
    
    /// Get the current size of the buffer
    #[must_use]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }
    
    /// Check if buffer is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
    
    /// Clear the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

/// Prioritized experience replay buffer
#[derive(Debug, Clone)]
pub struct PrioritizedReplayBuffer<O, A, S> {
    /// Base buffer
    buffer: Vec<Experience<O, A, S>>,
    /// Priorities
    priorities: Vec<f64>,
    /// Maximum capacity
    capacity: usize,
    /// Priority exponent (alpha)
    alpha: f64,
    /// Importance sampling exponent (beta)
    beta: f64,
    /// Small constant to ensure non-zero probabilities
    epsilon: f64,
    /// Current position for circular buffer
    position: usize,
    /// Current size
    size: usize,
}

impl<O, A, S> PrioritizedReplayBuffer<O, A, S>
where
    O: Observation + Clone,
    A: Action + Clone,
    S: State + Clone,
{
    /// Create a new prioritized replay buffer
    pub fn new(capacity: usize, alpha: f64, beta: f64) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            priorities: vec![1.0; capacity],
            capacity,
            alpha,
            beta,
            epsilon: 1e-6,
            position: 0,
            size: 0,
        }
    }
    
    /// Add an experience with priority
    pub fn push(&mut self, experience: Experience<O, A, S>, priority: f64) {
        let priority = (priority + self.epsilon).powf(self.alpha);
        
        if self.size < self.capacity {
            self.buffer.push(experience);
            self.priorities[self.size] = priority;
            self.size += 1;
        } else {
            self.buffer[self.position] = experience;
            self.priorities[self.position] = priority;
        }
        
        self.position = (self.position + 1) % self.capacity;
    }
    
    /// Sample a batch with importance weights
    pub fn sample(&self, batch_size: usize) -> Option<(Vec<Experience<O, A, S>>, Vec<f64>, Vec<usize>)> {
        if self.size < batch_size {
            return None;
        }
        
        // Compute sampling probabilities
        let sum_priorities: f64 = self.priorities[..self.size].iter().sum();
        let probs: Vec<f64> = self.priorities[..self.size]
            .iter()
            .map(|p| p / sum_priorities)
            .collect();
        
        // Sample indices based on priorities
        let mut rng = rand::thread_rng();
        let mut indices = Vec::with_capacity(batch_size);
        let mut experiences = Vec::with_capacity(batch_size);
        let mut weights = Vec::with_capacity(batch_size);
        
        // Use weighted sampling
        use rand_distr::{Distribution, WeightedIndex};
        let dist = WeightedIndex::new(&probs).unwrap();
        
        let min_prob = probs.iter().copied().fold(f64::INFINITY, f64::min);
        let max_weight = (self.size as f64 * min_prob).powf(-self.beta);
        
        for _ in 0..batch_size {
            let idx = dist.sample(&mut rng);
            indices.push(idx);
            experiences.push(self.buffer[idx].clone());
            
            // Compute importance sampling weight
            let weight = (self.size as f64 * probs[idx]).powf(-self.beta) / max_weight;
            weights.push(weight);
        }
        
        Some((experiences, weights, indices))
    }
    
    /// Update priorities for sampled experiences
    pub fn update_priorities(&mut self, indices: &[usize], priorities: &[f64]) {
        for (&idx, &priority) in indices.iter().zip(priorities) {
            if idx < self.size {
                self.priorities[idx] = (priority + self.epsilon).powf(self.alpha);
            }
        }
    }
    
    /// Set beta (importance sampling exponent)
    pub fn set_beta(&mut self, beta: f64) {
        self.beta = beta.clamp(0.0, 1.0);
    }
    
    /// Get current buffer size
    #[must_use]
    pub fn len(&self) -> usize {
        self.size
    }
    
    /// Check if buffer is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
}