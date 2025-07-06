use super::enhanced_router::EnhancedAIRouter;
use super::config::{get_models_config, get_model_config};
use super::intent::{IntentDetector, Intent};
use anyhow::Result;

/// Explain routing decision
pub fn explain_routing(prompt: &str) -> Result<String> {
    let router = EnhancedAIRouter::new().with_verbose(true);
    let results = router.test_routing(prompt);
    let intent_result = IntentDetector::detect_with_confidence(prompt);
    
    let mut output = String::new();
    output.push_str("intent: ");
    output.push_str(&format!("{:?}\n", intent_result.intent));
    output.push_str("confidence: ");
    output.push_str(&format!("{:.2}\n", intent_result.confidence));
    output.push_str("recommended_models:\n");
    
    if let Some(models) = results.get("recommended_models") {
        if let Some(model_array) = models.as_array() {
            for model in model_array {
                output.push_str(&format!("  - {}\n", model));
            }
        }
    }
    
    output.push_str("reason: >\n  ");
    
    // Build reason based on intent and signals
    let reasons = match intent_result.intent {
        Intent::ToolCall => format!(
            "Detected tool execution pattern. {} Using fast, trusted models that allow tool execution.",
            intent_result.signals.join(". ")
        ),
        Intent::CodeGeneration => format!(
            "Code generation request detected. {} Routing to powerful models with code capabilities.",
            intent_result.signals.join(". ")
        ),
        Intent::SystemAnalysis => format!(
            "System analysis required. {} Using balanced models with diagnostic capabilities.",
            intent_result.signals.join(". ")
        ),
        _ => format!(
            "Intent: {:?}. {} Selected appropriate models for this use case.",
            intent_result.intent,
            intent_result.signals.join(". ")
        ),
    };
    
    output.push_str(&reasons);
    output.push_str("\n");
    
    Ok(output)
}

/// Show trusted models
pub fn show_trusted_models() -> Result<String> {
    let config = get_models_config()
        .ok_or_else(|| anyhow::anyhow!("Models configuration not loaded"))?;
    
    let mut output = String::new();
    output.push_str("🔐 Trusted Models (allowed to execute tools)\n");
    output.push_str("==========================================\n\n");
    
    let trusted_models: Vec<_> = config.models.iter()
        .filter(|(_, m)| m.trusted && m.allow_tool_calls)
        .collect();
    
    if trusted_models.is_empty() {
        output.push_str("No trusted models configured.\n");
    } else {
        for (id, model) in trusted_models {
            output.push_str(&format!("• {} - {} ({:?})\n", 
                id, model.name, model.performance_tier));
            if let Some(notes) = &model.safety_notes {
                output.push_str(&format!("  Safety: {}\n", notes));
            }
        }
    }
    
    output.push_str("\n⚠️  Non-trusted models:\n");
    let untrusted_models: Vec<_> = config.models.iter()
        .filter(|(_, m)| !m.trusted || !m.allow_tool_calls)
        .collect();
    
    for (id, model) in untrusted_models {
        output.push_str(&format!("• {} - {} (trusted: {}, tool_calls: {})\n", 
            id, model.name, model.trusted, model.allow_tool_calls));
    }
    
    Ok(output)
}

/// Toggle model trust status
pub fn toggle_model_trust(model_id: &str, trusted: bool) -> Result<String> {
    // In a real implementation, this would modify the config file
    // For now, we'll just show what would happen
    let config = get_model_config(model_id)
        .ok_or_else(|| anyhow::anyhow!("Model '{}' not found", model_id))?;
    
    let action = if trusted { "trust" } else "untrust" };
    
    Ok(format!(
        "⚠️  To {} model '{}', update models.toml:\n\
         Set: trusted = {}\n\
         Current status: trusted = {}, allow_tool_calls = {}\n\
         \n\
         Note: This is a security-sensitive operation. \
         Only trust models from verified sources.",
        action, model_id, trusted, config.trusted, config.allow_tool_calls
    ))
}