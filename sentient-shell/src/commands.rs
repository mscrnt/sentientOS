use anyhow::Result;
use crate::ai::AiClient;

pub fn show_help() {
    println!("SentientShell Commands:");
    println!("  help       - Show this help message");
    println!("  status     - Show system status and connected AI models");
    println!("  ask <prompt> - Query AI model with a prompt");
    println!("  models     - List available AI models");
    println!("  image <prompt> - Generate image from prompt");
    println!("  exit       - Exit the shell");
    println!();
    println!("Examples:");
    println!("  ask What is the meaning of life?");
    println!("  image A beautiful sunset over mountains");
}

pub fn show_status(ai_client: &AiClient) -> Result<()> {
    println!("System Status:");
    println!("  Shell Version: {}", crate::SHELL_VERSION);
    println!("  Ollama Server: {}", ai_client.ollama_url());
    println!("  SD Server: {}", ai_client.sd_url());
    
    // Check connectivity
    print!("  Ollama Status: ");
    match ai_client.check_ollama_connection() {
        Ok(true) => println!("Connected ✓"),
        Ok(false) => println!("Not reachable ✗"),
        Err(e) => println!("Error: {}", e),
    }
    
    print!("  SD Status: ");
    match ai_client.check_sd_connection() {
        Ok(true) => println!("Connected ✓"),
        Ok(false) => println!("Not reachable ✗"),
        Err(e) => println!("Error: {}", e),
    }
    
    // Show preferred model
    match ai_client.get_preferred_model() {
        Ok(Some(model)) => println!("  Preferred Model: {}", model),
        Ok(None) => println!("  Preferred Model: None available"),
        Err(e) => println!("  Preferred Model: Error - {}", e),
    }
    
    Ok(())
}

pub fn ask_ai(ai_client: &mut AiClient, prompt: &str) -> Result<()> {
    println!("Thinking...");
    
    match ai_client.generate_text(prompt) {
        Ok(response) => {
            println!("\nResponse:");
            println!("{}", response);
        }
        Err(e) => {
            println!("Failed to get AI response: {}", e);
            
            // Try local inference if available
            #[cfg(feature = "local-inference")]
            {
                println!("Attempting local inference fallback...");
                // Note: In production, we'd pass the local_inference instance from ShellState
                println!("Local inference is available but not connected in this context");
            }
            
            // If all else fails, provide a demo response
            println!("\n[Demo Mode] Query: '{}'", prompt);
            println!("AI services are not available. In a production environment,");
            println!("this would connect to Ollama at http://192.168.69.197:11434");
        }
    }
    
    Ok(())
}

pub fn list_models(ai_client: &AiClient) -> Result<()> {
    println!("Available Models:");
    
    // List Ollama models
    println!("\nOllama Models:");
    match ai_client.list_ollama_models() {
        Ok(models) => {
            if models.is_empty() {
                println!("  No models found");
            } else {
                for model in models {
                    println!("  - {}", model);
                }
            }
        }
        Err(e) => {
            println!("  Error listing models: {}", e);
        }
    }
    
    // List SD models
    println!("\nStable Diffusion Models:");
    match ai_client.list_sd_models() {
        Ok(models) => {
            if models.is_empty() {
                println!("  No models found");
            } else {
                for model in models {
                    println!("  - {}", model);
                }
            }
        }
        Err(e) => {
            println!("  Error listing models: {}", e);
        }
    }
    
    Ok(())
}

pub fn generate_image(ai_client: &mut AiClient, prompt: &str) -> Result<()> {
    println!("Generating image...");
    
    match ai_client.generate_image(prompt) {
        Ok(image_info) => {
            println!("\nImage generated successfully!");
            println!("  Prompt: {}", prompt);
            println!("  Hash: {}", image_info.hash);
            println!("  Size: {} bytes", image_info.size);
            
            // In a real implementation, we might save the image or display a preview
            // For now, we just show the info
        }
        Err(e) => {
            println!("Failed to generate image: {}", e);
            
            // Provide demo response when SD is not available
            println!("\n[Demo Mode] Image prompt: '{}'", prompt);
            println!("SD WebUI is not available. In a production environment,");
            println!("this would connect to Stable Diffusion at http://192.168.69.197:7860");
            println!("Demo hash: a1b2c3d4e5f6789...");
        }
    }
    
    Ok(())
}