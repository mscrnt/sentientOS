//! Intelligent LLM routing with intent detection and capability matching

use super::{ModelEndpoint, InferenceRequest, InferenceResponse};
use anyhow::{Result, Context, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use log::{info, debug, warn};

/// Model capability flags
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    ToolCalling,
    BasicReasoning,
    AdvancedReasoning,
    CodeGeneration,
    CommandInterpretation,
    SystemDiagnostics,
    GeneralReasoning,
    Conversation,
    InstructionFollowing,
    FastInference,
    LongContext,
    ToolOrchestration,
    ComplexAnalysis,
    VisionUnderstanding,
    ScreenshotAnalysis,
    UIInteraction,
    VisualDebugging,
}

/// Performance tier for models
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PerformanceTier {
    #[serde(rename = "fast")]
    Fast,           // <100ms typical
    #[serde(rename = "balanced")]
    Balanced,       // <500ms typical
    #[serde(rename = "powerful")]
    Powerful,       // No latency constraint
    #[serde(rename = "specialized")]
    Specialized,    // Special purpose
}

/// Intent detected from user prompt
#[derive(Debug, Clone, PartialEq)]
pub enum Intent {
    ToolCall,
    CodeGeneration,
    SystemAnalysis,
    QuickResponse,
    VisualAnalysis,
    ComplexReasoning,
    GeneralQuery,
}

