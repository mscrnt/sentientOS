use crate::ai;
use crate::commands;
#[cfg(feature = "local-inference")]
use crate::inference;
use anyhow::Result;

pub struct ShellState {
    pub ai_client: ai::AiClient,
    #[cfg(feature = "local-inference")]
    pub local_inference: Option<inference::LocalInference>,
}

impl ShellState {
    pub fn new() -> Self {
        // Allow overriding URLs via environment variables for testing
        let ollama_url = std::env::var("OLLAMA_URL")
            .unwrap_or_else(|_| "http://192.168.69.197:11434".to_string());
        let sd_url =
            std::env::var("SD_URL").unwrap_or_else(|_| "http://192.168.69.197:7860".to_string());

        let ai_client = ai::AiClient::new(ollama_url, sd_url);

        #[cfg(feature = "local-inference")]
        let local_inference = inference::LocalInference::new().ok();

        Self {
            ai_client,
            #[cfg(feature = "local-inference")]
            local_inference,
        }
    }

    pub fn execute_command(&mut self, input: &str) -> Result<bool> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(false);
        }

        match parts[0] {
            "help" => {
                commands::show_help();
                Ok(false)
            }
            "status" => {
                commands::show_status(&self.ai_client)?;
                Ok(false)
            }
            "ask" => {
                if parts.len() < 2 {
                    println!("Usage: ask <prompt>");
                    return Ok(false);
                }
                let prompt = parts[1..].join(" ");
                commands::ask_ai(&mut self.ai_client, &prompt)?;
                Ok(false)
            }
            "models" => {
                commands::list_models(&self.ai_client)?;
                Ok(false)
            }
            "image" => {
                if parts.len() < 2 {
                    println!("Usage: image <prompt>");
                    return Ok(false);
                }
                let prompt = parts[1..].join(" ");
                commands::generate_image(&mut self.ai_client, &prompt)?;
                Ok(false)
            }
            "exit" => Ok(true),
            _ => {
                println!(
                    "Unknown command: {}. Type 'help' for available commands.",
                    parts[0]
                );
                Ok(false)
            }
        }
    }
}
