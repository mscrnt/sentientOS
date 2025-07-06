//! Answer generation using LLM with retrieved context

use anyhow::{Result, Context};
use crate::boot_llm;

/// Answer generator using LLM
pub struct Generator {
    model_name: String,
    use_boot_llm: bool,
}

impl Generator {
    /// Create new generator
    pub fn new(model_name: &str) -> Result<Self> {
        Ok(Self {
            model_name: model_name.to_string(),
            use_boot_llm: model_name == "phi" || model_name == "boot",
        })
    }
    
    /// Generate answer given query and context
    pub fn generate(&self, query: &str, context: &str) -> Result<String> {
        let prompt = self.build_prompt(query, context);
        
        if self.use_boot_llm || boot_llm::should_use_boot_llm() {
            // Use boot LLM (phi)
            boot_llm::get_boot_llm_response(&prompt)
        } else {
            // Use main LLM via AI client
            self.generate_with_ollama(&prompt)
        }
    }
    
    /// Build RAG prompt
    fn build_prompt(&self, query: &str, context: &str) -> String {
        format!(
            r#"You are a helpful AI assistant for SentientOS. Answer the user's question based on the provided context.

Context:
{}

Question: {}

Instructions:
1. Answer based ONLY on the provided context
2. If the context doesn't contain enough information, say so
3. Be concise and accurate
4. Cite source numbers when referencing specific information

Answer:"#,
            context, query
        )
    }
    
    /// Generate using Ollama
    fn generate_with_ollama(&self, prompt: &str) -> Result<String> {
        let ollama_url = std::env::var("OLLAMA_URL")
            .unwrap_or_else(|_| "http://192.168.69.197:11434".to_string());
        
        let client = reqwest::blocking::Client::new();
        
        #[derive(serde::Serialize)]
        struct GenerateRequest {
            model: String,
            prompt: String,
            stream: bool,
            options: GenerateOptions,
        }
        
        #[derive(serde::Serialize)]
        struct GenerateOptions {
            temperature: f32,
            max_tokens: i32,
        }
        
        #[derive(serde::Deserialize)]
        struct GenerateResponse {
            response: String,
        }
        
        let request = GenerateRequest {
            model: self.model_name.clone(),
            prompt: prompt.to_string(),
            stream: false,
            options: GenerateOptions {
                temperature: 0.3, // Lower temperature for factual answers
                max_tokens: 500,
            },
        };
        
        let response = client
            .post(format!("{}/api/generate", ollama_url))
            .json(&request)
            .send()
            .context("Failed to send generation request")?;
        
        if !response.status().is_success() {
            anyhow::bail!("Generation request failed: {}", response.status());
        }
        
        let gen_response: GenerateResponse = response.json()
            .context("Failed to parse generation response")?;
        
        Ok(gen_response.response)
    }
    
    /// Generate with structured output
    pub fn generate_structured(
        &self,
        query: &str,
        context: &str,
        format: OutputFormat,
    ) -> Result<String> {
        let mut prompt = self.build_prompt(query, context);
        
        match format {
            OutputFormat::Json => {
                prompt.push_str("\n\nProvide your answer in JSON format with keys: 'answer', 'confidence', 'sources'.");
            }
            OutputFormat::Markdown => {
                prompt.push_str("\n\nFormat your answer using Markdown with clear sections.");
            }
            OutputFormat::Plain => {
                // Default format
            }
        }
        
        self.generate(query, &prompt)
    }
}

/// Output format options
#[derive(Debug, Clone)]
pub enum OutputFormat {
    Plain,
    Json,
    Markdown,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_prompt_building() {
        let generator = Generator::new("test").unwrap();
        let prompt = generator.build_prompt(
            "What is HiveFix?",
            "[Source 1] HiveFix is a self-healing system..."
        );
        
        assert!(prompt.contains("What is HiveFix?"));
        assert!(prompt.contains("HiveFix is a self-healing system"));
        assert!(prompt.contains("Context:"));
    }
}