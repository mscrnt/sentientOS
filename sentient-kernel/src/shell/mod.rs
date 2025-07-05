use crate::serial_println;

pub const SHELL_BANNER: &str = r#"
╔═══════════════════════════════════════════╗
║      SentientShell v1.0 – AI-Native CLI   ║
║    The Intelligent Interface to SentientOS ║
╚═══════════════════════════════════════════╝
"#;

// Simple command buffer without allocation
static mut COMMAND_BUFFER: [u8; 256] = [0; 256];
static mut COMMAND_LEN: usize = 0;

pub fn handle_input_simple(ch: char) {
    unsafe {
        match ch {
            '\r' | '\n' => {
                serial_println!();
                if COMMAND_LEN > 0 {
                    execute_command_simple();
                    COMMAND_LEN = 0;
                }
                crate::serial::_print(format_args!("sentient> "));
            }
            '\x08' | '\x7f' => {
                // Backspace
                if COMMAND_LEN > 0 {
                    COMMAND_LEN -= 1;
                    crate::serial::_print(format_args!("\x08 \x08"));
                }
            }
            '\x03' => {
                // Ctrl+C
                serial_println!("^C");
                COMMAND_LEN = 0;
                crate::serial::_print(format_args!("sentient> "));
            }
            _ => {
                if ch.is_ascii() && !ch.is_control() && COMMAND_LEN < 255 {
                    COMMAND_BUFFER[COMMAND_LEN] = ch as u8;
                    COMMAND_LEN += 1;
                    crate::serial::_print(format_args!("{}", ch));
                }
            }
        }
    }
}

fn execute_command_simple() {
    unsafe {
        let cmd = core::str::from_utf8_unchecked(&COMMAND_BUFFER[..COMMAND_LEN]);

        // Simple string comparison without allocation
        if cmd_equals(cmd, "help") {
            cmd_help();
        } else if cmd_equals(cmd, "status") {
            cmd_status();
        } else if cmd_starts_with(cmd, "ask ") {
            let prompt = &cmd[4..];
            cmd_ask(prompt);
        } else if cmd_equals(cmd, "models") {
            cmd_models();
        } else if cmd_starts_with(cmd, "image ") {
            let prompt = &cmd[6..];
            cmd_image(prompt);
        } else if cmd_equals(cmd, "exit") {
            cmd_exit();
        } else {
            serial_println!(
                "Unknown command: {}. Type 'help' for available commands.",
                cmd
            );
        }
    }
}

fn cmd_equals(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    for (ca, cb) in a.chars().zip(b.chars()) {
        if ca != cb {
            return false;
        }
    }
    true
}

fn cmd_starts_with(s: &str, prefix: &str) -> bool {
    if s.len() < prefix.len() {
        return false;
    }
    for (cs, cp) in s.chars().zip(prefix.chars()) {
        if cs != cp {
            return false;
        }
    }
    true
}

fn cmd_help() {
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

fn cmd_status() {
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
            if let Some(_) = *ai_lock.lock() {
                serial_println!("  AI Subsystem: Active");
                serial_println!("  Model: Embedded AI Model");
            } else {
                serial_println!("  AI Subsystem: Not initialized");
            }
        }
        Err(_) => {
            serial_println!("  AI Subsystem: Error accessing");
        }
    }
}

fn cmd_ask(prompt: &str) {
    serial_println!("Thinking...");

    // In kernel mode, we use the AI subsystem directly
    match crate::ai::try_get_ai_subsystem() {
        Ok(ai_lock) => {
            if let Some(ref mut ai) = *ai_lock.lock() {
                use crate::ai::{InferenceRequest, InferenceResponse};

                let request = InferenceRequest::SystemAnalysis {
                    event: "shell_query",
                    metrics: crate::ai::SystemMetrics {
                        uptime_ms: 0,
                        free_memory: crate::mm::get_free_memory(),
                        task_count: 1,
                        interrupt_count: 0,
                    },
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

fn cmd_models() {
    serial_println!("Available Models:");
    serial_println!("\nKernel AI Model:");

    match crate::ai::try_get_ai_subsystem() {
        Ok(ai_lock) => {
            if let Some(_) = *ai_lock.lock() {
                serial_println!("  - Embedded GGUF Model");
                serial_println!("    Format: GGUF v3");
                serial_println!("    Type: Quantized Neural Network");
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

fn cmd_image(prompt: &str) {
    serial_println!("Image generation in kernel mode is not yet implemented.");
    serial_println!("Prompt received: '{}'", prompt);
    serial_println!("\n[Demo] Image generated: SHA256 = a1b2c3d4e5f6...");
}

fn cmd_exit() {
    serial_println!("Exiting shell...");
    serial_println!("System halt requested.");

    // Signal kernel to halt
    crate::sys::shutdown(uefi::Status::SUCCESS);
}
