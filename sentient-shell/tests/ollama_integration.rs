use sentient_shell::ai::{AiClient, OllamaRequest, OllamaResponse};

#[test]
#[ignore] // Run with: cargo test -- --ignored
fn test_ollama_connection() {
    // Test basic connectivity to Ollama
    let client = AiClient::new();
    
    // Try to get model list
    let response = reqwest::blocking::get("http://192.168.69.197:11434/api/tags")
        .expect("Failed to connect to Ollama");
    
    assert!(response.status().is_success(), "Ollama server not responding");
    
    let body = response.text().unwrap();
    println!("Available models: {}", body);
}

#[test]
#[ignore]
fn test_ollama_generate() {
    let client = AiClient::new();
    
    // Create a simple test prompt
    let test_prompt = "Say 'Hello from SentientOS test suite!' and nothing else.";
    
    let request = OllamaRequest {
        model: "deepseek-v2".to_string(),
        prompt: test_prompt.to_string(),
        stream: false,
        ..Default::default()
    };
    
    let response = client.query_ollama(&request);
    assert!(response.is_ok(), "Failed to query Ollama: {:?}", response.err());
    
    let response_text = response.unwrap();
    println!("Ollama response: {}", response_text);
    assert!(response_text.len() > 0, "Empty response from Ollama");
}

#[test]
#[ignore]
fn test_ollama_system_analysis() {
    let client = AiClient::new();
    
    // Test a system analysis prompt
    let system_prompt = r#"You are an AI assistant integrated into SentientOS kernel.
    Analyze the following system state and provide a brief diagnostic:
    - Memory: 4GB total, 2GB free
    - CPU: 50% usage
    - Processes: 42 running
    What is your assessment?"#;
    
    let request = OllamaRequest {
        model: "deepseek-v2".to_string(),
        prompt: system_prompt.to_string(),
        stream: false,
        ..Default::default()
    };
    
    let response = client.query_ollama(&request);
    assert!(response.is_ok(), "Failed to analyze system");
    
    let analysis = response.unwrap();
    println!("System analysis:\n{}", analysis);
    
    // Check that we got a meaningful response
    assert!(analysis.contains("memory") || analysis.contains("Memory") || 
            analysis.contains("CPU") || analysis.contains("system"),
            "Response doesn't seem to address system state");
}

#[test]
#[ignore]
fn test_ollama_streaming() {
    use std::io::Write;
    
    let client = AiClient::new();
    
    let request = OllamaRequest {
        model: "deepseek-v2".to_string(),
        prompt: "Count from 1 to 5, with each number on a new line.".to_string(),
        stream: true,
        ..Default::default()
    };
    
    print!("Streaming response: ");
    std::io::stdout().flush().unwrap();
    
    // For streaming, we'd need to modify the client to handle stream responses
    // For now, test non-streaming version
    let request_non_stream = OllamaRequest {
        stream: false,
        ..request
    };
    
    let response = client.query_ollama(&request_non_stream);
    assert!(response.is_ok());
    
    let text = response.unwrap();
    println!("\n{}", text);
    
    // Verify we got numbers in response
    assert!(text.contains("1") && text.contains("5"), 
            "Response should contain numbers 1 through 5");
}

#[test]
#[ignore]
fn test_shell_ask_command_integration() {
    use sentient_shell::ShellState;
    use std::sync::{Arc, Mutex};
    use std::io::Write;
    
    // Create a shell instance
    let mut shell = ShellState::new();
    
    // Capture output
    let output = Arc::new(Mutex::new(Vec::new()));
    
    // Test the ask command
    let result = shell.execute_command("ask What is 2 + 2?");
    assert!(result.is_ok(), "Ask command failed: {:?}", result.err());
    
    // The command should not exit
    assert_eq!(result.unwrap(), false);
}

#[test]
#[ignore]
fn test_model_listing() {
    let client = AiClient::new();
    
    // Test models command functionality
    let models = client.list_models();
    assert!(models.is_ok(), "Failed to list models: {:?}", models.err());
    
    let model_list = models.unwrap();
    println!("Available models:");
    for model in &model_list {
        println!("  - {}", model);
    }
    
    assert!(!model_list.is_empty(), "No models found on Ollama server");
    
    // Check if our preferred model is available
    let has_deepseek = model_list.iter().any(|m| m.contains("deepseek"));
    if !has_deepseek {
        println!("Warning: deepseek-v2 model not found. Tests may fail.");
    }
}

// Performance and load tests
#[test]
#[ignore]
fn test_ollama_response_time() {
    use std::time::Instant;
    
    let client = AiClient::new();
    
    let request = OllamaRequest {
        model: "deepseek-v2".to_string(),
        prompt: "Reply with just 'OK'.".to_string(),
        stream: false,
        options: Some(serde_json::json!({
            "temperature": 0.1,
            "top_p": 0.1,
            "max_tokens": 10
        })),
    };
    
    let start = Instant::now();
    let response = client.query_ollama(&request);
    let duration = start.elapsed();
    
    assert!(response.is_ok(), "Request failed");
    println!("Response time: {:?}", duration);
    
    // Warn if response is slow
    if duration.as_secs() > 5 {
        println!("Warning: Ollama response took more than 5 seconds");
    }
}

#[test]
#[ignore]
fn test_error_handling() {
    let client = AiClient::new();
    
    // Test with invalid model
    let request = OllamaRequest {
        model: "non_existent_model_12345".to_string(),
        prompt: "Test".to_string(),
        stream: false,
        ..Default::default()
    };
    
    let response = client.query_ollama(&request);
    assert!(response.is_err(), "Should fail with non-existent model");
    
    let error = response.err().unwrap();
    println!("Expected error: {}", error);
}