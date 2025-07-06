//! Enhanced CLI for LLM routing commands

use super::enhanced_router::EnhancedAIRouter;
use super::config::{get_models_config, get_model_config};
use super::intent::{IntentDetector, Intent};
use anyhow::{Result, bail};
use std::collections::HashMap;

/// Handle LLM routing commands
pub fn handle_llm_command(args: &[&str]) -> Result<String> {
    if args.is_empty() {
        return Ok(llm_help());
    }
    
    match args[0] {
        "route" => handle_route_command(&args[1..]),
        "model" => handle_model_command(&args[1..]),
        "help" => Ok(llm_help()),
        _ => Ok(format!("Unknown llm command: {}. Try 'llm help'", args[0])),
    }
}

/// Handle route subcommands
fn handle_route_command(args: &[&str]) -> Result<String> {
    if args.is_empty() {
        return Ok(route_help());
    }
    
    match args[0] {
        "list" => list_routing_rules(),
        "test" => {
            if args.len() < 2 {
                return Ok("Usage: llm route test <prompt>".to_string());
            }
            let verbose = args.contains(&"--verbose");
            let prompt = args.iter()
                .filter(|a| !a.starts_with("--"))
                .skip(1)
                .cloned()
                .collect::<Vec<_>>()
                .join(" ");
            test_routing(&prompt, verbose)
        },
        "explain" => {
            if args.len() < 2 {
                return Ok("Usage: llm route explain <prompt>".to_string());
            }
            super::llm_cli_extra::explain_routing(&args[1..].join(" "))
        },
        "info" => show_routing_info(),
        _ => Ok(format!("Unknown route command: {}. Try 'llm route help'", args[0])),
    }
}

/// Handle model subcommands
fn handle_model_command(args: &[&str]) -> Result<String> {
    if args.is_empty() {
        return Ok(model_help());
    }
    
    match args[0] {
        "info" => show_model_info(&args[1..]),
        "list" => list_all_models(),
        "show-trusted" => super::llm_cli_extra::show_trusted_models(),
        "trust" => {
            if args.len() < 2 {
                return Ok("Usage: llm model trust <model_id>".to_string());
            }
            super::llm_cli_extra::toggle_model_trust(args[1], true)
        },
        "untrust" => {
            if args.len() < 2 {
                return Ok("Usage: llm model untrust <model_id>".to_string());
            }
            super::llm_cli_extra::toggle_model_trust(args[1], false)
        },
        "capabilities" => list_model_capabilities(&args[1..]),
        _ => Ok(format!("Unknown model command: {}. Try 'llm model help'", args[0])),
    }
}

/// LLM help text
fn llm_help() -> String {
    r#"LLM Routing Commands:
  route             Routing configuration and testing
  model             Model information and capabilities
  help              Show this help message

Examples:
  llm route list
  llm route test "Write a function to sort an array"
  llm model info phi2_local
  llm model list"#.to_string()
}

/// Route help text
fn route_help() -> String {
    r#"LLM Route Commands:
  list              Show routing rules and intent mappings
  test <prompt>     Test routing decision for a prompt
  info              Show current routing configuration

Examples:
  llm route list
  llm route test "Analyze system performance"
  llm route info"#.to_string()
}

/// Model help text
fn model_help() -> String {
    r#"LLM Model Commands:
  info <model>      Show detailed information about a model
  list              List all configured models
  capabilities      Show model capabilities

Examples:
  llm model info deepseek-v2
  llm model list
  llm model capabilities phi2_local"#.to_string()
}

/// List routing rules
fn list_routing_rules() -> Result<String> {
    let config = get_models_config()
        .ok_or_else(|| anyhow::anyhow!("Models configuration not loaded"))?;
    
    let mut output = String::new();
    output.push_str("🔀 Routing Rules\n");
    output.push_str("================\n\n");
    
    output.push_str(&format!("Default Model: {}\n", config.routing.default_model));
    output.push_str(&format!("Offline Chain: {:?}\n\n", config.routing.offline_chain));
    
    output.push_str("Intent Mappings:\n");
    for (intent, models) in &config.routing.intents {
        output.push_str(&format!("  {} → {:?}\n", intent, models));
    }
    
    output.push_str("\nPerformance Tiers:\n");
    for (tier, models) in &config.routing.performance {
        output.push_str(&format!("  {} → {:?}\n", tier, models));
    }
    
    output.push_str("\nContext Length Routing:\n");
    for (size, models) in &config.routing.context {
        output.push_str(&format!("  {} → {:?}\n", size, models));
    }
    
    Ok(output)
}

/// Test routing for a prompt
fn test_routing(prompt: &str, verbose: bool) -> Result<String> {
    let router = EnhancedAIRouter::new().with_verbose(verbose);
    let results = router.test_routing(prompt);
    
    let mut output = String::new();
    output.push_str(&format!("🧪 Testing routing for: \"{}\"\n", prompt));
    output.push_str("=====================================\n\n");
    
    if let Some(intent) = results.get("intent") {
        output.push_str(&format!("Detected Intent: {}\n", intent));
    }
    
    if let Some(confidence) = results.get("confidence") {
        output.push_str(&format!("Confidence: {}\n", confidence));
    }
    
    if let Some(signals) = results.get("signals") {
        output.push_str(&format!("Detection Signals: {}\n", signals));
    }
    
    if let Some(models) = results.get("recommended_models") {
        output.push_str(&format!("Recommended Models: {}\n", models));
    }
    
    if let Some(tokens) = results.get("estimated_tokens") {
        output.push_str(&format!("Estimated Tokens: {}\n", tokens));
    }
    
    if let Some(temp) = results.get("temperature") {
        output.push_str(&format!("Temperature: {}\n", temp));
    }
    
    // Also show intent detection details
    output.push_str("\n📊 Intent Analysis:\n");
    let intent = IntentDetector::detect(prompt);
    let perf_req = IntentDetector::performance_requirements(&intent);
    let context_req = IntentDetector::estimate_context_requirement(prompt, &intent);
    
    output.push_str(&format!("  Performance Required: {:?}\n", perf_req));
    output.push_str(&format!("  Context Requirement: {} tokens\n", context_req));
    
    Ok(output)
}

