//! Environment registry for easy environment creation

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use sentient_rl_core::{Environment, EnvironmentConfig};

type EnvConstructor = Box<dyn Fn(EnvironmentConfig) -> sentient_rl_core::Result<Box<dyn Environment<
    Observation = Box<dyn sentient_rl_core::Observation>,
    Action = Box<dyn sentient_rl_core::Action>,
    State = Box<dyn sentient_rl_core::State>,
>>> + Send + Sync>;

lazy_static::lazy_static! {
    static ref REGISTRY: Arc<Mutex<EnvRegistry>> = Arc::new(Mutex::new(EnvRegistry::new()));
}

/// Global environment registry
pub struct EnvRegistry {
    /// Registered environments
    envs: HashMap<String, EnvConstructor>,
}

impl EnvRegistry {
    /// Create a new registry
    fn new() -> Self {
        Self {
            envs: HashMap::new(),
        }
    }
    
    /// Register an environment
    pub fn register<F>(&mut self, name: impl Into<String>, constructor: F)
    where
        F: Fn(EnvironmentConfig) -> sentient_rl_core::Result<Box<dyn Environment<
            Observation = Box<dyn sentient_rl_core::Observation>,
            Action = Box<dyn sentient_rl_core::Action>,
            State = Box<dyn sentient_rl_core::State>,
        >>> + Send + Sync + 'static,
    {
        self.envs.insert(name.into(), Box::new(constructor));
    }
    
    /// Create an environment by name
    pub fn make(&self, name: &str, config: EnvironmentConfig) -> sentient_rl_core::Result<Box<dyn Environment<
        Observation = Box<dyn sentient_rl_core::Observation>,
        Action = Box<dyn sentient_rl_core::Action>,
        State = Box<dyn sentient_rl_core::State>,
    >>> {
        self.envs
            .get(name)
            .ok_or_else(|| sentient_rl_core::RLError::Environment(format!("Unknown environment: {}", name)))
            .and_then(|constructor| constructor(config))
    }
    
    /// List registered environments
    pub fn list(&self) -> Vec<String> {
        self.envs.keys().cloned().collect()
    }
}

/// Register an environment globally
pub fn register_env<F>(name: impl Into<String>, constructor: F)
where
    F: Fn(EnvironmentConfig) -> sentient_rl_core::Result<Box<dyn Environment<
        Observation = Box<dyn sentient_rl_core::Observation>,
        Action = Box<dyn sentient_rl_core::Action>,
        State = Box<dyn sentient_rl_core::State>,
    >>> + Send + Sync + 'static,
{
    REGISTRY.lock().unwrap().register(name, constructor);
}

/// Create an environment by name
pub fn make_env(name: &str, config: EnvironmentConfig) -> sentient_rl_core::Result<Box<dyn Environment<
    Observation = Box<dyn sentient_rl_core::Observation>,
    Action = Box<dyn sentient_rl_core::Action>,
    State = Box<dyn sentient_rl_core::State>,
>>> {
    REGISTRY.lock().unwrap().make(name, config)
}

/// List all registered environments
pub fn list_envs() -> Vec<String> {
    REGISTRY.lock().unwrap().list()
}

// Add lazy_static to dependencies
const _: &str = r#"
[dependencies]
lazy_static = "1.4"
"#;