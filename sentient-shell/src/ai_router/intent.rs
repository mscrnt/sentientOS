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

/// Intent detector
pub struct IntentDetector;

impl IntentDetector {
    /// Detect intent from prompt
    pub fn detect(prompt: &str) -> Intent {
        let prompt_lower = prompt.to_lowercase();
        let word_count = prompt_lower.split_whitespace().count();
        
        // Tool calling patterns
        if prompt_lower.contains("!@") || 
           prompt_lower.contains("!$") ||
           prompt_lower.contains("!#") ||
           prompt_lower.contains("!&") ||
           prompt_lower.contains("!~") ||
           (prompt_lower.contains("call") && prompt_lower.contains("tool")) ||
           prompt_lower.contains("execute") && (prompt_lower.contains("command") || prompt_lower.contains("tool")) ||
           prompt_lower.contains("run") && (prompt_lower.contains("command") || prompt_lower.contains("tool")) {
            debug!("Intent: ToolCall - detected tool execution patterns");
            return Intent::ToolCall;
        }
        
        // Command execution patterns
        if prompt_lower.starts_with("sudo") ||
           prompt_lower.starts_with("ls") ||
           prompt_lower.starts_with("cd") ||
           prompt_lower.starts_with("pwd") ||
           prompt_lower.starts_with("mkdir") ||
           prompt_lower.contains("shell command") ||
           prompt_lower.contains("terminal command") {
            debug!("Intent: CommandExecution - detected command patterns");
            return Intent::CommandExecution;
        }
        
        // Code generation patterns
        if (prompt_lower.contains("write") || prompt_lower.contains("create") || prompt_lower.contains("implement")) && 
           (prompt_lower.contains("code") || 
            prompt_lower.contains("function") ||
            prompt_lower.contains("class") ||
            prompt_lower.contains("program") ||
            prompt_lower.contains("script") ||
            prompt_lower.contains("module")) ||
           prompt_lower.contains("implement") && prompt_lower.contains("in rust") ||
           prompt_lower.contains("generate") && prompt_lower.contains("code") {
            debug!("Intent: CodeGeneration - detected code writing patterns");
            return Intent::CodeGeneration;
        }
        
        // System analysis patterns
        if prompt_lower.contains("analyze") && (prompt_lower.contains("system") || prompt_lower.contains("log")) ||
           prompt_lower.contains("diagnose") ||
           prompt_lower.contains("debug") ||
           prompt_lower.contains("troubleshoot") ||
           (prompt_lower.contains("system") || prompt_lower.contains("service")) && 
           (prompt_lower.contains("health") ||
            prompt_lower.contains("status") ||
            prompt_lower.contains("performance") ||
            prompt_lower.contains("problem")) ||
           prompt_lower.contains("check") && (prompt_lower.contains("memory") || prompt_lower.contains("disk") || prompt_lower.contains("cpu")) {
            debug!("Intent: SystemAnalysis - detected system diagnostic patterns");
            return Intent::SystemAnalysis;
        }
        
        // Visual analysis patterns
        if prompt_lower.contains("screenshot") ||
           prompt_lower.contains("image") ||
           prompt_lower.contains("picture") ||
           prompt_lower.contains("visual") ||
           prompt_lower.contains("see") && prompt_lower.contains("screen") ||
           prompt_lower.contains("look at") && prompt_lower.contains("display") ||
           prompt_lower.contains("ui") && (prompt_lower.contains("debug") || prompt_lower.contains("analyze")) {
            debug!("Intent: VisualAnalysis - detected visual analysis patterns");
            return Intent::VisualAnalysis;
        }
        
        // Documentation patterns
        if prompt_lower.contains("document") ||
           prompt_lower.contains("explain how") ||
           prompt_lower.contains("tutorial") ||
           prompt_lower.contains("guide") ||
           prompt_lower.contains("readme") ||
           prompt_lower.contains("write docs") {
            debug!("Intent: Documentation - detected documentation patterns");
            return Intent::Documentation;
        }
        
        // Complex reasoning patterns
        if prompt_lower.contains("explain") && (prompt_lower.contains("why") || prompt_lower.contains("how")) ||
           prompt_lower.contains("compare") && prompt_lower.contains("between") ||
           prompt_lower.contains("analyze") && prompt_lower.contains("implications") ||
           prompt_lower.contains("pros and cons") ||
           prompt_lower.contains("trade-off") ||
           prompt_lower.contains("deep dive") ||
           word_count > 50 {  // Long prompts often need reasoning
            debug!("Intent: ComplexReasoning - detected complex analysis patterns");
            return Intent::ComplexReasoning;
        }
        
        // Quick response patterns
        if word_count < 10 &&
           !prompt_lower.contains("?") &&
           !prompt_lower.contains("explain") &&
           !prompt_lower.contains("how") &&
           !prompt_lower.contains("why") {
            debug!("Intent: QuickResponse - detected short, simple query");
            return Intent::QuickResponse;
        }
        
        // Conversation patterns
        if prompt_lower.contains("chat") ||
           prompt_lower.contains("talk") ||
           prompt_lower.contains("hello") ||
           prompt_lower.contains("hi ") ||
           prompt_lower.starts_with("hi") ||
           prompt_lower.contains("thanks") ||
           prompt_lower.contains("thank you") {
            debug!("Intent: Conversation - detected conversational patterns");
            return Intent::Conversation;
        }
        
        // Default to general query
        debug!("Intent: GeneralQuery - no specific patterns detected");
        Intent::GeneralQuery
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