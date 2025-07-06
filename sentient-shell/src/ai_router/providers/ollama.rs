// Ollama provider implementation
use crate::ai_router::*;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::time::Instant;

pub struct OllamaProvider {
    base_url: String,
    client: reqwest::blocking::Client,
}

impl OllamaProvider {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .unwrap(),
        }
    }
}

#[derive(Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    system: Option<String>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: i32,
}

#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
    total_duration: Option<u64>,
    prompt_eval_count: Option<i32>,
    eval_count: Option<i32>,
}

#[derive(Deserialize)]
struct OllamaModel {
    name: String,
    size: u64,
    digest: String,
}

impl ModelProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    fn is_available(&self) -> Result<bool> {
        let url = format!("{}/api/tags", self.base_url);
        match self.client.get(&url).send() {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    fn infer(&self, request: &InferenceRequest) -> Result<InferenceResponse> {
        let start_time = Instant::now();
        
        // For now, use a default model based on capability
        let model = match &request.capability {
            ModelCapability::CodeGeneration => "deepseek-v2:16b",
            ModelCapability::TextGeneration => "llama3.2:latest",
            ModelCapability::Embedding => "bge-m3:latest",
            _ => "llama3.2:latest",
        };

        let ollama_request = OllamaGenerateRequest {
            model: model.to_string(),
            prompt: request.prompt.clone(),
            system: request.system_prompt.clone(),
            stream: false,
            options: OllamaOptions {
                temperature: request.temperature.unwrap_or(0.7),
                num_predict: request.max_tokens.unwrap_or(500) as i32,
            },
        };

        let url = format!("{}/api/generate", self.base_url);
        let response = self.client
            .post(&url)
            .json(&ollama_request)
            .send()
            .context("Failed to send request to Ollama")?;

        if !response.status().is_success() {
            anyhow::bail!("Ollama request failed: {}", response.status());
        }

        let ollama_response: OllamaGenerateResponse = response.json()
            .context("Failed to parse Ollama response")?;

        Ok(InferenceResponse {
            text: Some(ollama_response.response),
            embedding: None,
            metadata: std::collections::HashMap::new(),
            model_used: model.to_string(),
            tokens_used: ollama_response.eval_count.map(|c| c as usize),
            duration_ms: start_time.elapsed().as_millis() as u64,
        })
    }

    fn list_models(&self) -> Result<Vec<ModelEndpoint>> {
        let url = format!("{}/api/tags", self.base_url);
        let response = self.client.get(&url).send()
            .context("Failed to connect to Ollama")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to list Ollama models");
        }

        #[derive(Deserialize)]
        struct TagsResponse {
            models: Vec<OllamaModel>,
        }

        let tags: TagsResponse = response.json()
            .context("Failed to parse models list")?;

        let endpoints: Vec<ModelEndpoint> = tags.models.into_iter().map(|model| {
            // Determine capabilities based on model name
            let capabilities = if model.name.contains("embed") || model.name.contains("bge") {
                vec![ModelCapability::Embedding]
            } else if model.name.contains("code") || model.name.contains("deepseek") {
                vec![ModelCapability::CodeGeneration, ModelCapability::TextGeneration]
            } else {
                vec![ModelCapability::TextGeneration, ModelCapability::QuestionAnswering]
            };

            ModelEndpoint {
                name: format!("ollama/{}", model.name),
                provider: "ollama".to_string(),
                model_id: model.name,
                endpoint_url: self.base_url.clone(),
                capabilities,
                max_tokens: Some(4096),
                context_window: Some(8192),
                is_active: true,
                priority: 10,
            }
        }).collect();

        Ok(endpoints)
    }
}