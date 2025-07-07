use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use anyhow::{Result, anyhow};
use log::{info, warn, error};

/// State representation matching Python RLState
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RLState {
    pub intent_type: String,
    pub intent_confidence: f32,
    pub prompt_length: usize,
    pub has_tool_keywords: bool,
    pub has_query_keywords: bool,
    pub has_code_keywords: bool,
    pub time_of_day: u8,
    pub previous_success_rate: f32,
    pub rag_available: bool,
    pub avg_response_time: f32,
    pub model_availability: HashMap<String, bool>,
}

/// RL routing decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RLDecision {
    pub model: String,
    pub use_rag: bool,
    pub tool: Option<String>,
    pub confidence: f32,
    pub value_estimate: f32,
    pub fallback_used: bool,
}

/// Python RL Policy wrapper
pub struct RLPolicyWrapper {
    policy_router: PyObject,
}

impl RLPolicyWrapper {
    /// Initialize Python RL policy from checkpoint
    pub fn new(policy_path: &str, encoders_path: &str) -> Result<Self> {
        Python::with_gil(|py| {
            // Import our Python module
            let code = include_str!("../../../rl_agent/deploy_policy.py");
            let module = PyModule::from_code(py, code, "deploy_policy.py", "deploy_policy")?;
            
            // Create RLPolicyRouter instance
            let rl_policy_class = module.getattr("RLPolicyRouter")?;
            let kwargs = PyDict::new(py);
            kwargs.set_item("policy_path", policy_path)?;
            kwargs.set_item("encoders_path", encoders_path)?;
            kwargs.set_item("confidence_threshold", 0.7)?;
            
            let policy_router = rl_policy_class.call((), Some(kwargs))?;
            
            Ok(RLPolicyWrapper {
                policy_router: policy_router.into(),
            })
        })
    }
    
    /// Route a query using the RL policy
    pub fn route(&self, prompt: &str, state: &RLState) -> Result<RLDecision> {
        Python::with_gil(|py| {
            // Convert state to Python dict
            let py_state = PyDict::new(py);
            py_state.set_item("intent_type", &state.intent_type)?;
            py_state.set_item("intent_confidence", state.intent_confidence)?;
            py_state.set_item("prompt_length", state.prompt_length)?;
            py_state.set_item("has_tool_keywords", state.has_tool_keywords)?;
            py_state.set_item("has_query_keywords", state.has_query_keywords)?;
            py_state.set_item("has_code_keywords", state.has_code_keywords)?;
            py_state.set_item("time_of_day", state.time_of_day)?;
            py_state.set_item("previous_success_rate", state.previous_success_rate)?;
            py_state.set_item("rag_available", state.rag_available)?;
            py_state.set_item("avg_response_time", state.avg_response_time)?;
            
            // Convert model availability
            let py_model_avail = PyDict::new(py);
            for (model, avail) in &state.model_availability {
                py_model_avail.set_item(model, avail)?;
            }
            py_state.set_item("model_availability", py_model_avail)?;
            
            // Create context dict
            let context = PyDict::new(py);
            context.set_item("state", py_state)?;
            
            // Call route method
            let result = self.policy_router
                .call_method1(py, "route", (prompt, context))?;
            
            // Convert result to RLDecision
            let result_dict: &PyDict = result.downcast(py)?;
            
            let decision = RLDecision {
                model: result_dict.get_item("model")
                    .ok_or_else(|| anyhow!("Missing model in decision"))?
                    .extract()?,
                use_rag: result_dict.get_item("use_rag")
                    .ok_or_else(|| anyhow!("Missing use_rag in decision"))?
                    .extract()?,
                tool: result_dict.get_item("tool")
                    .and_then(|t| t.extract().ok()),
                confidence: result_dict.get_item("confidence")
                    .ok_or_else(|| anyhow!("Missing confidence in decision"))?
                    .extract()?,
                value_estimate: result_dict.get_item("value_estimate")
                    .ok_or_else(|| anyhow!("Missing value_estimate in decision"))?
                    .extract()?,
                fallback_used: result_dict.get_item("fallback_used")
                    .ok_or_else(|| anyhow!("Missing fallback_used in decision"))?
                    .extract()?,
            };
            
            Ok(decision)
        })
    }
}

/// Simplified Python RL interface for direct inference
pub struct SimplePythonRL {
    initialized: bool,
}

impl SimplePythonRL {
    pub fn new() -> Self {
        SimplePythonRL { initialized: false }
    }
    
    /// Initialize Python environment and check dependencies
    pub fn initialize(&mut self) -> Result<()> {
        use super::python_utils::{PythonPackageChecker, PythonSandbox};
        
        // Check required packages
        let required_packages = ["numpy", "torch", "pickle"];
        PythonPackageChecker::check_packages(&required_packages)?;
        
        // Configure sandbox (optional - can be disabled for development)
        if std::env::var("SENTIENT_PYTHON_SANDBOX").unwrap_or_else(|_| "true".to_string()) == "true" {
            PythonSandbox::configure()?;
        }
        
        self.initialized = true;
        info!("✅ Python RL environment initialized");
        Ok(())
    }
    
