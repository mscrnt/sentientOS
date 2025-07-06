//! Rerank retrieved chunks using Qwen3-Reranker model

use super::RetrievedChunk;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};

/// Document reranker using cross-encoder model
pub struct Reranker {
    model_name: String,
    client: reqwest::blocking::Client,
    ollama_url: String,
}

impl Reranker {
    /// Create new reranker
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
    
    /// Rerank chunks based on relevance to query
    pub fn rerank(
        &self,
        query: &str,
        mut chunks: Vec<RetrievedChunk>,
    ) -> Result<Vec<RetrievedChunk>> {
        // If reranker model is available, use it
        if self.is_model_available() {
            self.rerank_with_model(query, chunks)
        } else {
            // Fallback: use simple keyword matching
            self.rerank_fallback(query, &mut chunks);
            Ok(chunks)
        }
    }
    
    /// Check if reranker model is available
    fn is_model_available(&self) -> bool {
        // Check if model exists in Ollama
        // For now, assume it's not available and use fallback
        false
    }
    
    /// Rerank using the model
    fn rerank_with_model(
        &self,
        query: &str,
        chunks: Vec<RetrievedChunk>,
    ) -> Result<Vec<RetrievedChunk>> {
        #[derive(Serialize)]
        struct RerankRequest {
            model: String,
            query: String,
            documents: Vec<String>,
        }
        
        #[derive(Deserialize)]
        struct RerankResponse {
            scores: Vec<f32>,
        }
        
        let documents: Vec<String> = chunks.iter()
            .map(|c| c.content.clone())
            .collect();
        
        let request = RerankRequest {
            model: self.model_name.clone(),
            query: query.to_string(),
            documents,
        };
        
        // In real implementation, call reranker API
        // For now, return original order
        Ok(chunks)
    }
    
    /// Simple fallback reranking based on keyword overlap
    fn rerank_fallback(&self, query: &str, chunks: &mut [RetrievedChunk]) {
        let query_words: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        
        // Score each chunk based on keyword overlap
        for chunk in chunks.iter_mut() {
            let chunk_words: Vec<String> = chunk.content
                .to_lowercase()
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            
            let overlap_count = query_words.iter()
                .filter(|qw| chunk_words.contains(qw))
                .count();
            
            // Boost score based on keyword overlap
            let boost = overlap_count as f32 / query_words.len() as f32;
            chunk.score = chunk.score * (1.0 + boost);
        }
        
        // Sort by new scores
        chunks.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    
    /// Rerank with diversity to avoid redundant results
    pub fn rerank_with_diversity(
        &self,
        query: &str,
        chunks: Vec<RetrievedChunk>,
        diversity_threshold: f32,
    ) -> Result<Vec<RetrievedChunk>> {
        let mut reranked = self.rerank(query, chunks)?;
        let mut diverse_results = Vec::new();
        
        for chunk in reranked {
            // Check if this chunk is too similar to already selected ones
            let is_diverse = diverse_results.iter().all(|selected: &RetrievedChunk| {
                self.calculate_similarity(&chunk.content, &selected.content) < diversity_threshold
            });
            
            if is_diverse {
                diverse_results.push(chunk);
            }
        }
        
        Ok(diverse_results)
    }
    
    /// Calculate simple text similarity
    fn calculate_similarity(&self, text1: &str, text2: &str) -> f32 {
        let words1: std::collections::HashSet<&str> = text1.split_whitespace().collect();
        let words2: std::collections::HashSet<&str> = text2.split_whitespace().collect();
        
        let intersection = words1.intersection(&words2).count() as f32;
        let union = words1.union(&words2).count() as f32;
        
        if union > 0.0 {
            intersection / union
        } else {
            0.0
        }
    }
}