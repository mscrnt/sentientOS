//! Text embedding using Qwen3-Embedding model

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};

/// Text embedder using Qwen3 or other embedding models
pub struct Embedder {
    model_name: String,
    client: reqwest::blocking::Client,
    ollama_url: String,
}

impl Embedder {
    /// Create new embedder with specified model
    pub fn new(model_name: &str) -> Result<Self> {
        let ollama_url = std::env::var("OLLAMA_URL")
            .unwrap_or_else(|_| "http://192.168.69.197:11434".to_string());
        
        Ok(Self {
            model_name: model_name.to_string(),
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()?,
            ollama_url,
        })
    }
    
    /// Embed a text into a vector
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // For now, use Ollama's embedding endpoint
        // In production, could use local ONNX model
        
        let url = format!("{}/api/embeddings", self.ollama_url);
        
        #[derive(Serialize)]
        struct EmbedRequest {
            model: String,
            prompt: String,
        }
        
        #[derive(Deserialize)]
        struct EmbedResponse {
            embedding: Vec<f32>,
        }
        
        let request = EmbedRequest {
            model: self.model_name.clone(),
            prompt: text.to_string(),
        };
        
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .context("Failed to send embedding request")?;
        
        if !response.status().is_success() {
            anyhow::bail!("Embedding request failed: {}", response.status());
        }
        
        let embed_response: EmbedResponse = response.json()
            .context("Failed to parse embedding response")?;
        
        Ok(embed_response.embedding)
    }
    
    /// Embed in fallback mode (simple hash-based)
    pub fn embed_fallback(&self, text: &str) -> Vec<f32> {
        // Simple fallback: use character frequencies as features
        let mut embedding = vec![0.0f32; 768]; // Standard embedding size
        
        // Simple but deterministic: hash characters to positions
        for (i, ch) in text.chars().enumerate() {
            let pos = (ch as usize + i) % embedding.len();
            embedding[pos] += 1.0;
        }
        
        // Normalize
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut embedding {
                *x /= norm;
            }
        }
        
        embedding
    }
    
    /// Batch embed multiple texts
    pub fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        texts.iter()
            .map(|text| self.embed(text))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fallback_embedding() {
        let embedder = Embedder::new("test").unwrap();
        
        let embed1 = embedder.embed_fallback("hello world");
        let embed2 = embedder.embed_fallback("hello world");
        let embed3 = embedder.embed_fallback("goodbye world");
        
        // Same text should produce same embedding
        assert_eq!(embed1, embed2);
        
        // Different text should produce different embedding
        assert_ne!(embed1, embed3);
        
        // Should be normalized
        let norm: f32 = embed1.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.001);
    }
}