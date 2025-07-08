//! Environment wrappers for common transformations

use async_trait::async_trait;
use std::collections::VecDeque;

use sentient_rl_core::{
    Environment, Step, StepInfo, Observation, ObservationSpace,
    Action, ActionSpace, State, StateSpace, Reward,
};

/// Wrapper that modifies rewards
pub struct RewardWrapper<E, F> {
    /// Inner environment
    pub env: E,
    /// Reward transformation function
    pub reward_fn: F,
}

#[async_trait]
impl<E, F> Environment for RewardWrapper<E, F>
where
    E: Environment,
    F: Fn(Reward, &Step<E::Observation, E::State>) -> Reward + Send + Sync,
{
    type Observation = E::Observation;
    type Action = E::Action;
    type State = E::State;
    
    fn observation_space(&self) -> Box<dyn ObservationSpace<Observation = Self::Observation>> {
        self.env.observation_space()
    }
    
    fn action_space(&self) -> Box<dyn ActionSpace<Action = Self::Action>> {
        self.env.action_space()
    }
    
    fn state_space(&self) -> Option<Box<dyn StateSpace<State = Self::State>>> {
        self.env.state_space()
    }
    
    async fn reset(&mut self) -> sentient_rl_core::Result<(Self::Observation, StepInfo)> {
        self.env.reset().await
    }
    
    async fn step(&mut self, action: Self::Action) -> sentient_rl_core::Result<Step<Self::Observation, Self::State>> {
        let mut step = self.env.step(action).await?;
        step.reward = (self.reward_fn)(step.reward, &step);
        Ok(step)
    }
    
    async fn render(&self) -> sentient_rl_core::Result<()> {
        self.env.render().await
    }
    
    async fn close(&mut self) -> sentient_rl_core::Result<()> {
        self.env.close().await
    }
}

/// Wrapper that transforms observations
pub struct ObservationWrapper<E, F, O2> {
    /// Inner environment
    pub env: E,
    /// Observation transformation function
    pub obs_fn: F,
    /// Phantom data for new observation type
    _phantom: std::marker::PhantomData<O2>,
}

/// Wrapper that transforms actions
pub struct ActionWrapper<E, F, A2> {
    /// Inner environment
    pub env: E,
    /// Action transformation function
    pub action_fn: F,
    /// Phantom data for new action type
    _phantom: std::marker::PhantomData<A2>,
}

/// Time limit wrapper
pub struct TimeLimit<E> {
    /// Inner environment
    pub env: E,
    /// Maximum steps
    pub max_steps: usize,
    /// Current step count
    pub steps: usize,
}

impl<E> TimeLimit<E> {
    /// Create a new time limit wrapper
    pub fn new(env: E, max_steps: usize) -> Self {
        Self {
            env,
            max_steps,
            steps: 0,
        }
    }
}

#[async_trait]
impl<E> Environment for TimeLimit<E>
where
    E: Environment,
{
    type Observation = E::Observation;
    type Action = E::Action;
    type State = E::State;
    
    fn observation_space(&self) -> Box<dyn ObservationSpace<Observation = Self::Observation>> {
        self.env.observation_space()
    }
    
    fn action_space(&self) -> Box<dyn ActionSpace<Action = Self::Action>> {
        self.env.action_space()
    }
    
    fn state_space(&self) -> Option<Box<dyn StateSpace<State = Self::State>>> {
        self.env.state_space()
    }
    
    async fn reset(&mut self) -> sentient_rl_core::Result<(Self::Observation, StepInfo)> {
        self.steps = 0;
        self.env.reset().await
    }
    
    async fn step(&mut self, action: Self::Action) -> sentient_rl_core::Result<Step<Self::Observation, Self::State>> {
        self.steps += 1;
        let mut step = self.env.step(action).await?;
        
        if self.steps >= self.max_steps && !step.done {
            step.truncated = true;
            step.done = true;
        }
        
        Ok(step)
    }
    
    async fn render(&self) -> sentient_rl_core::Result<()> {
        self.env.render().await
    }
    
    async fn close(&mut self) -> sentient_rl_core::Result<()> {
        self.env.close().await
    }
}

/// Frame stacking wrapper for temporal information
pub struct FrameStack<E> {
    /// Inner environment
    pub env: E,
    /// Number of frames to stack
    pub n_frames: usize,
    /// Frame buffer
    pub frames: VecDeque<E::Observation>,
}

impl<E> FrameStack<E>
where
    E::Observation: Clone,
{
    /// Create a new frame stack wrapper
    pub fn new(env: E, n_frames: usize) -> Self {
        Self {
            env,
            n_frames,
            frames: VecDeque::with_capacity(n_frames),
        }
    }
}

/// Observation normalization wrapper
pub struct Normalize<E> {
    /// Inner environment
    pub env: E,
    /// Running mean
    pub mean: Vec<f64>,
    /// Running std
    pub std: Vec<f64>,
    /// Update statistics
    pub update_stats: bool,
    /// Clip range
    pub clip_range: Option<(f64, f64)>,
}

impl<E> Normalize<E> {
    /// Create a new normalization wrapper
    pub fn new(env: E, obs_dim: usize) -> Self {
        Self {
            env,
            mean: vec![0.0; obs_dim],
            std: vec![1.0; obs_dim],
            update_stats: true,
            clip_range: Some((-5.0, 5.0)),
        }
    }
    
    /// Update running statistics
    pub fn update(&mut self, obs: &[f64]) {
        if !self.update_stats || obs.len() != self.mean.len() {
            return;
        }
        
        // Simple online update (simplified version)
        for i in 0..obs.len() {
            let delta = obs[i] - self.mean[i];
            self.mean[i] += delta * 0.01; // Learning rate
            self.std[i] = (self.std[i].powi(2) * 0.99 + delta.powi(2) * 0.01).sqrt();
        }
    }
    
    /// Normalize observation
    pub fn normalize(&self, obs: &[f64]) -> Vec<f64> {
        let mut normalized = Vec::with_capacity(obs.len());
        
        for i in 0..obs.len() {
            let z = (obs[i] - self.mean[i]) / (self.std[i] + 1e-8);
            let z = if let Some((min, max)) = self.clip_range {
                z.clamp(min, max)
            } else {
                z
            };
            normalized.push(z);
        }
        
        normalized
    }
}