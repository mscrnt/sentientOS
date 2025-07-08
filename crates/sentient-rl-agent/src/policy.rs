//! Neural network policies for RL agents
//! 
//! This module provides policy network implementations for various RL algorithms.
//! Supports multiple backends (PyTorch via tch, Candle, or pure ndarray).

use anyhow::{Result, Context};
use async_trait::async_trait;
use ndarray::{Array1, Array2, ArrayView1};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Policy network trait for RL agents
#[async_trait]
pub trait PolicyNetwork: Send + Sync {
    /// Forward pass through the network
    async fn forward(&self, observation: &ArrayView1<f32>) -> Result<PolicyOutput>;
    
    /// Sample action from the policy
    async fn sample_action(&self, observation: &ArrayView1<f32>) -> Result<(Array1<f32>, f32)>;
    
    /// Update network parameters
    async fn update(&mut self, gradients: &[f32]) -> Result<()>;
    
    /// Get current parameters
    async fn get_parameters(&self) -> Result<Vec<f32>>;
    
    /// Set parameters
    async fn set_parameters(&mut self, params: &[f32]) -> Result<()>;
    
    /// Clone the network
    fn clone_network(&self) -> Box<dyn PolicyNetwork>;
}

/// Output from policy network
#[derive(Debug, Clone)]
pub struct PolicyOutput {
    /// Action mean or logits
    pub action_output: Array1<f32>,
    /// Value estimate (for actor-critic)
    pub value: Option<f32>,
    /// Action log std (for continuous actions)
    pub log_std: Option<Array1<f32>>,
}

/// MLP (Multi-Layer Perceptron) policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLPConfig {
    /// Input dimension
    pub input_dim: usize,
    /// Hidden layer sizes
    pub hidden_dims: Vec<usize>,
    /// Output dimension (action space)
    pub output_dim: usize,
    /// Activation function
    pub activation: String,
    /// Whether to include value head (for actor-critic)
    pub use_value_head: bool,
    /// Initial log std for continuous actions
    pub init_log_std: f32,
}

impl Default for MLPConfig {
    fn default() -> Self {
        Self {
            input_dim: 4,
            hidden_dims: vec![64, 64],
            output_dim: 2,
            activation: "tanh".to_string(),
            use_value_head: true,
            init_log_std: -0.5,
        }
    }
}

/// Pure ndarray MLP implementation (no external dependencies)
pub struct MLPPolicy {
    config: MLPConfig,
    /// Weights for each layer
    weights: Vec<Array2<f32>>,
    /// Biases for each layer
    biases: Vec<Array1<f32>>,
    /// Value head weights (if enabled)
    value_weights: Option<Array2<f32>>,
    value_bias: Option<Array1<f32>>,
    /// Log std parameters (for continuous actions)
    log_std: Option<Array1<f32>>,
}

impl MLPPolicy {
    /// Create new MLP policy
    pub fn new(config: MLPConfig) -> Self {
        let mut weights = Vec::new();
        let mut biases = Vec::new();
        
        // Initialize layers
        let mut prev_dim = config.input_dim;
        for &hidden_dim in &config.hidden_dims {
            weights.push(Self::xavier_init(prev_dim, hidden_dim));
            biases.push(Array1::zeros(hidden_dim));
            prev_dim = hidden_dim;
        }
        
        // Output layer
        weights.push(Self::xavier_init(prev_dim, config.output_dim));
        biases.push(Array1::zeros(config.output_dim));
        
        // Value head (if enabled)
        let (value_weights, value_bias) = if config.use_value_head {
            let last_hidden = config.hidden_dims.last().copied().unwrap_or(config.input_dim);
            (
                Some(Self::xavier_init(last_hidden, 1)),
                Some(Array1::zeros(1)),
            )
        } else {
            (None, None)
        };
        
        // Log std for continuous actions
        let log_std = Some(Array1::from_elem(config.output_dim, config.init_log_std));
        
        Self {
            config,
            weights,
            biases,
            value_weights,
            value_bias,
            log_std,
        }
    }
    
    /// Xavier initialization for weights
    fn xavier_init(in_dim: usize, out_dim: usize) -> Array2<f32> {
        let limit = (6.0 / (in_dim + out_dim) as f32).sqrt();
        let mut rng = rand::thread_rng();
        Array2::from_shape_fn((in_dim, out_dim), |_| {
            rng.gen_range(-limit..limit)
        })
    }
    
