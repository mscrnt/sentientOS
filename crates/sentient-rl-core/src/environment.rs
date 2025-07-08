//! Environment traits and types

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{Action, ActionSpace, Observation, ObservationSpace, Reward, State, StateSpace};

/// Result of a single environment step
#[derive(Debug, Clone)]
pub struct Step<O, S> {
    /// Observation from the environment
    pub observation: O,
    /// Reward signal
    pub reward: Reward,
    /// Whether the episode is done
    pub done: bool,
    /// Whether the episode was truncated (e.g., time limit)
    pub truncated: bool,
    /// Additional info from the environment
    pub info: StepInfo,
    /// Internal state (if available)
    pub state: Option<S>,
}

/// Additional information from a step
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StepInfo {
    /// Custom fields
    #[serde(flatten)]
    pub fields: serde_json::Map<String, serde_json::Value>,
}

/// Episode information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    /// Episode ID
    pub id: String,
    /// Total reward
    pub total_reward: f64,
    /// Number of steps
    pub steps: usize,
    /// Whether episode was truncated
    pub truncated: bool,
    /// Start time
    pub start_time: chrono::DateTime<chrono::Utc>,
    /// End time
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// Configuration for environments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    /// Random seed
    pub seed: Option<u64>,
    /// Maximum episode steps
    pub max_steps: Option<usize>,
    /// Render mode
    pub render_mode: Option<String>,
    /// Additional parameters
    #[serde(flatten)]
    pub params: serde_json::Map<String, serde_json::Value>,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            seed: None,
            max_steps: None,
            render_mode: None,
            params: serde_json::Map::new(),
        }
    }
}

/// Core environment trait
#[async_trait]
pub trait Environment: Send + Sync {
    /// Observation type
    type Observation: Observation;
    /// Action type
    type Action: Action;
    /// State type
    type State: State;
    
    /// Get the observation space
    fn observation_space(&self) -> Box<dyn ObservationSpace<Observation = Self::Observation>>;
    
    /// Get the action space
    fn action_space(&self) -> Box<dyn ActionSpace<Action = Self::Action>>;
    
    /// Get the state space (if available)
    fn state_space(&self) -> Option<Box<dyn StateSpace<State = Self::State>>> {
        None
    }
    
    /// Reset the environment
    async fn reset(&mut self) -> crate::Result<(Self::Observation, StepInfo)>;
    
    /// Take a step in the environment
    async fn step(&mut self, action: Self::Action) -> crate::Result<Step<Self::Observation, Self::State>>;
    
    /// Render the environment (optional)
    async fn render(&self) -> crate::Result<()> {
        Ok(())
    }
    
    /// Close the environment
    async fn close(&mut self) -> crate::Result<()> {
        Ok(())
    }
    
    /// Get current episode info
    fn episode_info(&self) -> Option<Episode> {
        None
    }
}

/// Wrapper for environments that tracks episodes
pub struct TrackedEnvironment<E> {
    /// Inner environment
    pub env: E,
    /// Current episode
    pub episode: Option<Episode>,
    /// Step counter
    pub step_count: usize,
}

impl<E> TrackedEnvironment<E> {
    /// Create a new tracked environment
    pub fn new(env: E) -> Self {
        Self {
            env,
            episode: None,
            step_count: 0,
        }
    }
}

#[async_trait]
impl<E> Environment for TrackedEnvironment<E>
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
    
    async fn reset(&mut self) -> crate::Result<(Self::Observation, StepInfo)> {
        // End current episode if exists
        if let Some(ref mut episode) = self.episode {
            episode.end_time = Some(chrono::Utc::now());
        }
        
        // Start new episode
        self.episode = Some(Episode {
            id: uuid::Uuid::new_v4().to_string(),
            total_reward: 0.0,
            steps: 0,
            truncated: false,
            start_time: chrono::Utc::now(),
            end_time: None,
        });
        self.step_count = 0;
        
        self.env.reset().await
    }
    
    async fn step(&mut self, action: Self::Action) -> crate::Result<Step<Self::Observation, Self::State>> {
        let step = self.env.step(action).await?;
        
        self.step_count += 1;
        if let Some(ref mut episode) = self.episode {
            episode.total_reward += step.reward.0;
            episode.steps = self.step_count;
            
            if step.done || step.truncated {
                episode.truncated = step.truncated;
                episode.end_time = Some(chrono::Utc::now());
            }
        }
        
        Ok(step)
    }
    
    async fn render(&self) -> crate::Result<()> {
        self.env.render().await
    }
    
    async fn close(&mut self) -> crate::Result<()> {
        self.env.close().await
    }
    
    fn episode_info(&self) -> Option<Episode> {
        self.episode.clone()
    }
}

// Add uuid to dependencies in Cargo.toml
const _: &str = r#"
[dependencies]
uuid = { version = "1.6", features = ["v4", "serde"] }
"#;