//! Intent detection for intelligent routing

use super::ModelCapability;
use serde::{Deserialize, Serialize};
use log::debug;

/// Intent detected from user prompt
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Intent {
    ToolCall,
    CodeGeneration,
    SystemAnalysis,
    QuickResponse,
    VisualAnalysis,
    ComplexReasoning,
    GeneralQuery,
    CommandExecution,
    Documentation,
    Conversation,
}

/// Intent detection result with confidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentResult {
    pub intent: Intent,
    pub confidence: f32,
    pub signals: Vec<String>,
}

/// Intent detector
pub struct IntentDetector;

impl IntentDetector {
    /// Detect intent with confidence scoring
    pub fn detect_with_confidence(prompt: &str) -> IntentResult {
        let prompt_lower = prompt.to_lowercase();
        let word_count = prompt_lower.split_whitespace().count();
        let mut scores: Vec<(Intent, f32, Vec<String>)> = Vec::new();
        
        // Tool calling patterns
        let mut tool_score = 0.0;
        let mut tool_signals = Vec::new();
        
        if prompt_lower.contains("!@") || prompt_lower.contains("!$") || 
           prompt_lower.contains("!#") || prompt_lower.contains("!&") || 
           prompt_lower.contains("!~") {
            tool_score += 1.0;
            tool_signals.push("Command prefix detected".to_string());
        }
        
        if (prompt_lower.contains("call") || prompt_lower.contains("execute") || 
            prompt_lower.contains("run")) && 
           (prompt_lower.contains("tool") || prompt_lower.contains("command")) {
            tool_score += 0.8;
            tool_signals.push("Tool execution keywords".to_string());
        }
        
        if tool_score > 0.0 {
            scores.push((Intent::ToolCall, tool_score.min(1.0), tool_signals));
        }
        
        // Code generation patterns
        let mut code_score = 0.0;
        let mut code_signals = Vec::new();
        
        if (prompt_lower.contains("write") || prompt_lower.contains("create") || 
            prompt_lower.contains("implement") || prompt_lower.contains("generate")) {
            if prompt_lower.contains("code") || prompt_lower.contains("function") ||
               prompt_lower.contains("class") || prompt_lower.contains("program") ||
               prompt_lower.contains("script") || prompt_lower.contains("module") {
                code_score += 0.9;
                code_signals.push("Code generation keywords".to_string());
            }
        }
        
        if prompt_lower.contains("in rust") || prompt_lower.contains("in python") ||
           prompt_lower.contains("in javascript") {
            code_score += 0.3;
            code_signals.push("Programming language specified".to_string());
        }
        
        if code_score > 0.0 {
            scores.push((Intent::CodeGeneration, code_score.min(1.0), code_signals));
        }
        
        // System analysis patterns
        let mut sys_score = 0.0;
        let mut sys_signals = Vec::new();
        
        if prompt_lower.contains("analyze") || prompt_lower.contains("diagnose") ||
           prompt_lower.contains("debug") || prompt_lower.contains("troubleshoot") {
            sys_score += 0.7;
            sys_signals.push("Analysis keywords".to_string());
        }
        
        if (prompt_lower.contains("system") || prompt_lower.contains("service")) && 
           (prompt_lower.contains("health") || prompt_lower.contains("status") ||
            prompt_lower.contains("performance") || prompt_lower.contains("problem")) {
            sys_score += 0.5;
            sys_signals.push("System monitoring context".to_string());
        }
        
        if sys_score > 0.0 {
            scores.push((Intent::SystemAnalysis, sys_score.min(1.0), sys_signals));
        }
        
        // Visual analysis patterns
        let mut visual_score = 0.0;
        let mut visual_signals = Vec::new();
        
        if prompt_lower.contains("screenshot") || prompt_lower.contains("image") ||
           prompt_lower.contains("picture") || prompt_lower.contains("visual") {
            visual_score += 0.9;
            visual_signals.push("Visual content keywords".to_string());
        }
        
        if visual_score > 0.0 {
            scores.push((Intent::VisualAnalysis, visual_score.min(1.0), visual_signals));
        }
        
        // Complex reasoning patterns
        let mut complex_score = 0.0;
        let mut complex_signals = Vec::new();
        
        if prompt_lower.contains("explain") && (prompt_lower.contains("why") || 
                                                prompt_lower.contains("how")) {
            complex_score += 0.7;
            complex_signals.push("Explanation request".to_string());
        }
        
        if prompt_lower.contains("compare") || prompt_lower.contains("analyze") && 
           prompt_lower.contains("implications") || prompt_lower.contains("trade-off") {
            complex_score += 0.6;
            complex_signals.push("Complex analysis keywords".to_string());
        }
        
        if word_count > 50 {
            complex_score += 0.3;
            complex_signals.push("Long prompt".to_string());
        }
        
        if complex_score > 0.0 {
            scores.push((Intent::ComplexReasoning, complex_score.min(1.0), complex_signals));
        }
        
        // Quick response patterns
        if word_count < 10 && !prompt_lower.contains("?") {
            scores.push((Intent::QuickResponse, 0.8, vec!["Short query".to_string()]));
        }
        
        // Sort by confidence and pick the highest
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        if let Some((intent, confidence, signals)) = scores.first() {
            debug!("Intent: {:?}, Confidence: {:.2}, Signals: {:?}", intent, confidence, signals);
            IntentResult {
                intent: intent.clone(),
                confidence: *confidence,
                signals: signals.clone(),
            }
        } else {
            // Default to general query with low confidence
            IntentResult {
                intent: Intent::GeneralQuery,
                confidence: 0.3,
                signals: vec!["No specific patterns detected".to_string()],
            }
        }
    }
    