    /// Apply activation function
    fn activation(&self, x: &Array1<f32>) -> Array1<f32> {
        match self.config.activation.as_str() {
            "relu" => x.mapv(|v| v.max(0.0)),
            "tanh" => x.mapv(|v| v.tanh()),
            "sigmoid" => x.mapv(|v| 1.0 / (1.0 + (-v).exp())),
            _ => x.clone(),
        }
    }
    
    /// Forward pass through the network
    fn forward_impl(&self, input: &ArrayView1<f32>) -> PolicyOutput {
        let mut hidden = input.to_owned();
        
        // Pass through hidden layers
        for i in 0..self.config.hidden_dims.len() {
            hidden = hidden.dot(&self.weights[i]) + &self.biases[i];
            hidden = self.activation(&hidden);
        }
        
        // Output layer (no activation for policy logits)
        let action_output = hidden.dot(&self.weights.last().unwrap()) 
            + self.biases.last().unwrap();
        
        // Value head (if enabled)
        let value = if let (Some(w), Some(b)) = (&self.value_weights, &self.value_bias) {
            let value_out = hidden.dot(w) + b;
            Some(value_out[0])
        } else {
            None
        };
        
        PolicyOutput {
            action_output,
            value,
            log_std: self.log_std.clone(),
        }
    }
}

#[async_trait]
impl PolicyNetwork for MLPPolicy {
    async fn forward(&self, observation: &ArrayView1<f32>) -> Result<PolicyOutput> {
        Ok(self.forward_impl(observation))
    }
    
    async fn sample_action(&self, observation: &ArrayView1<f32>) -> Result<(Array1<f32>, f32)> {
        let output = self.forward_impl(observation);
        
        // For discrete actions, sample from categorical distribution
        if self.log_std.is_none() {
            // Softmax over action logits
            let max_logit = output.action_output.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
            let exp_logits = output.action_output.mapv(|x| (x - max_logit).exp());
            let sum_exp = exp_logits.sum();
            let probs = exp_logits / sum_exp;
            
            // Sample action
            let mut rng = rand::thread_rng();
            let sample = rng.gen::<f32>();
            let mut cumsum = 0.0;
            let mut action_idx = 0;
            
            for (i, &p) in probs.iter().enumerate() {
                cumsum += p;
                if sample < cumsum {
                    action_idx = i;
                    break;
                }
            }
            
            // One-hot encode action
            let mut action = Array1::zeros(self.config.output_dim);
            action[action_idx] = 1.0;
            
            // Log probability
            let log_prob = probs[action_idx].ln();
            
            Ok((action, log_prob))
        } else {
            // For continuous actions, sample from Gaussian
            let mean = &output.action_output;
            let log_std = self.log_std.as_ref().unwrap();
            let std = log_std.mapv(|x| x.exp());
            
            // Sample from N(mean, std)
            let mut rng = rand::thread_rng();
            use rand_distr::{Normal, Distribution};
            
            let mut action = Array1::zeros(self.config.output_dim);
            let mut log_prob = 0.0;
            
            for i in 0..self.config.output_dim {
                let dist = Normal::new(mean[i], std[i])?;
                let sample = dist.sample(&mut rng);
                action[i] = sample;
                
                // Log probability of the sample
                let diff = sample - mean[i];
                log_prob += -0.5 * (2.0 * std::f32::consts::PI).ln() 
                    - log_std[i] 
                    - 0.5 * (diff * diff) / (std[i] * std[i]);
            }
            
            // Apply tanh squashing if needed
            let squashed_action = action.mapv(|x| x.tanh());
            
            // Adjust log_prob for tanh squashing
            let log_prob_adjustment = action.mapv(|x| {
                let tanh_x = x.tanh();
                (1.0 - tanh_x * tanh_x + 1e-6).ln()
            }).sum();
            
            Ok((squashed_action, log_prob - log_prob_adjustment))
        }
    }
    
    async fn update(&mut self, gradients: &[f32]) -> Result<()> {
        // Simple gradient update (would be replaced by proper optimizer in production)
        let learning_rate = 3e-4;
        let mut grad_idx = 0;
        
        // Update weights and biases
        for i in 0..self.weights.len() {
            let weight_size = self.weights[i].len();
            let bias_size = self.biases[i].len();
            
            // Update weights
            for j in 0..weight_size {
                if grad_idx < gradients.len() {
                    self.weights[i].as_slice_mut().unwrap()[j] -= learning_rate * gradients[grad_idx];
                    grad_idx += 1;
                }
            }
            
            // Update biases
            for j in 0..bias_size {
                if grad_idx < gradients.len() {
                    self.biases[i][j] -= learning_rate * gradients[grad_idx];
                    grad_idx += 1;
                }
            }
        }
        
        Ok(())
    }
    
