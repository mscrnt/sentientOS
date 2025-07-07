// AI Model Router for SentientOS
// Central registry and router for AI inference requests

pub mod registry;
pub mod router;
pub mod providers;
pub mod cli;
pub mod stream_parser;
pub mod intent;
pub mod config;
pub mod enhanced_router;
pub mod intelligent_router;
pub mod llm_cli;
pub mod llm_cli_extra;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Model capability types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelCapability {
    TextGeneration,
    CodeGeneration,
    ImageGeneration,
    Embedding,
    Classification,
    Translation,
    Summarization,
    QuestionAnswering,
    Custom(String),
}

/// Model endpoint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEndpoint {
    pub name: String,
    pub provider: String,  // ollama, openai, local
    pub model_id: String,
    pub endpoint_url: String,
    pub capabilities: Vec<ModelCapability>,
    pub max_tokens: Option<usize>,
    pub context_window: Option<usize>,
    pub is_active: bool,
    pub priority: i32,  // Higher priority endpoints are preferred
}

/// Inference request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub prompt: String,
    pub capability: ModelCapability,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
    pub system_prompt: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Inference response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResponse {
    pub text: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub model_used: String,
    pub tokens_used: Option<usize>,
    pub duration_ms: u64,
}

/// Model provider trait
pub trait ModelProvider: Send + Sync {
    fn name(&self) -> &str;
    fn is_available(&self) -> Result<bool>;
    fn infer(&self, request: &InferenceRequest) -> Result<InferenceResponse>;
    fn list_models(&self) -> Result<Vec<ModelEndpoint>>;
}