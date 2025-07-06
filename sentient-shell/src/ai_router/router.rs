// Request routing logic
use super::*;
use super::registry::get_model_registry;
use anyhow::{Result, bail};
use std::time::Instant;
use log::{info, debug, warn, error};

pub struct AIRouter;

impl AIRouter {
    /// Route an inference request to the best available model
    pub fn route_request(request: &InferenceRequest) -> Result<InferenceResponse> {
        let start_time = Instant::now();
        let registry = get_model_registry();
        
        // Find suitable endpoints
        let endpoints = registry.get_endpoints_by_capability(&request.capability);
        
        if endpoints.is_empty() {
            bail!("No models available for capability: {:?}", request.capability);
        }
        
        debug!("🤖 [AI-ROUTER] Found {} endpoints for {:?}", endpoints.len(), request.capability);
        
        // Try endpoints in priority order
        let mut last_error = None;
        
        for endpoint in endpoints {
            debug!("🤖 [AI-ROUTER] Trying endpoint: {}:{}", endpoint.provider, endpoint.model_id);
            
            // Check token limits
            if let (Some(max_tokens), Some(context_window)) = (request.max_tokens, endpoint.context_window) {
                if max_tokens > context_window {
                    debug!("⚠️  [AI-ROUTER] Skipping {} - token limit exceeded", endpoint.name);
                    continue;
                }
            }
            
            // Get provider and make request
            if let Some(provider) = registry.get_provider(&endpoint.provider) {
                match provider.infer(request) {
                    Ok(mut response) => {
                        response.model_used = format!("{}:{}", endpoint.provider, endpoint.model_id);
                        response.duration_ms = start_time.elapsed().as_millis() as u64;
                        
                        info!("✅ [AI-ROUTER] Request completed by {} in {}ms", 
                            response.model_used, response.duration_ms);
                        
                        return Ok(response);
                    }
                    Err(e) => {
                        warn!("⚠️  [AI-ROUTER] Failed with {}: {}", endpoint.name, e);
                        last_error = Some(e);
                        
                        // Mark endpoint as inactive on repeated failures
                        // This would need more sophisticated error tracking
                    }
                }
            }
        }
        
        bail!("All endpoints failed. Last error: {:?}", last_error)
    }
    
    /// Route a request to a specific model
    pub fn route_to_model(
        provider: &str, 
        model_id: &str, 
        request: &InferenceRequest
    ) -> Result<InferenceResponse> {
        let registry = get_model_registry();
        
        // Check if endpoint exists and is active
        match registry.get_endpoint(provider, model_id) {
            Some(endpoint) if endpoint.is_active => {
                // Get provider and make request
                match registry.get_provider(provider) {
                    Some(provider) => {
                        let start_time = Instant::now();
                        let mut response = provider.infer(request)?;
                        
                        response.model_used = format!("{}:{}", endpoint.provider, model_id);
                        response.duration_ms = start_time.elapsed().as_millis() as u64;
                        
                        Ok(response)
                    }
                    None => bail!("Provider not found: {}", provider),
                }
            }
            Some(_) => bail!("Endpoint is not active: {}:{}", provider, model_id),
            None => bail!("Endpoint not found: {}:{}", provider, model_id),
        }
    }
    
    /// Get routing statistics
    pub fn get_stats() -> RouterStats {
        // This would track actual usage statistics
        RouterStats {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_latency_ms: 0,
            endpoint_usage: HashMap::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RouterStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_latency_ms: u64,
    pub endpoint_usage: HashMap<String, u64>,
}

/// Load balancing strategies
#[derive(Debug, Clone, PartialEq)]
pub enum LoadBalancingStrategy {
    Priority,      // Use highest priority endpoint
    RoundRobin,    // Rotate between endpoints
    LeastLatency,  // Use endpoint with lowest average latency
    Random,        // Random selection
}

pub struct LoadBalancer {
    strategy: LoadBalancingStrategy,
    round_robin_index: std::sync::atomic::AtomicUsize,
}

impl LoadBalancer {
    pub fn new(strategy: LoadBalancingStrategy) -> Self {
        Self {
            strategy,
            round_robin_index: std::sync::atomic::AtomicUsize::new(0),
        }
    }
    
    pub fn select_endpoint(&self, endpoints: Vec<ModelEndpoint>) -> Option<ModelEndpoint> {
        if endpoints.is_empty() {
            return None;
        }
        
        match self.strategy {
            LoadBalancingStrategy::Priority => {
                // Already sorted by priority
                endpoints.into_iter().next()
            }
            LoadBalancingStrategy::RoundRobin => {
                let len = endpoints.len();
                let index = self.round_robin_index.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                endpoints.into_iter().nth(index % len)
            }
            LoadBalancingStrategy::Random => {
                use rand::seq::SliceRandom;
                let mut rng = rand::thread_rng();
                endpoints.choose(&mut rng).cloned()
            }
            LoadBalancingStrategy::LeastLatency => {
                // Would need latency tracking
                endpoints.into_iter().next()
            }
        }
    }
}