//! Enhanced router with intent-based routing

use super::*;
use super::intent::{IntentDetector, Intent};
use super::config::{get_models_config, get_model_config, get_models_for_intent};
use super::registry::get_model_registry;
use anyhow::{Result, bail, Context};
use std::time::Instant;
use log::{info, debug, warn};

/// Enhanced AI Router with intent detection
pub struct EnhancedAIRouter {
    fallback_chain: Vec<String>,
}

impl EnhancedAIRouter {
    /// Create new enhanced router
    pub fn new() -> Self {
        // Load configuration if not already loaded
        if get_models_config().is_none() {
            if let Err(e) = super::config::load_models_config("config/models.toml") {
                warn!("Failed to load models config: {}. Using defaults.", e);
            }
        }
        
        let fallback_chain = get_models_config()
            .map(|c| c.routing.offline_chain.clone())
            .unwrap_or_else(|| vec!["phi2_local".to_string()]);
        
        Self { fallback_chain }
    }
    
    /// Route request with intent detection
    pub fn route_with_intent(&self, prompt: &str) -> Result<InferenceResponse> {
        let start_time = Instant::now();
        
        // Detect intent
        let intent = IntentDetector::detect(prompt);
        info!("🧠 [INTENT] Detected: {:?}", intent);
        
        // Convert intent to capability
        let capability = IntentDetector::intent_to_capability(&intent);
        
        // Create inference request
        let request = InferenceRequest {
            prompt: prompt.to_string(),
            capability: capability.clone(),
            max_tokens: Some(self.estimate_max_tokens(&intent)),
            temperature: Some(self.get_temperature(&intent)),
            system_prompt: self.get_system_prompt(&intent),
            metadata: HashMap::new(),
        };
        
        // Get recommended models
        let mut model_chain = self.get_model_chain(&intent, &request)?;
        
        info!("🎯 [ROUTER] Model chain: {:?}", model_chain);
        
        // Try each model in the chain
        let mut last_error = None;
        
        for model_id in model_chain {
            match self.try_model(&model_id, &request) {
                Ok(mut response) => {
                    response.duration_ms = start_time.elapsed().as_millis() as u64;
                    info!("✅ [ROUTER] Completed by {} in {}ms", model_id, response.duration_ms);
                    return Ok(response);
                },
                Err(e) => {
                    warn!("⚠️  [ROUTER] Model {} failed: {}", model_id, e);
                    last_error = Some(e);
                }
            }
        }
        
        // All models failed - try fallback
        warn!("⚠️  [ROUTER] All primary models failed, trying fallback chain");
        
        for model_id in &self.fallback_chain {
            match self.try_model(model_id, &request).await {
                Ok(mut response) => {
                    response.duration_ms = start_time.elapsed().as_millis() as u64;
                    info!("✅ [ROUTER] Fallback {} succeeded in {}ms", model_id, response.duration_ms);
                    return Ok(response);
                },
                Err(e) => {
                    warn!("⚠️  [ROUTER] Fallback {} failed: {}", model_id, e);
                    last_error = Some(e);
                }
            }
        }
        
        bail!("All models failed. Last error: {:?}", last_error)
    }
    
    /// Get model chain for intent
    fn get_model_chain(&self, intent: &Intent, request: &InferenceRequest) -> Result<Vec<String>> {
        let mut chain = Vec::new();
        
        // Get intent-specific models from config
        let intent_key = match intent {
            Intent::ToolCall => "tool_call",
            Intent::CodeGeneration => "code_generation",
            Intent::SystemAnalysis => "system_analysis",
            Intent::QuickResponse => "quick_response",
            Intent::VisualAnalysis => "visual_analysis",
            Intent::ComplexReasoning => "complex_reasoning",
            _ => "general",
        };
        
        chain.extend(get_models_for_intent(intent_key));
        
        // Also add recommended models
        for model in IntentDetector::recommended_models(intent) {
            if !chain.contains(&model.to_string()) {
                chain.push(model.to_string());
            }
        }
        
        // Filter by availability
        let registry = get_model_registry();
        chain.retain(|model_id| {
            // Check if model is registered
            if let Some(config) = get_model_config(model_id) {
                // Check if provider exists
                registry.get_provider(&config.provider).is_some()
            } else {
                false
            }
        });
        
        Ok(chain)
    }
    
