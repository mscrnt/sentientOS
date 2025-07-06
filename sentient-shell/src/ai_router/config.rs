//! Configuration loader for model routing

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use lazy_static::lazy_static;

/// Performance tier for models
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PerformanceTier {
    #[serde(rename = "fast")]
    Fast,
    #[serde(rename = "balanced")]
    Balanced,
    #[serde(rename = "powerful")]
    Powerful,
    #[serde(rename = "specialized")]
    Specialized,
}

/// Model configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ModelConfig {
    pub name: String,
    pub provider: String,
    pub endpoint: Option<String>,
    pub location: Option<String>,
    pub model_id: Option<String>,
    pub trusted: bool,
    pub allow_tool_calls: bool,
    pub offline_only: Option<bool>,
    pub capabilities: Vec<String>,
    pub performance_tier: PerformanceTier,
    pub context_length: usize,
    pub priority: u32,
    pub use_cases: Vec<String>,
    pub safety_notes: Option<String>,
}

/// Routing configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RoutingConfig {
    pub default_model: String,
    pub offline_chain: Vec<String>,
    pub intents: HashMap<String, Vec<String>>,
    pub performance: HashMap<String, Vec<String>>,
    pub context: HashMap<String, Vec<String>>,
}

/// Load balancing configuration
#[derive(Debug, Clone, Deserialize)]
pub struct LoadBalancingConfig {
    pub strategy: String,
    pub max_concurrent_requests: usize,
    pub timeout_ms: u64,
    pub retry_attempts: u32,
}

/// Full models configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ModelsConfig {
    pub models: HashMap<String, ModelConfig>,
    pub routing: RoutingConfig,
    pub load_balancing: LoadBalancingConfig,
}

lazy_static! {
    static ref MODELS_CONFIG: Arc<RwLock<Option<ModelsConfig>>> = Arc::new(RwLock::new(None));
}

/// Load models configuration from file
pub fn load_models_config(path: impl AsRef<Path>) -> Result<()> {
    let content = std::fs::read_to_string(path.as_ref())
        .context("Failed to read models config file")?;
    
    let config: ModelsConfig = toml::from_str(&content)
        .context("Failed to parse models config")?;
    
    // Validate configuration
    validate_config(&config)?;
    
    // Store in global state
    let mut global_config = MODELS_CONFIG.write().unwrap();
    *global_config = Some(config);
    
    Ok(())
}

/// Get models configuration
pub fn get_models_config() -> Option<ModelsConfig> {
    MODELS_CONFIG.read().unwrap().clone()
}

/// Validate configuration
fn validate_config(config: &ModelsConfig) -> Result<()> {
    // Check default model exists
    if !config.models.contains_key(&config.routing.default_model) {
        anyhow::bail!("Default model '{}' not found in models", config.routing.default_model);
    }
    
    // Check offline chain models exist
    for model in &config.routing.offline_chain {
        if !config.models.contains_key(model) {
            anyhow::bail!("Offline chain model '{}' not found in models", model);
        }
    }
    
    // Check intent models exist
    for (intent, models) in &config.routing.intents {
        for model in models {
            if !config.models.contains_key(model) {
                anyhow::bail!("Intent '{}' references unknown model '{}'", intent, model);
            }
        }
    }
    
    Ok(())
}

/// Get models for a specific intent
pub fn get_models_for_intent(intent: &str) -> Vec<String> {
    if let Some(config) = get_models_config() {
        config.routing.intents.get(intent)
            .cloned()
            .unwrap_or_else(|| vec![config.routing.default_model.clone()])
    } else {
        vec![]
    }
}

/// Get offline fallback models
pub fn get_offline_models() -> Vec<String> {
    get_models_config()
        .map(|c| c.routing.offline_chain.clone())
        .unwrap_or_default()
}

/// Get model configuration by ID
pub fn get_model_config(model_id: &str) -> Option<ModelConfig> {
    get_models_config()
        .and_then(|c| c.models.get(model_id).cloned())
}

/// Get models by performance tier
pub fn get_models_by_tier(tier: PerformanceTier) -> Vec<(String, ModelConfig)> {
    if let Some(config) = get_models_config() {
        config.models.iter()
            .filter(|(_, m)| m.performance_tier == tier)
            .map(|(id, m)| (id.clone(), m.clone()))
            .collect()
    } else {
        vec![]
    }
}

/// Check if a model supports a capability
pub fn model_supports_capability(model_id: &str, capability: &str) -> bool {
    get_model_config(model_id)
        .map(|m| m.capabilities.contains(&capability.to_string()))
        .unwrap_or(false)
}

/// Get recommended model for context length
pub fn get_model_for_context(tokens: usize) -> Option<String> {
    if let Some(config) = get_models_config() {
        // Find smallest model that fits
        let mut candidates: Vec<_> = config.models.iter()
            .filter(|(_, m)| m.context_length >= tokens)
            .collect();
        
        // Sort by context length (ascending) and priority (descending)
        candidates.sort_by(|(_, a), (_, b)| {
            a.context_length.cmp(&b.context_length)
                .then(b.priority.cmp(&a.priority))
        });
        
        candidates.first().map(|(id, _)| id.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_load_valid_config() {
        let config_content = r#"
[models.test_model]
name = "Test Model"
provider = "local"
capabilities = ["tool_calling"]
performance_tier = "fast"
context_length = 2048
priority = 100
use_cases = ["testing"]

[routing]
default_model = "test_model"
offline_chain = ["test_model"]

[routing.intents]
tool_call = ["test_model"]

[routing.performance]
realtime = ["test_model"]

[routing.context]
short = ["test_model"]

[load_balancing]
strategy = "capability_first"
max_concurrent_requests = 3
timeout_ms = 30000
retry_attempts = 2
"#;
        
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(config_content.as_bytes()).unwrap();
        
        assert!(load_models_config(temp_file.path()).is_ok());
        assert!(get_models_config().is_some());
    }
}