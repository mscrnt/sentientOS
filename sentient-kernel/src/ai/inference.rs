use alloc::string::String;
use crate::ai::SchedulerHints;

#[derive(Debug, Clone)]
pub enum InferenceRequest {
    TextGeneration {
        prompt: String,
        max_tokens: usize,
    },
    SystemAnalysis {
        event_type: &'static str,
        data: String,
    },
    SchedulerOptimization {
        cpu_usage: f32,
        memory_usage: f32,
    },
}

#[derive(Debug, Clone)]
pub enum InferenceResponse {
    Text(String),
    SystemCommand(String),
    SchedulerHint(SchedulerHints),
}