    /// Try to execute request with specific model
    fn try_model(&self, model_id: &str, request: &InferenceRequest) -> Result<InferenceResponse> {
        let config = get_model_config(model_id)
            .ok_or_else(|| anyhow::anyhow!("Model {} not found in config", model_id))?;
        
        let registry = get_model_registry();
        
        // Create endpoint
        let endpoint = ModelEndpoint {
            name: config.name.clone(),
            provider: config.provider.clone(),
            model_id: config.model_id.clone().unwrap_or_else(|| model_id.to_string()),
            endpoint_url: config.endpoint.clone().unwrap_or_default(),
            capabilities: vec![request.capability.clone()],
            max_tokens: Some(config.context_length),
            context_window: Some(config.context_length),
            is_active: true,
            priority: config.priority as i32,
        };
        
        // Check context length
        if let Some(max_tokens) = request.max_tokens {
            if max_tokens > config.context_length {
                bail!("Context too long for model {} ({} > {})", 
                      model_id, max_tokens, config.context_length);
            }
        }
        
        // Get provider
        let provider = registry.get_provider(&config.provider)
            .ok_or_else(|| anyhow::anyhow!("Provider {} not found", config.provider))?;
        
        // Make request
        let mut response = provider.infer(request)
            .context(format!("Inference failed for model {}", model_id))?;
        
        response.model_used = model_id.to_string();
        
        Ok(response)
    }
    
    /// Estimate max tokens for intent
    fn estimate_max_tokens(&self, intent: &Intent) -> usize {
        match intent {
            Intent::ToolCall | Intent::CommandExecution => 500,
            Intent::QuickResponse => 200,
            Intent::CodeGeneration => 2000,
            Intent::SystemAnalysis => 1500,
            Intent::ComplexReasoning => 3000,
            Intent::Documentation => 2500,
            _ => 1000,
        }
    }
    
    /// Get temperature for intent
    fn get_temperature(&self, intent: &Intent) -> f32 {
        match intent {
            Intent::ToolCall | Intent::CommandExecution => 0.1, // Deterministic
            Intent::CodeGeneration => 0.3,                      // Some creativity
            Intent::ComplexReasoning => 0.5,                    // Balanced
            Intent::Conversation => 0.7,                        // More variety
            _ => 0.4,
        }
    }
    
    /// Get system prompt for intent
    fn get_system_prompt(&self, intent: &Intent) -> Option<String> {
        match intent {
            Intent::ToolCall => Some(
                "You are a tool execution assistant. Parse the user's request and identify \
                 which tools to call. Use the format !@ tool_name {args} for tool calls.".to_string()
            ),
            Intent::CodeGeneration => Some(
                "You are an expert programmer. Generate clean, well-documented code that \
                 follows best practices. Include error handling and comments.".to_string()
            ),
            Intent::SystemAnalysis => Some(
                "You are a system administrator. Analyze the system state and provide \
                 actionable insights. Be specific about issues and solutions.".to_string()
            ),
            _ => None,
        }
    }
    
    /// Test routing for a prompt
    pub fn test_routing(&self, prompt: &str) -> HashMap<String, serde_json::Value> {
        let intent = IntentDetector::detect(prompt);
        let capability = IntentDetector::intent_to_capability(&intent);
        
        let request = InferenceRequest {
            prompt: prompt.to_string(),
            capability,
            max_tokens: Some(1000),
            temperature: Some(0.5),
            system_prompt: None,
            metadata: HashMap::new(),
        };
        
        let model_chain = self.get_model_chain(&intent, &request)
            .unwrap_or_default();
        
        HashMap::from([
            ("intent".to_string(), serde_json::json!(format!("{:?}", intent))),
            ("recommended_models".to_string(), serde_json::json!(model_chain)),
            ("estimated_tokens".to_string(), serde_json::json!(self.estimate_max_tokens(&intent))),
            ("temperature".to_string(), serde_json::json!(self.get_temperature(&intent))),
        ])
    }
}