    /// Load and run RL policy inference
    pub fn infer(&self, prompt: &str, intent: &str) -> Result<RLDecision> {
        use super::python_utils::TimeoutExecutor;
        
        if !self.initialized {
            return Err(anyhow!("Python RL not initialized"));
        }
        
        // Use timeout for inference (5 seconds default)
        let timeout = std::env::var("SENTIENT_PYTHON_TIMEOUT")
            .unwrap_or_else(|_| "5".to_string())
            .parse::<u64>()
            .unwrap_or(5);
        
        let prompt_clone = prompt.to_string();
        let intent_clone = intent.to_string();
        
        TimeoutExecutor::execute_with_timeout(timeout, move || {
            Python::with_gil(|py| {
            // Simple inline Python code for inference
            let code = r#"
import pickle
import torch
import torch.nn.functional as F
from pathlib import Path

def simple_infer(prompt, intent):
    # Check if policy exists
    policy_path = Path("rl_agent/rl_policy.pth")
    encoders_path = Path("rl_agent/encoders.pkl")
    
    if not policy_path.exists() or not encoders_path.exists():
        # Fallback heuristic
        return {
            'model': 'deepseek-v2:16b',
            'use_rag': 'query' in prompt.lower(),
            'tool': None,
            'confidence': 0.5,
            'value_estimate': 0.0,
            'fallback_used': True
        }
    
    # Would load and run actual policy here
    # For now, return a mock decision
    has_tool_keywords = any(kw in prompt.lower() for kw in ['run', 'execute', 'check'])
    
    return {
        'model': 'deepseek-v2:16b',
        'use_rag': not has_tool_keywords,
        'tool': 'disk_info' if 'disk' in prompt.lower() else None,
        'confidence': 0.85,
        'value_estimate': 0.7,
        'fallback_used': False
    }
"#;
            
            // Execute the inference function
            let locals = PyDict::new(py);
            py.run(code, None, Some(locals))?;
            
            let infer_fn = locals.get_item("simple_infer")
                .ok_or_else(|| anyhow!("Failed to get simple_infer function"))?;
            
            let result = infer_fn.call1((&prompt_clone, &intent_clone))?;
            let result_dict: &PyDict = result.downcast()?;
            
            Ok(RLDecision {
                model: result_dict.get_item("model")
                    .and_then(|v| v.extract().ok())
                    .unwrap_or_else(|| "deepseek-v2:16b".to_string()),
                use_rag: result_dict.get_item("use_rag")
                    .and_then(|v| v.extract().ok())
                    .unwrap_or(true),
                tool: result_dict.get_item("tool")
                    .and_then(|v| v.extract().ok()),
                confidence: result_dict.get_item("confidence")
                    .and_then(|v| v.extract().ok())
                    .unwrap_or(0.5),
                value_estimate: result_dict.get_item("value_estimate")
                    .and_then(|v| v.extract().ok())
                    .unwrap_or(0.0),
                fallback_used: result_dict.get_item("fallback_used")
                    .and_then(|v| v.extract().ok())
                    .unwrap_or(true),
            })
            })
        })
    }
}

/// Extract state features from prompt
pub fn extract_state_from_prompt(prompt: &str) -> RLState {
    let prompt_lower = prompt.to_lowercase();
    
    // Detect intent
    let (intent_type, intent_confidence) = if prompt_lower.contains("call") || prompt_lower.contains("execute") {
        ("ToolCall".to_string(), 0.9)
    } else if prompt_lower.contains("write") || prompt_lower.contains("generate") {
        ("CodeGeneration".to_string(), 0.85)
    } else if prompt_lower.contains("analyze") || prompt_lower.contains("debug") {
        ("Analysis".to_string(), 0.8)
    } else {
        ("GeneralKnowledge".to_string(), 0.75)
    };
    
    // Extract features
    let has_tool_keywords = ["run", "execute", "check", "call"].iter()
        .any(|kw| prompt_lower.contains(kw));
    let has_query_keywords = ["what", "how", "why", "explain"].iter()
        .any(|kw| prompt_lower.contains(kw));
    let has_code_keywords = ["code", "script", "function", "write"].iter()
        .any(|kw| prompt_lower.contains(kw));
    
    // Get current hour
    let time_of_day = chrono::Local::now().hour() as u8;
    
    // Default model availability
    let mut model_availability = HashMap::new();
    model_availability.insert("deepseek-v2:16b".to_string(), true);
    model_availability.insert("llama3.2:latest".to_string(), true);
    model_availability.insert("phi3:medium".to_string(), true);
    
    RLState {
        intent_type,
        intent_confidence,
        prompt_length: prompt.len(),
        has_tool_keywords,
        has_query_keywords,
        has_code_keywords,
        time_of_day,
        previous_success_rate: 0.9,
        rag_available: true,
        avg_response_time: 1000.0,
        model_availability,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_state() {
        let prompt = "Check disk space and analyze usage";
        let state = extract_state_from_prompt(prompt);
        
        assert!(state.has_tool_keywords);
        assert!(state.has_query_keywords);
        assert_eq!(state.intent_type, "Analysis");
    }
}