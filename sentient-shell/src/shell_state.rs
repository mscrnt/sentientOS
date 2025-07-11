use crate::ai;
use crate::commands;
use crate::commands_functions;
#[cfg(feature = "local-inference")]
use crate::inference;
use crate::package;
use anyhow::Result;

pub struct ShellState {
    pub ai_client: ai::AiClient,
    #[cfg(feature = "local-inference")]
    pub local_inference: Option<inference::LocalInference>,
    pub package_registry: package::PackageRegistry,
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

        let mut package_registry = package::PackageRegistry::new();
        if let Err(e) = package_registry.init() {
            log::warn!("Failed to initialize package registry: {}", e);
        }

        Self {
            ai_client,
            #[cfg(feature = "local-inference")]
            local_inference,
            package_registry,
        }
    }

    pub fn execute_command(&mut self, input: &str) -> Result<bool> {
        // Check for prefix commands
        if input.starts_with("!@") || input.starts_with("!#") || 
           input.starts_with("!$") || input.starts_with("!&") || input.starts_with("!~") {
            return match crate::validated_exec::execute_with_prefix(input) {
                Ok(_) => Ok(false),
                Err(e) => {
                    eprintln!("Validation error: {}", e);
                    Ok(false)
                }
            };
        }
        
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(false);
        }

        match parts[0] {
            "help" => {
                commands_functions::show_help();
                Ok(false)
            }
            "status" => {
                commands_functions::show_status(&self.ai_client)?;
                Ok(false)
            }
            "ask" => {
                if parts.len() < 2 {
                    println!("Usage: ask <prompt>");
                    return Ok(false);
                }
                let prompt = parts[1..].join(" ");
                
                // Check if we should use boot LLM
                if crate::boot_llm::should_use_boot_llm() {
                    match crate::boot_llm::get_boot_llm_response(&prompt) {
                        Ok(response) => println!("{}", response),
                        Err(e) => eprintln!("Boot LLM error: {}", e),
                    }
                } else {
                    commands_functions::ask_ai(&mut self.ai_client, &prompt)?;
                }
                Ok(false)
            }
            "models" => {
                commands_functions::list_models(&self.ai_client)?;
                Ok(false)
            }
            "image" => {
                if parts.len() < 2 {
                    println!("Usage: image <prompt>");
                    return Ok(false);
                }
                let prompt = parts[1..].join(" ");
                commands_functions::generate_image(&mut self.ai_client, &prompt)?;
                Ok(false)
            }
            "pkg" => {
                if parts.len() < 2 {
                    commands_functions::pkg_usage();
                    return Ok(false);
                }
                commands_functions::handle_pkg_command(&mut self.package_registry, &mut self.ai_client, &parts[1..])?;
                Ok(false)
            }
            "service" => {
                if parts.len() < 2 {
                    println!("{}", crate::service::api::handle_command(&[])?);
                    return Ok(false);
                }
                let result = crate::service::api::handle_command(&parts[1..])?;
                println!("{}", result);
                Ok(false)
            }
            "ai" => {
                if parts.len() < 2 {
                    println!("{}", crate::ai_router::cli::handle_command(&[])?);
                    return Ok(false);
                }
                let result = crate::ai_router::cli::handle_command(&parts[1..])?;
                println!("{}", result);
                Ok(false)
            }
            "rag" => {
                if parts.len() < 2 {
                    println!("{}", crate::rag::cli::handle_rag_command(&[])?);
                    return Ok(false);
                }
                let result = crate::rag::cli::handle_rag_command(&parts[1..])?;
                println!("{}", result);
                Ok(false)
            }
            "tool" => {
                crate::shell::tools::handle_tool_command(&parts[1..])?;
                Ok(false)
            }
            "llm" => {
                match crate::ai_router::llm_cli::handle_llm_command(&parts[1..]) {
                    Ok(result) => println!("{}", result),
                    Err(e) => eprintln!("Error: {}", e),
                }
                Ok(false)
            }
            "rag_tool" => {
                let runtime = tokio::runtime::Runtime::new()?;
                runtime.block_on(crate::commands::rag_tool::handle_command(&parts[1..]))?;
                Ok(false)
            }
            "rl" => {
                let runtime = tokio::runtime::Runtime::new()?;
                runtime.block_on(crate::commands::rl_trace::handle_command(&parts[1..]))?;
                Ok(false)
            }
            "sentient" => {
                if parts.len() < 2 {
                    println!("Usage: sentient goal <goal description>");
                    return Ok(false);
                }
                if parts[1] == "goal" && parts.len() > 2 {
                    let goal = parts[2..].join(" ");
                    let args = crate::commands::sentient_goal::SentientGoalArgs {
                        goal,
                        verbose: false,
                        dry_run: false,
                        max_steps: 50,
                        save_trace: false,
                    };
                    let runtime = tokio::runtime::Runtime::new()?;
                    runtime.block_on(crate::commands::sentient_goal::execute(args))?;
                } else {
                    println!("Unknown sentient command. Use: sentient goal <goal>");
                }
                Ok(false)
            }
            "exit" => Ok(true),
            _ => {
                // Check if it's an installed package command
                if self.package_registry.is_installed(parts[0]) {
                    commands_functions::run_package(&mut self.ai_client, parts[0], &parts[1..])?;
                    Ok(false)
                } else {
                    println!(
                        "Unknown command: {}. Type 'help' for available commands.",
                        parts[0]
                    );
                    Ok(false)
                }
            }
        }
    }
}
