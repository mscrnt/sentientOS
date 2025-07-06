// CLI interface for AI router
use super::*;
use super::registry::get_model_registry;
use super::router::AIRouter;
use super::providers::ollama::OllamaProvider;
use anyhow::Result;
use std::sync::Arc;

pub fn handle_command(args: &[&str]) -> Result<String> {
    if args.is_empty() {
        return Ok(help());
    }

    match args[0] {
        "list" => list_models(),
        "status" => show_status(),
        "query" => {
            if args.len() < 2 {
                return Ok("Usage: ai query <prompt>".to_string());
            }
            query_model(&args[1..].join(" "))
        }
        "init" => init_providers(),
        "help" => Ok(help()),
        _ => Ok(format!("Unknown command: {}. Try 'ai help'", args[0])),
    }
}

fn help() -> String {
    r#"AI Router Commands:
  list              List available models and endpoints
  status            Show AI router status and health
  query <prompt>    Send a query to the AI router
  init              Initialize AI providers
  help              Show this help message

Examples:
  ai list
  ai query "Explain quantum computing"
  ai status"#.to_string()
}

fn list_models() -> Result<String> {
    let registry = get_model_registry();
    let endpoints = registry.list_endpoints();
    
    if endpoints.is_empty() {
        return Ok("No models registered. Run 'ai init' to discover models.".to_string());
    }
    
    let mut output = String::from("Available AI Models:\n");
    output.push_str("─────────────────────────────────────────────────────\n");
    output.push_str("PROVIDER    MODEL                   CAPABILITIES         STATUS\n");
    output.push_str("─────────────────────────────────────────────────────\n");
    
    for endpoint in endpoints {
        let status = if endpoint.is_active { "🟢 active" } else { "🔴 inactive" };
        let capabilities: Vec<&str> = endpoint.capabilities.iter()
            .map(|c| match c {
                ModelCapability::TextGeneration => "text",
                ModelCapability::CodeGeneration => "code",
                ModelCapability::ImageGeneration => "image",
                ModelCapability::Embedding => "embed",
                ModelCapability::Classification => "classify",
                ModelCapability::Translation => "translate",
                ModelCapability::Summarization => "summary",
                ModelCapability::QuestionAnswering => "qa",
                ModelCapability::Custom(s) => s.as_str(),
            })
            .collect();
        
        output.push_str(&format!(
            "{:<11} {:<23} {:<20} {}\n",
            endpoint.provider,
            endpoint.model_id,
            capabilities.join(", "),
            status
        ));
    }
    
    Ok(output)
}

fn show_status() -> Result<String> {
    let registry = get_model_registry();
    let health = registry.health_check();
    
    let mut output = String::from("AI Router Status:\n");
    output.push_str("─────────────────────────────────────────────────────\n");
    
    for (provider, available) in health {
        let status = if available { "🟢 available" } else { "🔴 unavailable" };
        output.push_str(&format!("{:<20} {}\n", provider, status));
    }
    
    let endpoints = registry.list_endpoints();
    let active_count = endpoints.iter().filter(|e| e.is_active).count();
    
    output.push_str("\nSummary:\n");
    output.push_str(&format!("Total endpoints: {}\n", endpoints.len()));
    output.push_str(&format!("Active endpoints: {}\n", active_count));
    
    Ok(output)
}

fn query_model(prompt: &str) -> Result<String> {
    let request = InferenceRequest {
        prompt: prompt.to_string(),
        capability: ModelCapability::TextGeneration,
        max_tokens: Some(500),
        temperature: Some(0.7),
        system_prompt: Some("You are a helpful AI assistant in SentientOS.".to_string()),
        metadata: HashMap::new(),
    };
    
    match AIRouter::route_request(&request) {
        Ok(response) => {
            let mut output = String::new();
            if let Some(text) = response.text {
                output.push_str(&text);
                output.push_str("\n\n");
            }
            output.push_str(&format!("Model: {}\n", response.model_used));
            output.push_str(&format!("Time: {}ms\n", response.duration_ms));
            if let Some(tokens) = response.tokens_used {
                output.push_str(&format!("Tokens: {}\n", tokens));
            }
            Ok(output)
        }
        Err(e) => Ok(format!("Error: {}", e)),
    }
}

fn init_providers() -> Result<String> {
    let registry = get_model_registry();
    
    // Initialize Ollama provider
    let ollama_url = std::env::var("OLLAMA_URL")
        .unwrap_or_else(|_| "http://192.168.69.197:11434".to_string());
    
    let ollama = Arc::new(OllamaProvider::new(ollama_url.clone()));
    
    match registry.register_provider(ollama) {
        Ok(_) => Ok(format!("Initialized Ollama provider at {}", ollama_url)),
        Err(e) => Ok(format!("Failed to initialize Ollama: {}", e)),
    }
}