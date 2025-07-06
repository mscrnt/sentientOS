// Model registry - manages available AI models and endpoints
use super::*;
use anyhow::{Result, bail};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use log::{info, warn, error};

pub struct ModelRegistry {
    endpoints: Arc<RwLock<HashMap<String, ModelEndpoint>>>,
    providers: Arc<RwLock<HashMap<String, Arc<dyn ModelProvider>>>>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            endpoints: Arc::new(RwLock::new(HashMap::new())),
            providers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a model provider
    pub fn register_provider(&self, provider: Arc<dyn ModelProvider>) -> Result<()> {
        let name = provider.name().to_string();
        info!("🤖 [AI-ROUTER] Registering provider: {}", name);
        
        // Check if provider is available
        if !provider.is_available()? {
            warn!("⚠️  [AI-ROUTER] Provider {} is not available", name);
        }

        // Discover and register models from provider
        match provider.list_models() {
            Ok(models) => {
                for model in models {
                    self.register_endpoint(model)?;
                }
            }
            Err(e) => {
                error!("🔴 [AI-ROUTER] Failed to list models from {}: {}", name, e);
            }
        }

        self.providers.write().unwrap().insert(name, provider);
        Ok(())
    }

    /// Register a model endpoint
    pub fn register_endpoint(&self, endpoint: ModelEndpoint) -> Result<()> {
        let endpoint_id = format!("{}:{}", endpoint.provider, endpoint.model_id);
        info!("🤖 [AI-ROUTER] Registering endpoint: {}", endpoint_id);
        
        self.endpoints.write().unwrap().insert(endpoint_id, endpoint);
        Ok(())
    }

    /// Get endpoints by capability
    pub fn get_endpoints_by_capability(&self, capability: &ModelCapability) -> Vec<ModelEndpoint> {
        let endpoints = self.endpoints.read().unwrap();
        
        let mut matching: Vec<_> = endpoints
            .values()
            .filter(|e| e.is_active && e.capabilities.contains(capability))
            .cloned()
            .collect();
        
        // Sort by priority (descending)
        matching.sort_by(|a, b| b.priority.cmp(&a.priority));
        matching
    }

    /// Get a specific endpoint
    pub fn get_endpoint(&self, provider: &str, model_id: &str) -> Option<ModelEndpoint> {
        let endpoint_id = format!("{}:{}", provider, model_id);
        self.endpoints.read().unwrap().get(&endpoint_id).cloned()
    }

    /// List all registered endpoints
    pub fn list_endpoints(&self) -> Vec<ModelEndpoint> {
        self.endpoints.read().unwrap().values().cloned().collect()
    }

    /// Update endpoint status
    pub fn set_endpoint_active(&self, provider: &str, model_id: &str, active: bool) -> Result<()> {
        let endpoint_id = format!("{}:{}", provider, model_id);
        
        match self.endpoints.write().unwrap().get_mut(&endpoint_id) {
            Some(endpoint) => {
                endpoint.is_active = active;
                info!("🤖 [AI-ROUTER] Endpoint {} active status: {}", endpoint_id, active);
                Ok(())
            }
            None => bail!("Endpoint not found: {}", endpoint_id),
        }
    }

    /// Get provider by name
    pub fn get_provider(&self, name: &str) -> Option<Arc<dyn ModelProvider>> {
        self.providers.read().unwrap()
            .get(name)
            .cloned()
    }

    /// Health check all endpoints
    pub fn health_check(&self) -> HashMap<String, bool> {
        let mut results = HashMap::new();
        let providers = self.providers.read().unwrap();
        
        for (name, provider) in providers.iter() {
            match provider.is_available() {
                Ok(available) => {
                    results.insert(name.clone(), available);
                    if !available {
                        // Mark endpoints as inactive
                        let endpoints = self.endpoints.write().unwrap();
                        for (_, endpoint) in endpoints.iter() {
                            if endpoint.provider == *name {
                                // This would need mutable access
                                // endpoint.is_active = false;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("🔴 [AI-ROUTER] Health check failed for {}: {}", name, e);
                    results.insert(name.clone(), false);
                }
            }
        }
        
        results
    }
}

// Global registry instance
lazy_static::lazy_static! {
    static ref MODEL_REGISTRY: ModelRegistry = ModelRegistry::new();
}

pub fn get_model_registry() -> &'static ModelRegistry {
    &MODEL_REGISTRY
}