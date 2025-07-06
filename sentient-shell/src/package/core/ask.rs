use anyhow::Result;
use crate::ai::AiClient;
use super::{CorePackage, PackageCategory};

pub struct Ask;

impl CorePackage for Ask {
    fn name(&self) -> &'static str { "ask" }
    fn version(&self) -> &'static str { "1.0.0" }
    fn description(&self) -> &'static str { "Natural language interface to Ollama AI" }
    fn category(&self) -> PackageCategory { PackageCategory::Knowledge }
}

pub async fn run(ai_client: &mut AiClient, args: &[&str]) -> Result<String> {
    if args.is_empty() {
        return Ok("Usage: ask <question>\nExample: ask What is the meaning of life?".to_string());
    }
    
    let question = args.join(" ");
    
    // Add system context to make responses more helpful
    let prompt = format!(
        "You are an AI assistant in SentientOS. Please provide a helpful, concise response. \
         User question: {}",
        question
    );
    
    match ai_client.generate_text(&prompt) {
        Ok(response) => Ok(response),
        Err(e) => Err(anyhow::anyhow!("Failed to get AI response: {}", e)),
    }
}