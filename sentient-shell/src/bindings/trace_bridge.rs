use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use std::path::Path;

/// Trace entry for Python serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEntryPython {
    pub trace_id: String,
    pub timestamp: String,
    pub prompt: String,
    pub intent: String,
    pub model_used: String,
    pub tool_executed: Option<String>,
    pub rag_used: bool,
    pub success: bool,
    pub duration_ms: u64,
    pub reward: Option<f32>,
    pub feedback: Option<String>,
    pub state: StateDictPython,
}

/// State dictionary for Python
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDictPython {
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
    pub model_availability: std::collections::HashMap<String, bool>,
}

/// Bridge for converting Rust traces to Python format
pub struct TraceBridge;

impl TraceBridge {
    /// Convert Rust trace to Python dictionary
    pub fn trace_to_pydict(py: Python, trace: &TraceEntryPython) -> Result<PyObject> {
        let dict = PyDict::new(py);
        
        // Basic fields
        dict.set_item("trace_id", &trace.trace_id)?;
        dict.set_item("timestamp", &trace.timestamp)?;
        dict.set_item("prompt", &trace.prompt)?;
        dict.set_item("intent", &trace.intent)?;
        dict.set_item("model_used", &trace.model_used)?;
        dict.set_item("tool_executed", &trace.tool_executed)?;
        dict.set_item("rag_used", trace.rag_used)?;
        dict.set_item("success", trace.success)?;
        dict.set_item("duration_ms", trace.duration_ms)?;
        dict.set_item("reward", trace.reward)?;
        dict.set_item("feedback", &trace.feedback)?;
        
        // Convert state
        let state_dict = PyDict::new(py);
        state_dict.set_item("intent_type", &trace.state.intent_type)?;
        state_dict.set_item("intent_confidence", trace.state.intent_confidence)?;
        state_dict.set_item("prompt_length", trace.state.prompt_length)?;
        state_dict.set_item("has_tool_keywords", trace.state.has_tool_keywords)?;
        state_dict.set_item("has_query_keywords", trace.state.has_query_keywords)?;
        state_dict.set_item("has_code_keywords", trace.state.has_code_keywords)?;
        state_dict.set_item("time_of_day", trace.state.time_of_day)?;
        state_dict.set_item("previous_success_rate", trace.state.previous_success_rate)?;
        state_dict.set_item("rag_available", trace.state.rag_available)?;
        state_dict.set_item("avg_response_time", trace.state.avg_response_time)?;
        
        // Model availability
        let model_avail = PyDict::new(py);
        for (model, avail) in &trace.state.model_availability {
            model_avail.set_item(model, avail)?;
        }
        state_dict.set_item("model_availability", model_avail)?;
        
        dict.set_item("state", state_dict)?;
        
        Ok(dict.into())
    }
    
    /// Load traces from JSONL and convert to Python list
    pub fn load_traces_for_python(trace_file: &Path) -> Result<Vec<TraceEntryPython>> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        
        let file = File::open(trace_file)?;
        let reader = BufReader::new(file);
        let mut traces = Vec::new();
        
        for line in reader.lines() {
            let line = line?;
            if !line.trim().is_empty() {
                match serde_json::from_str::<TraceEntryPython>(&line) {
                    Ok(trace) => traces.push(trace),
                    Err(e) => log::warn!("Skipping invalid trace: {}", e),
                }
            }
        }
        
        Ok(traces)
    }
    
    /// Export traces to Python-friendly format
    pub fn export_to_python(traces: Vec<TraceEntryPython>) -> Result<PyObject> {
        Python::with_gil(|py| {
            let py_list = pyo3::types::PyList::empty(py);
            
            for trace in traces {
                let py_dict = Self::trace_to_pydict(py, &trace)?;
                py_list.append(py_dict)?;
            }
            
            Ok(py_list.into())
        })
    }
}

/// Live reload support for RL policy
pub struct PolicyReloader {
    policy_path: String,
    last_modified: Option<std::time::SystemTime>,
}

impl PolicyReloader {
    pub fn new(policy_path: String) -> Self {
        PolicyReloader {
            policy_path,
            last_modified: None,
        }
    }
    
    /// Check if policy file has been modified
    pub fn needs_reload(&mut self) -> Result<bool> {
        let metadata = std::fs::metadata(&self.policy_path)?;
        let modified = metadata.modified()?;
        
        match self.last_modified {
            None => {
                self.last_modified = Some(modified);
                Ok(true) // First load
            }
            Some(last) => {
                if modified > last {
                    self.last_modified = Some(modified);
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }
    }
    
    /// Reload policy in Python
    pub fn reload_policy(&self) -> Result<()> {
        Python::with_gil(|py| {
            let code = format!(r#"
import pickle
import torch

# Force reload of policy
policy_path = "{}"
if 'loaded_policy' in globals():
    del globals()['loaded_policy']

with open(policy_path, 'rb') as f:
    loaded_policy = pickle.load(f)
    
print(f"🔄 Reloaded RL policy from {{policy_path}}")
"#, self.policy_path);
            
            py.run(&code, None, None)?;
            Ok(())
        })
    }
}