//! Test boot LLM functionality

use sentient_shell::boot_llm::{BOOT_LLM, should_use_boot_llm, get_boot_llm_response};

fn main() {
    env_logger::init();
    
    println!("=== Boot LLM Test ===\n");
    
    // Force boot mode
    std::env::set_var("SENTIENT_BOOT_MODE", "1");
    
    println!("Should use boot LLM: {}", should_use_boot_llm());
    
    // Test various prompts
    let test_prompts = vec![
        "help",
        "status",
        "validate rm -rf /tmp",
        "validate echo hello",
        "safe-mode",
        "What is 2+2?",
    ];
    
    for prompt in test_prompts {
        println!("\nPrompt: {}", prompt);
        match get_boot_llm_response(prompt) {
            Ok(response) => {
                println!("Response: {}", response);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
    
    // Test with prefix commands
    println!("\n--- Testing Prefix Commands ---");
    
    let prefix_prompts = vec![
        "!@ service list",
        "!~ python test.py",
    ];
    
    for prompt in prefix_prompts {
        println!("\nPrefix command: {}", prompt);
        match get_boot_llm_response(prompt) {
            Ok(response) => {
                println!("Response: {}", response);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}