    /// Detect intent from prompt (legacy method)
    pub fn detect(prompt: &str) -> Intent {
        Self::detect_with_confidence(prompt).intent
    }
    
    /// Map intent to model capability
    pub fn intent_to_capability(intent: &Intent) -> ModelCapability {
        match intent {
            Intent::ToolCall | Intent::CommandExecution => ModelCapability::Custom("tool_calling".to_string()),
            Intent::CodeGeneration => ModelCapability::CodeGeneration,
            Intent::SystemAnalysis => ModelCapability::Custom("system_analysis".to_string()),
            Intent::QuickResponse => ModelCapability::TextGeneration,
            Intent::VisualAnalysis => ModelCapability::Custom("vision".to_string()),
            Intent::ComplexReasoning => ModelCapability::QuestionAnswering,
            Intent::GeneralQuery => ModelCapability::TextGeneration,
            Intent::Documentation => ModelCapability::TextGeneration,
            Intent::Conversation => ModelCapability::TextGeneration,
        }
    }
    
    /// Get recommended models for intent
    pub fn recommended_models(intent: &Intent) -> Vec<&'static str> {
        match intent {
            Intent::ToolCall => vec!["phi2_local", "mistral:7b-instruct", "llama3:8b"],
            Intent::CommandExecution => vec!["phi2_local", "mistral:7b-instruct"],
            Intent::CodeGeneration => vec!["deepseek-coder-v2", "llama3:8b", "codellama:13b"],
            Intent::SystemAnalysis => vec!["llama3:8b", "mistral:7b-instruct", "phi2_local"],
            Intent::QuickResponse => vec!["phi2_local", "mistral:7b-instruct"],
            Intent::VisualAnalysis => vec!["llama3.2-vision"],
            Intent::ComplexReasoning => vec!["deepseek-coder-v2", "llama3:70b", "mixtral:8x7b"],
            Intent::GeneralQuery => vec!["llama3:8b", "mistral:7b-instruct", "phi2_local"],
            Intent::Documentation => vec!["llama3:8b", "deepseek-coder-v2"],
            Intent::Conversation => vec!["llama3:8b", "mistral:7b-instruct"],
        }
    }
    
    /// Estimate required context length
    pub fn estimate_context_requirement(prompt: &str, intent: &Intent) -> usize {
        let base_tokens = prompt.len() / 4; // Rough token estimate
        
        match intent {
            Intent::ToolCall | Intent::CommandExecution | Intent::QuickResponse => base_tokens + 500,
            Intent::CodeGeneration => base_tokens + 2000, // Need room for code
            Intent::SystemAnalysis => base_tokens + 1500, // Need room for analysis
            Intent::ComplexReasoning => base_tokens + 3000, // Need room for reasoning
            Intent::Documentation => base_tokens + 2500, // Need room for docs
            _ => base_tokens + 1000,
        }
    }
    
    /// Get performance requirements
    pub fn performance_requirements(intent: &Intent) -> PerformanceRequirement {
        match intent {
            Intent::ToolCall | Intent::CommandExecution | Intent::QuickResponse => {
                PerformanceRequirement::Realtime
            },
            Intent::SystemAnalysis | Intent::GeneralQuery | Intent::Conversation => {
                PerformanceRequirement::Fast
            },
            Intent::CodeGeneration | Intent::Documentation => {
                PerformanceRequirement::Balanced
            },
            Intent::ComplexReasoning | Intent::VisualAnalysis => {
                PerformanceRequirement::Powerful
            },
        }
    }
}

/// Performance requirement levels
#[derive(Debug, Clone, PartialEq)]
pub enum PerformanceRequirement {
    Realtime,  // <100ms
    Fast,      // <500ms
    Balanced,  // <2s
    Powerful,  // No constraint
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tool_call_detection() {
        assert_eq!(IntentDetector::detect("!@ call disk_info"), Intent::ToolCall);
        assert_eq!(IntentDetector::detect("execute tool memory_info"), Intent::ToolCall);
        assert_eq!(IntentDetector::detect("run the disk cleanup tool"), Intent::ToolCall);
    }
    
    #[test]
    fn test_code_generation_detection() {
        assert_eq!(IntentDetector::detect("write a function to sort an array"), Intent::CodeGeneration);
        assert_eq!(IntentDetector::detect("create a Rust module for networking"), Intent::CodeGeneration);
        assert_eq!(IntentDetector::detect("implement a binary search algorithm"), Intent::CodeGeneration);
    }
    
    #[test]
    fn test_system_analysis_detection() {
        assert_eq!(IntentDetector::detect("analyze system performance"), Intent::SystemAnalysis);
        assert_eq!(IntentDetector::detect("check memory usage"), Intent::SystemAnalysis);
        assert_eq!(IntentDetector::detect("diagnose the network issue"), Intent::SystemAnalysis);
    }
    
    #[test]
    fn test_performance_requirements() {
        assert_eq!(
            IntentDetector::performance_requirements(&Intent::ToolCall),
            PerformanceRequirement::Realtime
        );
        assert_eq!(
            IntentDetector::performance_requirements(&Intent::ComplexReasoning),
            PerformanceRequirement::Powerful
        );
    }
}