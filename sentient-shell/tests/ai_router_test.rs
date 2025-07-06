//! Tests for AI Router safety and routing logic

use sentient_shell::ai_router::{
    intent::{IntentDetector, Intent, IntentResult},
    config::{load_models_config, get_model_config, get_models_for_intent},
    enhanced_router::EnhancedAIRouter,
};

#[test]
fn test_tool_call_intent_detection() {
    // Test command prefix detection
    let result = IntentDetector::detect_with_confidence("!@ call disk_info");
    assert_eq!(result.intent, Intent::ToolCall);
    assert!(result.confidence >= 0.9);
    assert!(result.signals.contains(&"Command prefix detected".to_string()));
    
    // Test natural language tool call
    let result = IntentDetector::detect_with_confidence("Please execute the disk cleanup tool");
    assert_eq!(result.intent, Intent::ToolCall);
    assert!(result.confidence >= 0.7);
}

#[test]
fn test_code_generation_intent() {
    let result = IntentDetector::detect_with_confidence("Write a function to sort an array in Rust");
    assert_eq!(result.intent, Intent::CodeGeneration);
    assert!(result.confidence >= 0.8);
    assert!(result.signals.iter().any(|s| s.contains("Code generation")));
}

#[test]
fn test_confidence_scoring() {
    // High confidence cases
    let high_conf = IntentDetector::detect_with_confidence("!@ tool disk_info");
    assert!(high_conf.confidence >= 0.9);
    
    // Medium confidence
    let med_conf = IntentDetector::detect_with_confidence("analyze the system logs");
    assert!(med_conf.confidence >= 0.5 && med_conf.confidence < 0.9);
    
    // Low confidence (ambiguous)
    let low_conf = IntentDetector::detect_with_confidence("hello");
    assert!(low_conf.confidence < 0.5);
}

#[test]
fn test_model_safety_configuration() {
    // This test requires models.toml to be loaded
    // In a real test environment, we'd use a test config file
    
    // Test that phi2_local is trusted for tool calls
    if let Some(config) = get_model_config("phi2_local") {
        assert!(config.trusted);
        assert!(config.allow_tool_calls);
    }
    
    // Test that deepseek is not trusted for tool calls
    if let Some(config) = get_model_config("deepseek_v2") {
        assert!(!config.trusted);
        assert!(!config.allow_tool_calls);
    }
}

#[test]
fn test_tool_call_routing_safety() {
    // Test that tool calls are only routed to trusted models
    let tool_models = get_models_for_intent("tool_call");
    
    for model_id in &tool_models {
        if let Some(config) = get_model_config(model_id) {
            // All models in tool_call intent should allow tool calls
            assert!(config.allow_tool_calls, 
                "Model {} is in tool_call routing but doesn't allow tool calls", model_id);
        }
    }
}

#[test]
fn test_routing_decision_logging() {
    let router = EnhancedAIRouter::new().with_verbose(true);
    let results = router.test_routing("!@ call system_info");
    
    // Verify all expected fields are present
    assert!(results.contains_key("intent"));
    assert!(results.contains_key("confidence"));
    assert!(results.contains_key("signals"));
    assert!(results.contains_key("recommended_models"));
    assert!(results.contains_key("estimated_tokens"));
    assert!(results.contains_key("temperature"));
}

#[test]
fn test_fallback_chain() {
    // Test that offline chain is used when no models are available
    let router = EnhancedAIRouter::new();
    let results = router.test_routing("Simple query");
    
    if let Some(models) = results.get("recommended_models") {
        // Should include at least one model from offline chain
        let models_str = models.to_string();
        assert!(models_str.contains("phi2_local") || models_str.contains("llama3_local"));
    }
}

#[test]
fn test_context_length_estimation() {
    let short_prompt = "Hello";
    let long_prompt = "a".repeat(1000);
    
    let short_result = IntentDetector::detect_with_confidence(&short_prompt);
    let long_result = IntentDetector::detect_with_confidence(&long_prompt);
    
    // Long prompts should trigger complex reasoning intent
    if long_prompt.len() > 200 {
        assert!(matches!(long_result.intent, Intent::ComplexReasoning | Intent::GeneralQuery));
    }
}

#[test]
fn test_performance_requirements() {
    use sentient_shell::ai_router::intent::PerformanceRequirement;
    
    // Tool calls should require realtime performance
    assert_eq!(
        IntentDetector::performance_requirements(&Intent::ToolCall),
        PerformanceRequirement::Realtime
    );
    
    // Complex reasoning can be slower
    assert_eq!(
        IntentDetector::performance_requirements(&Intent::ComplexReasoning),
        PerformanceRequirement::Powerful
    );
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::path::Path;
    
    #[test]
    #[ignore] // Requires actual config file
    fn test_config_loading() {
        let config_path = Path::new("config/models.toml");
        if config_path.exists() {
            assert!(load_models_config(config_path).is_ok());
            
            // Verify critical models exist
            assert!(get_model_config("phi2_local").is_some());
            assert!(get_model_config("llama3_local").is_some());
        }
    }
    
    #[test]
    #[ignore] // Requires actual routing
    fn test_end_to_end_routing() {
        let router = EnhancedAIRouter::new();
        
        // Test various prompts
        let test_cases = vec![
            ("!@ call disk_info", Intent::ToolCall),
            ("Write a binary search function", Intent::CodeGeneration),
            ("Analyze system memory usage", Intent::SystemAnalysis),
            ("Hi", Intent::QuickResponse),
        ];
        
        for (prompt, expected_intent) in test_cases {
            let results = router.test_routing(prompt);
            if let Some(intent) = results.get("intent") {
                let intent_str = intent.as_str().unwrap_or("");
                assert!(intent_str.contains(&format!("{:?}", expected_intent)));
            }
        }
    }
}