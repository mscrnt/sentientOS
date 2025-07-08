//! Trajectory and experience storage

use serde::{Deserialize, Serialize};

use crate::{Action, Observation, Reward, State};

/// Single transition in a trajectory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition<O, A, S> {
    /// Current observation
    pub observation: O,
    /// Action taken
    pub action: A,
    /// Reward received
    pub reward: Reward,
    /// Next observation
    pub next_observation: O,
    /// Whether episode ended
    pub done: bool,
    /// Internal state (if available)
    pub state: Option<S>,
    /// Next internal state (if available)
    pub next_state: Option<S>,
}

/// Experience for learning (similar to transition but may include additional info)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience<O, A, S> {
    /// The transition
    pub transition: Transition<O, A, S>,
    /// Value estimate (if available)
    pub value: Option<f64>,
    /// Action log probability (for policy gradient)
    pub log_prob: Option<f64>,
    /// Advantage estimate (for actor-critic)
    pub advantage: Option<f64>,
    /// TD error
    pub td_error: Option<f64>,
}

impl<O, A, S> From<Transition<O, A, S>> for Experience<O, A, S> {
    fn from(transition: Transition<O, A, S>) -> Self {
        Self {
            transition,
            value: None,
            log_prob: None,
            advantage: None,
            td_error: None,
        }
    }
}

/// Complete trajectory of an episode
#[derive(Debug, Clone)]
pub struct Trajectory<O, A, S> {
    /// Sequence of transitions
    pub transitions: Vec<Transition<O, A, S>>,
    /// Total reward
    pub total_reward: f64,
    /// Episode ID
    pub episode_id: String,
}

impl<O, A, S> Trajectory<O, A, S> {
    /// Create a new empty trajectory
    pub fn new(episode_id: String) -> Self {
        Self {
            transitions: Vec::new(),
            total_reward: 0.0,
            episode_id,
        }
    }
    
    /// Add a transition to the trajectory
    pub fn push(&mut self, transition: Transition<O, A, S>) {
        self.total_reward += transition.reward.0;
        self.transitions.push(transition);
    }
    
    /// Get the length of the trajectory
    #[must_use]
    pub fn len(&self) -> usize {
        self.transitions.len()
    }
    
    /// Check if trajectory is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.transitions.is_empty()
    }
    
    /// Compute returns (cumulative discounted rewards)
    #[must_use]
    pub fn returns(&self, gamma: f64) -> Vec<f64> {
        let mut returns = vec![0.0; self.len()];
        let mut running_return = 0.0;
        
        for i in (0..self.len()).rev() {
            if self.transitions[i].done {
                running_return = 0.0;
            }
            running_return = self.transitions[i].reward.0 + gamma * running_return;
            returns[i] = running_return;
        }
        
        returns
    }
    
    /// Compute advantages using GAE (Generalized Advantage Estimation)
    pub fn gae_advantages(&self, values: &[f64], gamma: f64, lambda: f64) -> Vec<f64> {
        let mut advantages = vec![0.0; self.len()];
        let mut running_advantage = 0.0;
        
        for i in (0..self.len()).rev() {
            let td_error = if i == self.len() - 1 {
                self.transitions[i].reward.0 - values[i]
            } else {
                self.transitions[i].reward.0 + gamma * values[i + 1] - values[i]
            };
            
            if self.transitions[i].done {
                running_advantage = 0.0;
            }
            
            running_advantage = td_error + gamma * lambda * running_advantage;
            advantages[i] = running_advantage;
        }
        
        advantages
    }
}

/// Batch of trajectories
#[derive(Debug, Clone)]
pub struct TrajectoryBatch<O, A, S> {
    /// Collection of trajectories
    pub trajectories: Vec<Trajectory<O, A, S>>,
}

impl<O, A, S> TrajectoryBatch<O, A, S> {
    /// Create a new empty batch
    #[must_use]
    pub fn new() -> Self {
        Self {
            trajectories: Vec::new(),
        }
    }
    
    /// Add a trajectory to the batch
    pub fn push(&mut self, trajectory: Trajectory<O, A, S>) {
        self.trajectories.push(trajectory);
    }
    
    /// Get total number of transitions across all trajectories
    #[must_use]
    pub fn total_transitions(&self) -> usize {
        self.trajectories.iter().map(|t| t.len()).sum()
    }
    
    /// Get average episode reward
    #[must_use]
    pub fn avg_reward(&self) -> f64 {
        if self.trajectories.is_empty() {
            0.0
        } else {
            let total: f64 = self.trajectories.iter().map(|t| t.total_reward).sum();
            total / self.trajectories.len() as f64
        }
    }
}

impl<O, A, S> Default for TrajectoryBatch<O, A, S> {
    fn default() -> Self {
        Self::new()
    }
}