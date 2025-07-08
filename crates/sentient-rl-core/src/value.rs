//! Value functions for RL algorithms

use async_trait::async_trait;

use crate::{Action, Observation, State};

/// State value function V(s)
#[async_trait]
pub trait ValueFunction: Send + Sync {
    /// State type
    type State: State;
    
    /// Estimate the value of a state
    async fn value(&self, state: &Self::State) -> crate::Result<f64>;
    
    /// Batch value estimation
    async fn batch_value(&self, states: &[Self::State]) -> crate::Result<Vec<f64>> {
        let mut values = Vec::with_capacity(states.len());
        for state in states {
            values.push(self.value(state).await?);
        }
        Ok(values)
    }
}

/// Action value function Q(s, a)
#[async_trait]
pub trait ActionValueFunction: Send + Sync {
    /// Observation type
    type Observation: Observation;
    /// Action type
    type Action: Action;
    
    /// Estimate the value of taking an action in a given state
    async fn q_value(
        &self,
        observation: &Self::Observation,
        action: &Self::Action,
    ) -> crate::Result<f64>;
    
    /// Get Q-values for all actions
    async fn all_q_values(&self, observation: &Self::Observation) -> crate::Result<Vec<f64>>;
    
    /// Get the best action and its value
    async fn best_action_value(
        &self,
        observation: &Self::Observation,
    ) -> crate::Result<(Self::Action, f64)>;
}

/// Advantage function A(s, a) = Q(s, a) - V(s)
#[async_trait]
pub trait Advantage: Send + Sync {
    /// Observation type
    type Observation: Observation;
    /// Action type
    type Action: Action;
    
    /// Compute advantage for an action in a state
    async fn advantage(
        &self,
        observation: &Self::Observation,
        action: &Self::Action,
    ) -> crate::Result<f64>;
}

/// Neural network based value function
pub struct NeuralValueFunction<N> {
    /// Neural network
    pub network: N,
    /// Whether to use GPU
    pub use_gpu: bool,
}

/// Neural network based Q-function
pub struct NeuralQFunction<N> {
    /// Neural network
    pub network: N,
    /// Number of actions
    pub num_actions: usize,
    /// Whether to use GPU
    pub use_gpu: bool,
}

/// Tabular value function (for discrete state spaces)
pub struct TabularValueFunction {
    /// Value table
    pub values: std::collections::HashMap<String, f64>,
    /// Default value for unseen states
    pub default_value: f64,
}

impl TabularValueFunction {
    /// Create a new tabular value function
    pub fn new(default_value: f64) -> Self {
        Self {
            values: std::collections::HashMap::new(),
            default_value,
        }
    }
    
    /// Update value for a state
    pub fn update(&mut self, state_key: String, value: f64) {
        self.values.insert(state_key, value);
    }
}

#[async_trait]
impl<S> ValueFunction for TabularValueFunction
where
    S: State + serde::Serialize,
{
    type State = S;
    
    async fn value(&self, state: &Self::State) -> crate::Result<f64> {
        let key = serde_json::to_string(state)?;
        Ok(self.values.get(&key).copied().unwrap_or(self.default_value))
    }
}

/// Tabular Q-function (for discrete state-action spaces)
pub struct TabularQFunction {
    /// Q-value table
    pub q_values: std::collections::HashMap<String, Vec<f64>>,
    /// Number of actions
    pub num_actions: usize,
    /// Default Q-value
    pub default_q_value: f64,
}

impl TabularQFunction {
    /// Create a new tabular Q-function
    pub fn new(num_actions: usize, default_q_value: f64) -> Self {
        Self {
            q_values: std::collections::HashMap::new(),
            num_actions,
            default_q_value,
        }
    }
    
    /// Update Q-value for a state-action pair
    pub fn update(&mut self, state_key: String, action: usize, value: f64) {
        let values = self.q_values.entry(state_key).or_insert_with(|| {
            vec![self.default_q_value; self.num_actions]
        });
        if action < self.num_actions {
            values[action] = value;
        }
    }
}

#[async_trait]
impl<O> ActionValueFunction for TabularQFunction
where
    O: Observation + serde::Serialize,
{
    type Observation = O;
    type Action = crate::DiscreteAction;
    
    async fn q_value(
        &self,
        observation: &Self::Observation,
        action: &Self::Action,
    ) -> crate::Result<f64> {
        let key = serde_json::to_string(observation)?;
        let values = self.q_values.get(&key);
        
        Ok(values
            .and_then(|v| v.get(action.0))
            .copied()
            .unwrap_or(self.default_q_value))
    }
    
    async fn all_q_values(&self, observation: &Self::Observation) -> crate::Result<Vec<f64>> {
        let key = serde_json::to_string(observation)?;
        
        Ok(self.q_values
            .get(&key)
            .cloned()
            .unwrap_or_else(|| vec![self.default_q_value; self.num_actions]))
    }
    
    async fn best_action_value(
        &self,
        observation: &Self::Observation,
    ) -> crate::Result<(Self::Action, f64)> {
        let q_values = self.all_q_values(observation).await?;
        
        let (best_action, best_value) = q_values
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, &v)| (crate::DiscreteAction(i), v))
            .unwrap_or((crate::DiscreteAction(0), self.default_q_value));
            
        Ok((best_action, best_value))
    }
}