//! Utility functions and helpers for RL agents

use std::sync::Arc;

/// Trait for schedules (e.g., for epsilon decay)
pub trait Schedule: Send + Sync {
    /// Get value at step t
    fn value(&self, t: usize) -> f64;
}

/// Linear schedule that decays from start to end over steps
#[derive(Debug, Clone)]
pub struct LinearSchedule {
    /// Starting value
    pub start: f64,
    /// Ending value
    pub end: f64,
    /// Number of steps for decay
    pub steps: usize,
}

impl LinearSchedule {
    /// Create a new linear schedule
    pub fn new(start: f64, end: f64, steps: usize) -> Self {
        Self { start, end, steps }
    }
}

impl Schedule for LinearSchedule {
    fn value(&self, t: usize) -> f64 {
        if t >= self.steps {
            self.end
        } else {
            let progress = t as f64 / self.steps as f64;
            self.start + (self.end - self.start) * progress
        }
    }
}

/// Exponential decay schedule
#[derive(Debug, Clone)]
pub struct ExponentialSchedule {
    /// Starting value
    pub start: f64,
    /// Minimum value
    pub min_value: f64,
    /// Decay rate
    pub decay_rate: f64,
}

impl ExponentialSchedule {
    /// Create a new exponential schedule
    pub fn new(start: f64, min_value: f64, decay_rate: f64) -> Self {
        Self {
            start,
            min_value,
            decay_rate,
        }
    }
}

impl Schedule for ExponentialSchedule {
    fn value(&self, t: usize) -> f64 {
        let value = self.start * (self.decay_rate.powf(t as f64));
        value.max(self.min_value)
    }
}

/// Constant schedule
#[derive(Debug, Clone)]
pub struct ConstantSchedule {
    /// Constant value
    pub value: f64,
}

impl Schedule for ConstantSchedule {
    fn value(&self, _t: usize) -> f64 {
        self.value
    }
}

/// Polyak averaging for target network updates
pub fn polyak_update(target_weight: f64, source_weight: f64, tau: f64) -> f64 {
    tau * source_weight + (1.0 - tau) * target_weight
}

/// Compute discounted returns
pub fn compute_returns(rewards: &[f64], gamma: f64, terminal_value: f64) -> Vec<f64> {
    let mut returns = vec![0.0; rewards.len()];
    let mut running_return = terminal_value;
    
    for i in (0..rewards.len()).rev() {
        running_return = rewards[i] + gamma * running_return;
        returns[i] = running_return;
    }
    
    returns
}

/// Compute GAE (Generalized Advantage Estimation)
pub fn compute_gae(
    rewards: &[f64],
    values: &[f64],
    next_value: f64,
    gamma: f64,
    lambda: f64,
) -> Vec<f64> {
    let mut advantages = vec![0.0; rewards.len()];
    let mut running_advantage = 0.0;
    
    for i in (0..rewards.len()).rev() {
        let next_v = if i == rewards.len() - 1 {
            next_value
        } else {
            values[i + 1]
        };
        
        let td_error = rewards[i] + gamma * next_v - values[i];
        running_advantage = td_error + gamma * lambda * running_advantage;
        advantages[i] = running_advantage;
    }
    
    advantages
}

/// Running mean and std calculator
#[derive(Debug, Clone)]
pub struct RunningMeanStd {
    /// Mean
    pub mean: f64,
    /// Variance
    pub var: f64,
    /// Count
    pub count: usize,
}

impl RunningMeanStd {
    /// Create new running statistics
    pub fn new() -> Self {
        Self {
            mean: 0.0,
            var: 1.0,
            count: 0,
        }
    }
    
    /// Update with a new value
    pub fn update(&mut self, x: f64) {
        self.count += 1;
        let delta = x - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = x - self.mean;
        self.var += (delta * delta2 - self.var) / self.count as f64;
    }
    
    /// Update with a batch of values
    pub fn update_batch(&mut self, batch: &[f64]) {
        for &x in batch {
            self.update(x);
        }
    }
    
    /// Get standard deviation
    pub fn std(&self) -> f64 {
        self.var.sqrt()
    }
    
    /// Normalize a value
    pub fn normalize(&self, x: f64) -> f64 {
        (x - self.mean) / (self.std() + 1e-8)
    }
}

impl Default for RunningMeanStd {
    fn default() -> Self {
        Self::new()
    }
}

/// Clip value to range
pub fn clip(x: f64, min: f64, max: f64) -> f64 {
    x.clamp(min, max)
}

/// Soft update for neural network parameters
pub async fn soft_update_params<P: AsRef<std::path::Path>>(
    target_path: P,
    source_path: P,
    tau: f64,
) -> sentient_rl_core::Result<()> {
    // This is a placeholder - actual implementation would depend on the neural network backend
    // For now, just copy the file
    if tau >= 1.0 {
        tokio::fs::copy(source_path, target_path).await?;
    }
    Ok(())
}