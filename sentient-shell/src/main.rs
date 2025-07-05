mod serial;
mod commands;
mod ai;
mod inference;

use anyhow::Result;
use std::io::{self, Write};

const SHELL_VERSION: &str = "1.0";
const BANNER: &str = r#"
╔═══════════════════════════════════════════╗
║      SentientShell v1.0 – AI-Native CLI   ║
║    The Intelligent Interface to SentientOS ║
╚═══════════════════════════════════════════╝
"#;

fn main() -> Result<()> {
    env_logger::init();
    
    // Check if we're running in serial mode (for kernel/QEMU)
    let serial_mode = std::env::var("SENTIENT_SERIAL").is_ok();
    
    if serial_mode {
        log::info!("Running in serial mode");
        serial::run_serial_shell()
    } else {
        log::info!("Running in terminal mode");
        run_terminal_shell()
    }
}

fn run_terminal_shell() -> Result<()> {
    println!("{}", BANNER);
    println!("Type 'help' for available commands.\n");
    
    let mut shell = ShellState::new();
    
    loop {
        print!("sentient> ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        
        match shell.execute_command(input) {
            Ok(should_exit) => {
                if should_exit {
                    println!("Goodbye!");
                    break;
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
    
    Ok(())
}

pub struct ShellState {
    ai_client: ai::AiClient,
    #[cfg(feature = "local-inference")]
    local_inference: Option<inference::LocalInference>,
}

impl ShellState {
    pub fn new() -> Self {
        // Allow overriding URLs via environment variables for testing
        let ollama_url = std::env::var("OLLAMA_URL")
            .unwrap_or_else(|_| "http://192.168.69.197:11434".to_string());
        let sd_url = std::env::var("SD_URL")
            .unwrap_or_else(|_| "http://192.168.69.197:7860".to_string());
            
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
                println!("Unknown command: {}. Type 'help' for available commands.", parts[0]);
                Ok(false)
            }
        }
    }
}