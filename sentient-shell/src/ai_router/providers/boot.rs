//! Boot model provider for offline/recovery AI

use crate::ai_router::*;
use crate::boot_llm::{BOOT_LLM, BootLLM};
use anyhow::Result;
use std::time::Instant;

pub struct BootModelProvider;

impl BootModelProvider {
    pub fn new() -> Self {
        Self
    }
}

impl ModelProvider for BootModelProvider {
    fn name(&self) -> &str {
        "boot"
    }

    fn is_available(&self) -> Result<bool> {
        Ok(BOOT_LLM.lock().unwrap().available)
    }

    fn infer(&self, request: &InferenceRequest) -> Result<InferenceResponse> {
        let start_time = Instant::now();
        
        let response_text = BOOT_LLM.lock().unwrap().evaluate(&request.prompt)?;
        
        Ok(InferenceResponse {
            text: Some(response_text),
            embedding: None,
            metadata: std::collections::HashMap::new(),
            model_used: "phi-boot".to_string(),
            tokens_used: Some(request.prompt.split_whitespace().count()),
            duration_ms: start_time.elapsed().as_millis() as u64,
        })
    }

    fn list_models(&self) -> Result<Vec<ModelEndpoint>> {
        Ok(vec![ModelEndpoint {
            name: "boot/phi".to_string(),
            provider: "boot".to_string(),
            model_id: "phi-2-q8_0".to_string(),
            endpoint_url: "kernel://llm/boot".to_string(),
            capabilities: vec![
                ModelCapability::TextGeneration,
                ModelCapability::QuestionAnswering,
                ModelCapability::Custom("validation".to_string()),
                ModelCapability::Custom("safety-check".to_string()),
            ],
            max_tokens: Some(512),
            context_window: Some(2048),
            is_active: self.is_available().unwrap_or(false),
            priority: 1, // Lower priority than online models
        }])
    }
}