    async fn get_parameters(&self) -> Result<Vec<f32>> {
        let mut params = Vec::new();
        
        // Collect all parameters
        for i in 0..self.weights.len() {
            params.extend_from_slice(self.weights[i].as_slice().unwrap());
            params.extend_from_slice(self.biases[i].as_slice().unwrap());
        }
        
        // Value head parameters
        if let (Some(w), Some(b)) = (&self.value_weights, &self.value_bias) {
            params.extend_from_slice(w.as_slice().unwrap());
            params.extend_from_slice(b.as_slice().unwrap());
        }
        
        // Log std parameters
        if let Some(log_std) = &self.log_std {
            params.extend_from_slice(log_std.as_slice().unwrap());
        }
        
        Ok(params)
    }
    
    async fn set_parameters(&mut self, params: &[f32]) -> Result<()> {
        let mut param_idx = 0;
        
        // Set weights and biases
        for i in 0..self.weights.len() {
            let weight_size = self.weights[i].len();
            let bias_size = self.biases[i].len();
            
            // Set weights
            if param_idx + weight_size <= params.len() {
                self.weights[i].as_slice_mut().unwrap()
                    .copy_from_slice(&params[param_idx..param_idx + weight_size]);
                param_idx += weight_size;
            }
            
            // Set biases
            if param_idx + bias_size <= params.len() {
                self.biases[i].as_slice_mut().unwrap()
                    .copy_from_slice(&params[param_idx..param_idx + bias_size]);
                param_idx += bias_size;
            }
        }
        
        // Set value head parameters
        if let (Some(w), Some(b)) = (&mut self.value_weights, &mut self.value_bias) {
            let weight_size = w.len();
            let bias_size = b.len();
            
            if param_idx + weight_size <= params.len() {
                w.as_slice_mut().unwrap()
                    .copy_from_slice(&params[param_idx..param_idx + weight_size]);
                param_idx += weight_size;
            }
            
            if param_idx + bias_size <= params.len() {
                b.as_slice_mut().unwrap()
                    .copy_from_slice(&params[param_idx..param_idx + bias_size]);
                param_idx += bias_size;
            }
        }
        
        // Set log std parameters
        if let Some(log_std) = &mut self.log_std {
            let size = log_std.len();
            if param_idx + size <= params.len() {
                log_std.as_slice_mut().unwrap()
                    .copy_from_slice(&params[param_idx..param_idx + size]);
            }
        }
        
        Ok(())
    }
    
    fn clone_network(&self) -> Box<dyn PolicyNetwork> {
        let mut cloned = Self::new(self.config.clone());
        
        // Deep copy weights and biases
        for i in 0..self.weights.len() {
            cloned.weights[i] = self.weights[i].clone();
            cloned.biases[i] = self.biases[i].clone();
        }
        
        // Copy value head
        if let (Some(w), Some(b)) = (&self.value_weights, &self.value_bias) {
            cloned.value_weights = Some(w.clone());
            cloned.value_bias = Some(b.clone());
        }
        
        // Copy log std
        if let Some(log_std) = &self.log_std {
            cloned.log_std = Some(log_std.clone());
        }
        
        Box::new(cloned)
    }
}

/// Create a policy network based on configuration
pub fn create_policy_network(config: &MLPConfig) -> Box<dyn PolicyNetwork> {
    // For now, only support pure ndarray implementation
    // In future, could check for features and use tch or candle
    Box::new(MLPPolicy::new(config.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::arr1;
    
    #[tokio::test]
    async fn test_mlp_forward() {
        let config = MLPConfig {
            input_dim: 4,
            hidden_dims: vec![32, 32],
            output_dim: 2,
            activation: "tanh".to_string(),
            use_value_head: true,
            init_log_std: -0.5,
        };
        
        let policy = MLPPolicy::new(config);
        let obs = arr1(&[0.1, 0.2, 0.3, 0.4]);
        let output = policy.forward(&obs.view()).await.unwrap();
        
        assert_eq!(output.action_output.len(), 2);
        assert!(output.value.is_some());
    }
    
    #[tokio::test]
    async fn test_action_sampling() {
        let config = MLPConfig {
            input_dim: 4,
            hidden_dims: vec![32],
            output_dim: 2,
            activation: "relu".to_string(),
            use_value_head: false,
            init_log_std: -0.5,
        };
        
        let policy = MLPPolicy::new(config);
        let obs = arr1(&[0.1, 0.2, 0.3, 0.4]);
        
        let (action, log_prob) = policy.sample_action(&obs.view()).await.unwrap();
        assert_eq!(action.len(), 2);
        assert!(log_prob.is_finite());
    }
}