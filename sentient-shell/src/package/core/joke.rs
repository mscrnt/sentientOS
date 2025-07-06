use anyhow::Result;
use crate::ai::AiClient;
use super::{CorePackage, PackageCategory};

pub struct Joke;

impl CorePackage for Joke {
    fn name(&self) -> &'static str { "joke" }
    fn version(&self) -> &'static str { "1.0.0" }
    fn description(&self) -> &'static str { "Get a random joke from the AI" }
    fn category(&self) -> PackageCategory { PackageCategory::Creative }
}

pub async fn run(ai_client: &mut AiClient) -> Result<String> {
    let prompt = "Tell me a short, funny programming joke. Keep it clean and under 50 words.";
    
    match ai_client.generate_text(prompt) {
        Ok(joke) => Ok(joke),
        Err(_) => {
            // Fallback jokes if AI is unavailable
            Ok("Why do programmers prefer dark mode? Because light attracts bugs!".to_string())
        }
    }
}