/// Show routing info
fn show_routing_info() -> Result<String> {
    let config = get_models_config()
        .ok_or_else(|| anyhow::anyhow!("Models configuration not loaded"))?;
    
    let mut output = String::new();
    output.push_str("🎯 Routing Configuration\n");
    output.push_str("=======================\n\n");
    
    output.push_str(&format!("Load Balancing Strategy: {}\n", config.load_balancing.strategy));
    output.push_str(&format!("Max Concurrent Requests: {}\n", config.load_balancing.max_concurrent_requests));
    output.push_str(&format!("Timeout: {}ms\n", config.load_balancing.timeout_ms));
    output.push_str(&format!("Retry Attempts: {}\n", config.load_balancing.retry_attempts));
    
    output.push_str("\n📈 Model Priorities:\n");
    let mut models: Vec<_> = config.models.iter().collect();
    models.sort_by_key(|(_, m)| std::cmp::Reverse(m.priority));
    
    for (id, model) in models.iter().take(10) {
        output.push_str(&format!("  {} - {} (priority: {})\n", id, model.name, model.priority));
    }
    
    Ok(output)
}

/// Show model info
fn show_model_info(args: &[&str]) -> Result<String> {
    if args.is_empty() {
        return list_all_models();
    }
    
    let model_id = args[0];
    let config = get_model_config(model_id)
        .ok_or_else(|| anyhow::anyhow!("Model '{}' not found", model_id))?;
    
    let mut output = String::new();
    output.push_str(&format!("🤖 Model: {} ({})\n", config.name, model_id));
    output.push_str("=====================================\n\n");
    
    output.push_str(&format!("Provider: {}\n", config.provider));
    if let Some(endpoint) = &config.endpoint {
        output.push_str(&format!("Endpoint: {}\n", endpoint));
    }
    if let Some(location) = &config.location {
        output.push_str(&format!("Location: {}\n", location));
    }
    
    output.push_str(&format!("Performance Tier: {:?}\n", config.performance_tier));
    output.push_str(&format!("Context Length: {} tokens\n", config.context_length));
    output.push_str(&format!("Priority: {}\n", config.priority));
    
    output.push_str("\nCapabilities:\n");
    for cap in &config.capabilities {
        output.push_str(&format!("  • {}\n", cap));
    }
    
    output.push_str("\nUse Cases:\n");
    for use_case in &config.use_cases {
        output.push_str(&format!("  • {}\n", use_case));
    }
    
    Ok(output)
}

/// List all models
fn list_all_models() -> Result<String> {
    let config = get_models_config()
        .ok_or_else(|| anyhow::anyhow!("Models configuration not loaded"))?;
    
    let mut output = String::new();
    output.push_str("📚 Available Models\n");
    output.push_str("==================\n\n");
    
    // Group by provider
    let mut by_provider: HashMap<String, Vec<(&String, &_)>> = HashMap::new();
    for (id, model) in &config.models {
        by_provider.entry(model.provider.clone())
            .or_default()
            .push((id, model));
    }
    
    for (provider, models) in by_provider {
        output.push_str(&format!("Provider: {}\n", provider));
        
        for (id, model) in models {
            output.push_str(&format!("  • {} - {} ({:?}, {} tokens)\n", 
                id, model.name, model.performance_tier, model.context_length));
        }
        output.push_str("\n");
    }
    
    Ok(output)
}

/// List model capabilities
fn list_model_capabilities(args: &[&str]) -> Result<String> {
    let mut output = String::new();
    output.push_str("🎯 Model Capabilities\n");
    output.push_str("====================\n\n");
    
    if args.is_empty() {
        // Show all capabilities
        let config = get_models_config()
            .ok_or_else(|| anyhow::anyhow!("Models configuration not loaded"))?;
        
        let mut all_caps: HashMap<String, Vec<String>> = HashMap::new();
        
        for (id, model) in &config.models {
            for cap in &model.capabilities {
                all_caps.entry(cap.clone())
                    .or_default()
                    .push(id.clone());
            }
        }
        
        for (cap, models) in all_caps {
            output.push_str(&format!("{}: {:?}\n", cap, models));
        }
    } else {
        // Show capabilities for specific model
        let model_id = args[0];
        let config = get_model_config(model_id)
            .ok_or_else(|| anyhow::anyhow!("Model '{}' not found", model_id))?;
        
        output.push_str(&format!("Model: {}\n", config.name));
        output.push_str("Capabilities:\n");
        for cap in &config.capabilities {
            output.push_str(&format!("  • {}\n", cap));
        }
    }
    
    Ok(output)
}