/// Model configuration from TOML
#[derive(Debug, Clone, Deserialize)]
pub struct ModelConfig {
    pub name: String,
    pub provider: String,
    pub endpoint: Option<String>,
    pub location: Option<String>,
    pub model_id: Option<String>,
    pub capabilities: Vec<String>,
    pub performance_tier: PerformanceTier,
    pub context_length: usize,
    pub priority: u32,
    pub use_cases: Vec<String>,
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

/// Full configuration structure
#[derive(Debug, Clone, Deserialize)]
pub struct ModelsConfig {
    pub models: HashMap<String, ModelConfig>,
    pub routing: RoutingConfig,
    pub load_balancing: LoadBalancingConfig,
}

/// Model health status
#[derive(Debug, Clone)]
struct ModelHealth {
    pub available: bool,
    pub last_check: Instant,
    pub latency_ms: Option<u64>,
    pub error_count: u32,
}

/// Intelligent router for LLM selection
pub struct IntelligentRouter {
    config: ModelsConfig,
    health: Arc<RwLock<HashMap<String, ModelHealth>>>,
    providers: Arc<RwLock<HashMap<String, Box<dyn super::ModelProvider>>>>,
}

impl IntelligentRouter {
    /// Create new intelligent router
    pub fn new(config_path: &str) -> Result<Self> {
        // Load configuration
        let config_str = std::fs::read_to_string(config_path)
            .context("Failed to read models config")?;
        let config: ModelsConfig = toml::from_str(&config_str)
            .context("Failed to parse models config")?;
        
        let router = Self {
            config,
            health: Arc::new(RwLock::new(HashMap::new())),
            providers: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Initialize health tracking
        router.init_health_tracking();
        
        Ok(router)
    }
    
    /// Initialize health tracking for all models
    fn init_health_tracking(&self) {
        let mut health = self.health.write().unwrap();
        for (model_id, _) in &self.config.models {
            health.insert(model_id.clone(), ModelHealth {
                available: true,  // Assume available initially
                last_check: Instant::now(),
                latency_ms: None,
                error_count: 0,
            });
        }
    }
    
    /// Route request to best available model
    pub async fn route(&self, request: InferenceRequest) -> Result<InferenceResponse> {
        // Detect intent from prompt
        let intent = self.detect_intent(&request.prompt);
        debug!("Detected intent: {:?}", intent);
        
        // Get candidate models based on intent
        let candidates = self.get_candidate_models(&intent, &request)?;
        debug!("Candidate models: {:?}", candidates);
        
        // Try models in order
        for model_id in candidates {
            match self.try_model(&model_id, &request).await {
                Ok(response) => {
                    // Update health on success
                    self.update_health(&model_id, true, None);
                    return Ok(response);
                },
                Err(e) => {
                    warn!("Model {} failed: {}", model_id, e);
                    self.update_health(&model_id, false, Some(e.to_string()));
                    continue;
                }
            }
        }
        
        bail!("All models failed to process request")
    }
    
    /// Detect intent from prompt
    fn detect_intent(&self, prompt: &str) -> Intent {
        let prompt_lower = prompt.to_lowercase();
        
        // Tool calling patterns
        if prompt_lower.contains("!@") || 
           prompt_lower.contains("!$") ||
           prompt_lower.contains("call") && prompt_lower.contains("tool") ||
           prompt_lower.contains("execute") ||
           prompt_lower.contains("run command") {
            return Intent::ToolCall;
        }
        
        // Code generation patterns
        if prompt_lower.contains("write") && (
               prompt_lower.contains("code") || 
               prompt_lower.contains("function") ||
               prompt_lower.contains("class") ||
               prompt_lower.contains("program")) ||
           prompt_lower.contains("implement") ||
           prompt_lower.contains("create") && prompt_lower.contains("script") {
            return Intent::CodeGeneration;
        }
        
        // System analysis patterns
        if prompt_lower.contains("analyze") ||
           prompt_lower.contains("diagnose") ||
           prompt_lower.contains("debug") ||
           prompt_lower.contains("system") && (
               prompt_lower.contains("health") ||
               prompt_lower.contains("status") ||
               prompt_lower.contains("performance")) {
            return Intent::SystemAnalysis;
        }
        
        // Visual analysis patterns
        if prompt_lower.contains("screenshot") ||
           prompt_lower.contains("image") ||
           prompt_lower.contains("visual") ||
           prompt_lower.contains("see") ||
           prompt_lower.contains("look at") {
            return Intent::VisualAnalysis;
        }
        
        // Complex reasoning patterns
        if prompt_lower.contains("explain") && prompt_lower.contains("how") ||
           prompt_lower.contains("compare") ||
           prompt_lower.contains("analyze") && prompt_lower.contains("why") ||
           prompt_lower.len() > 200 {  // Long prompts often need reasoning
            return Intent::ComplexReasoning;
        }
        
        // Quick response patterns
        if prompt_lower.split_whitespace().count() < 10 &&
           !prompt_lower.contains("?") {
            return Intent::QuickResponse;
        }
        
        // Default to general query
        Intent::GeneralQuery
    }
    
    /// Get candidate models based on intent and request
    fn get_candidate_models(&self, intent: &Intent, request: &InferenceRequest) -> Result<Vec<String>> {
        let mut candidates = Vec::new();
        
        // Get intent-based models
        let intent_key = match intent {
            Intent::ToolCall => "tool_call",
            Intent::CodeGeneration => "code_generation",
            Intent::SystemAnalysis => "system_analysis",
            Intent::QuickResponse => "quick_response",
            Intent::VisualAnalysis => "visual_analysis",
            Intent::ComplexReasoning => "complex_reasoning",
            Intent::GeneralQuery => {
                // Use default model for general queries
                candidates.push(self.config.routing.default_model.clone());
                return Ok(candidates);
            }
        };
        
        if let Some(models) = self.config.routing.intents.get(intent_key) {
            candidates.extend(models.clone());
        }
        
        // Filter by availability
        let health = self.health.read().unwrap();
        candidates.retain(|model_id| {
            health.get(model_id)
                .map(|h| h.available && h.error_count < 3)
                .unwrap_or(false)
        });
        
        // Sort by priority
        candidates.sort_by_key(|model_id| {
            self.config.models.get(model_id)
                .map(|m| std::cmp::Reverse(m.priority))
                .unwrap_or(std::cmp::Reverse(0))
        });
        
        // Add offline fallback if no candidates
        if candidates.is_empty() && self.is_offline() {
            candidates.extend(self.config.routing.offline_chain.clone());
        }
        
        Ok(candidates)
    }
    
    /// Try to execute request with specific model
    async fn try_model(&self, model_id: &str, request: &InferenceRequest) -> Result<InferenceResponse> {
        let model_config = self.config.models.get(model_id)
            .ok_or_else(|| anyhow::anyhow!("Model {} not found", model_id))?;
        
        // Get or create provider
        let provider = self.get_provider(model_id, model_config)?;
        
        // Check context length
        let estimated_tokens = request.prompt.len() / 4; // Rough estimate
        if estimated_tokens > model_config.context_length {
            bail!("Prompt too long for model {} (estimated {} tokens, max {})", 
                  model_id, estimated_tokens, model_config.context_length);
        }
        
        // Create endpoint
        let endpoint = ModelEndpoint {
            id: model_id.to_string(),
            name: model_config.name.clone(),
            provider: model_config.provider.clone(),
            capabilities: self.parse_capabilities(&model_config.capabilities),
        };
        
        // Execute request with timeout
        let start = Instant::now();
        let timeout = Duration::from_millis(self.config.load_balancing.timeout_ms);
        
        let response = tokio::time::timeout(timeout, provider.infer(&endpoint, request))
            .await
            .context("Request timed out")?
            .context("Inference failed")?;
        
        // Record latency
        let latency_ms = start.elapsed().as_millis() as u64;
        self.update_latency(model_id, latency_ms);
        
        Ok(response)
    }
    
    /// Get or create provider for model
    fn get_provider(&self, model_id: &str, config: &ModelConfig) -> Result<Box<dyn super::ModelProvider>> {
        // This is a simplified version - in practice, you'd create appropriate providers
        match config.provider.as_str() {
            "local" => {
                // Return local/boot provider
                Ok(Box::new(super::providers::boot::BootModelProvider::new()))
            },
            "ollama" => {
                // Return Ollama provider
                let endpoint = config.endpoint.as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Ollama endpoint not specified"))?;
                Ok(Box::new(super::providers::ollama::OllamaProvider::new(endpoint.clone())))
            },
            _ => bail!("Unknown provider: {}", config.provider),
        }
    }
    
    /// Parse capability strings
    fn parse_capabilities(&self, caps: &[String]) -> HashMap<String, serde_json::Value> {
        let mut result = HashMap::new();
        for cap in caps {
            result.insert(cap.clone(), serde_json::json!(true));
        }
        result
    }
    
    /// Check if system is offline
    fn is_offline(&self) -> bool {
        // Check if remote models are unavailable
        let health = self.health.read().unwrap();
        let remote_available = self.config.models.iter()
            .filter(|(_, config)| config.provider != "local")
            .any(|(id, _)| {
                health.get(id).map(|h| h.available).unwrap_or(false)
            });
        
        !remote_available
    }
    
    /// Update model health status
    fn update_health(&self, model_id: &str, success: bool, error: Option<String>) {
        let mut health = self.health.write().unwrap();
        if let Some(status) = health.get_mut(model_id) {
            status.last_check = Instant::now();
            if success {
                status.available = true;
                status.error_count = 0;
            } else {
                status.error_count += 1;
                if status.error_count >= 3 {
                    status.available = false;
                }
            }
        }
    }
    
    /// Update model latency
    fn update_latency(&self, model_id: &str, latency_ms: u64) {
        let mut health = self.health.write().unwrap();
        if let Some(status) = health.get_mut(model_id) {
            status.latency_ms = Some(latency_ms);
        }
    }
    
    /// Get routing information for CLI
    pub fn get_routing_info(&self) -> HashMap<String, serde_json::Value> {
        let mut info = HashMap::new();
        
        // Model status
        let health = self.health.read().unwrap();
        let models_status: HashMap<String, serde_json::Value> = self.config.models.iter()
            .map(|(id, config)| {
                let status = health.get(id);
                (id.clone(), serde_json::json!({
                    "name": config.name,
                    "provider": config.provider,
                    "tier": config.performance_tier,
                    "available": status.map(|s| s.available).unwrap_or(false),
                    "latency_ms": status.and_then(|s| s.latency_ms),
                    "priority": config.priority,
                }))
            })
            .collect();
        
        info.insert("models".to_string(), serde_json::json!(models_status));
        info.insert("routing_rules".to_string(), serde_json::json!(self.config.routing));
        info.insert("offline_mode".to_string(), serde_json::json!(self.is_offline()));
        
        info
    }
    
    /// Test routing decision for a prompt
    pub fn test_routing(&self, prompt: &str) -> Result<HashMap<String, serde_json::Value>> {
        let intent = self.detect_intent(prompt);
        let request = InferenceRequest {
            prompt: prompt.to_string(),
            max_tokens: None,
            temperature: None,
            system_prompt: None,
        };
        
        let candidates = self.get_candidate_models(&intent, &request)?;
        
        Ok(HashMap::from([
            ("intent".to_string(), serde_json::json!(format!("{:?}", intent))),
            ("candidates".to_string(), serde_json::json!(candidates)),
            ("selected".to_string(), serde_json::json!(candidates.first())),
        ]))
    }
}