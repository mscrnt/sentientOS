use crate::serial_println;
use alloc::string::String;
use alloc::vec::Vec;

const SHELL_BANNER: &str = r#"
╔═══════════════════════════════════════════╗
║      SentientShell v1.0 – AI-Native CLI   ║
║    The Intelligent Interface to SentientOS ║
╚═══════════════════════════════════════════╝
"#;

pub struct Shell {
    input_buffer: String,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            input_buffer: String::new(),
        }
    }

    pub fn start(&mut self) {
        serial_println!("{}", SHELL_BANNER);
        serial_println!("Type 'help' for available commands.\n");
        self.show_prompt();
    }

    pub fn handle_input(&mut self, ch: char) {
        match ch {
            '\r' | '\n' => {
                serial_println!();
                if !self.input_buffer.is_empty() {
                    self.execute_command(&self.input_buffer.clone());
                    self.input_buffer.clear();
                }
                self.show_prompt();
            }
            '\x08' | '\x7f' => {
                // Backspace
                if !self.input_buffer.is_empty() {
                    self.input_buffer.pop();
                    crate::serial::_print(format_args!("\x08 \x08"));
                }
            }
            '\x03' => {
                // Ctrl+C
                serial_println!("^C");
                self.input_buffer.clear();
                self.show_prompt();
            }
            _ => {
                if ch.is_ascii() && !ch.is_control() {
                    self.input_buffer.push(ch);
                    crate::serial::_print(format_args!("{}", ch));
                }
            }
        }
    }

    fn show_prompt(&self) {
        crate::serial::_print(format_args!("sentient> "));
    }

    fn execute_command(&mut self, input: &str) {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        match parts[0] {
            "help" => self.cmd_help(),
            "status" => self.cmd_status(),
            "ask" => {
                if parts.len() > 1 {
                    let prompt = parts[1..].join(" ");
                    self.cmd_ask(&prompt);
                } else {
                    serial_println!("Usage: ask <prompt>");
                }
            }
            "models" => self.cmd_models(),
            "image" => {
                if parts.len() > 1 {
                    let prompt = parts[1..].join(" ");
                    self.cmd_image(&prompt);
                } else {
                    serial_println!("Usage: image <prompt>");
                }
            }
            "exit" => self.cmd_exit(),
            _ => {
                serial_println!(
                    "Unknown command: {}. Type 'help' for available commands.",
                    parts[0]
                );
            }
        }
    }

    fn cmd_help(&self) {
        serial_println!("SentientShell Commands:");
        serial_println!("  help       - Show this help message");
        serial_println!("  status     - Show system status and connected AI models");
        serial_println!("  ask <prompt> - Query AI model with a prompt");
        serial_println!("  models     - List available AI models");
        serial_println!("  image <prompt> - Generate image from prompt");
        serial_println!("  exit       - Exit the shell");
        serial_println!();
        serial_println!("Examples:");
        serial_println!("  ask What is the meaning of life?");
        serial_println!("  image A beautiful sunset over mountains");
    }

    fn cmd_status(&self) {
        serial_println!("System Status:");
        serial_println!("  Shell Version: 1.0");
        serial_println!("  Kernel: SentientOS v0.1.0");
        serial_println!(
            "  Memory: {} MB free",
            crate::mm::get_free_memory() / (1024 * 1024)
        );

        // Check AI subsystem
        match crate::ai::try_get_ai_subsystem() {
            Ok(ai_lock) => {
                if let Some(ref ai) = *ai_lock.lock() {
                    serial_println!("  AI Subsystem: Active");
                    serial_println!("  Model: Loaded at 0x{:016x}", ai.model_info.memory_address);
                } else {
                    serial_println!("  AI Subsystem: Not initialized");
                }
            }
            Err(_) => {
                serial_println!("  AI Subsystem: Error accessing");
            }
        }
    }

    fn cmd_ask(&mut self, prompt: &str) {
        serial_println!("Thinking...");

        // In kernel mode, we use the AI subsystem directly
        match crate::ai::try_get_ai_subsystem() {
            Ok(ai_lock) => {
                if let Some(ref mut ai) = *ai_lock.lock() {
                    use crate::ai::{InferenceRequest, InferenceResponse};

                    let request = InferenceRequest::SystemAnalysis {
                        event: "shell_query",
                        metrics: crate::ai::SystemMetrics::current(),
                    };

                    match ai.request_inference(request) {
                        Ok(InferenceResponse::DiagnosticInfo(info)) => {
                            serial_println!("\nResponse: {}", info);
                        }
                        Ok(_) => {
                            serial_println!("\nResponse: AI returned unexpected response type");
                        }
                        Err(e) => {
                            serial_println!("\nError: Failed to get AI response - {}", e);
                        }
                    }
                } else {
                    serial_println!("\nError: AI subsystem not initialized");
                }
            }
            Err(e) => {
                serial_println!("\nError: Failed to access AI subsystem - {}", e);
            }
        }

        // Fallback response for demo
        serial_println!(
            "\n[Demo Response] Query: '{}' - The AI model integration is being established.",
            prompt
        );
    }

    fn cmd_models(&self) {
        serial_println!("Available Models:");
        serial_println!("\nKernel AI Model:");

        match crate::ai::try_get_ai_subsystem() {
            Ok(ai_lock) => {
                if let Some(ref ai) = *ai_lock.lock() {
                    serial_println!("  - Embedded GGUF Model");
                    serial_println!("    Size: {} bytes", ai.model_info.size_bytes);
                    serial_println!("    Location: 0x{:016x}", ai.model_info.memory_address);
                } else {
                    serial_println!("  - No model loaded");
                }
            }
            Err(_) => {
                serial_println!("  - Error accessing AI subsystem");
            }
        }

        serial_println!("\nRemote Models (when network available):");
        serial_println!("  - deepseek-v2 (via Ollama)");
        serial_println!("  - stable-diffusion-xl (via SD WebUI)");
    }

    fn cmd_image(&mut self, prompt: &str) {
        serial_println!("Image generation in kernel mode is not yet implemented.");
        serial_println!("Prompt received: '{}'", prompt);
        serial_println!("\n[Demo] Image generated: SHA256 = a1b2c3d4e5f6...");
    }

    fn cmd_exit(&self) {
        serial_println!("Exiting shell...");
        serial_println!("System halt requested.");

        // Signal kernel to halt
        crate::sys::shutdown(uefi::Status::SUCCESS);
    }
}
