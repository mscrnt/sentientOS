use sentient_shell::ai::AiClient;
use std::time::Instant;

#[test]
#[ignore] // Run with: cargo test -- --ignored
fn test_sd_connection() {
    let client = AiClient::new(
        "http://192.168.69.197:11434".to_string(),
        "http://192.168.69.197:7860".to_string(),
    );

    // Test basic connectivity to SD
    let response = reqwest::blocking::get("http://192.168.69.197:7860/sdapi/v1/sd-models")
        .expect("Failed to connect to Stable Diffusion");

    assert!(response.status().is_success(), "SD server not responding");

    let body = response.text().unwrap();
    println!("SD server is running, models available");
    assert!(
        body.contains("model_name"),
        "Invalid response from SD server"
    );
}

#[test]
#[ignore]
fn test_sd_list_models() {
    let client = AiClient::new(
        "http://192.168.69.197:11434".to_string(),
        "http://192.168.69.197:7860".to_string(),
    );

    let models = client.list_sd_models();
    assert!(
        models.is_ok(),
        "Failed to list SD models: {:?}",
        models.err()
    );

    let model_list = models.unwrap();
    println!("Available SD models:");
    for model in &model_list {
        println!("  - {}", model);
    }

    assert!(!model_list.is_empty(), "No SD models found");
}

#[test]
#[ignore]
fn test_sd_options() {
    // Test that we can get current SD options
    let response = reqwest::blocking::get("http://192.168.69.197:7860/sdapi/v1/options")
        .expect("Failed to get SD options");

    assert!(response.status().is_success());

    let options: serde_json::Value = response.json().unwrap();
    println!("Current SD settings:");
    println!(
        "  Model: {}",
        options["sd_model_checkpoint"].as_str().unwrap_or("unknown")
    );
    println!("  VAE: {}", options["sd_vae"].as_str().unwrap_or("auto"));

    if let Some(width) = options["img_width"].as_u64() {
        println!("  Default width: {}", width);
    }
    if let Some(height) = options["img_height"].as_u64() {
        println!("  Default height: {}", height);
    }
}

#[test]
#[ignore]
fn test_sd_txt2img_simple() {
    let client = AiClient::new(
        "http://192.168.69.197:11434".to_string(),
        "http://192.168.69.197:7860".to_string(),
    );

    println!("Generating test image...");
    let start = Instant::now();

    // Test with a simple prompt
    let mut client_mut = client;
    let result = client_mut.generate_image("A red cube on a white background");

    let duration = start.elapsed();
    println!("Generation time: {:?}", duration);

    assert!(
        result.is_ok(),
        "Failed to generate image: {:?}",
        result.err()
    );

    let image_info = result.unwrap();
    println!("Image generated successfully!");
    println!("  Hash: {}", image_info.hash);
    println!("  Size: {} bytes", image_info.size);

    assert!(image_info.size > 0, "Generated image is empty");
}

#[test]
#[ignore]
fn test_sd_txt2img_detailed() {
    // Test with more detailed parameters
    let request = serde_json::json!({
        "prompt": "masterpiece, best quality, 1girl, solo, long hair, looking at viewer, smile, blue eyes",
        "negative_prompt": "lowres, bad anatomy, bad hands, text, error, missing fingers",
        "steps": 20,
        "cfg_scale": 7,
        "width": 512,
        "height": 768,
        "sampler_name": "DPM++ 2M",
        "batch_size": 1,
        "n_iter": 1
    });

    let client = reqwest::blocking::Client::new();
    let response = client
        .post("http://192.168.69.197:7860/sdapi/v1/txt2img")
        .json(&request)
        .send()
        .expect("Failed to send request");

    assert!(response.status().is_success(), "SD API returned error");

    let result: serde_json::Value = response.json().unwrap();
    assert!(result["images"].is_array(), "No images in response");

    let images = result["images"].as_array().unwrap();
    assert!(!images.is_empty(), "No images generated");

    println!("Detailed image generated successfully");
    println!("  Number of images: {}", images.len());

    // Check if we got parameters back
    if let Some(info) = result["info"].as_str() {
        let info_json: serde_json::Value = serde_json::from_str(info).unwrap_or_default();
        if let Some(seed) = info_json["seed"].as_i64() {
            println!("  Seed used: {}", seed);
        }
    }
}

#[test]
#[ignore]
fn test_shell_image_command() {
    use sentient_shell::ShellState;

    let mut shell = ShellState::new();

    println!("Testing image command through shell...");
    let result = shell.execute_command("image A cute robot holding a sign that says 'SentientOS'");

    assert!(result.is_ok(), "Image command failed: {:?}", result.err());
    assert_eq!(
        result.unwrap(),
        false,
        "Image command should not exit shell"
    );
}

#[test]
#[ignore]
fn test_sd_samplers() {
    // List available samplers
    let response = reqwest::blocking::get("http://192.168.69.197:7860/sdapi/v1/samplers")
        .expect("Failed to get samplers");

    assert!(response.status().is_success());

    let samplers: Vec<serde_json::Value> = response.json().unwrap();
    println!("Available samplers:");
    for sampler in &samplers {
        if let Some(name) = sampler["name"].as_str() {
            println!("  - {}", name);
        }
    }

    assert!(!samplers.is_empty(), "No samplers available");
}

#[test]
#[ignore]
fn test_sd_performance() {
    // Test generation performance with different settings
    let test_cases = vec![
        ("Low quality", 10, 256, 256),
        ("Medium quality", 20, 512, 512),
        ("High quality", 30, 768, 768),
    ];

    for (name, steps, width, height) in test_cases {
        let request = serde_json::json!({
            "prompt": "A simple geometric shape",
            "steps": steps,
            "width": width,
            "height": height,
        });

        let start = Instant::now();

        let client = reqwest::blocking::Client::new();
        let response = client
            .post("http://192.168.69.197:7860/sdapi/v1/txt2img")
            .json(&request)
            .send();

        match response {
            Ok(resp) if resp.status().is_success() => {
                let duration = start.elapsed();
                println!(
                    "{}: {}x{} @ {} steps - {:?}",
                    name, width, height, steps, duration
                );
            }
            Ok(resp) => {
                println!("{}: Failed with status {}", name, resp.status());
            }
            Err(e) => {
                println!("{}: Error - {}", name, e);
            }
        }
    }
}
