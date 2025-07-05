use alloc::string::String;
use crate::ai::{SchedulerHints, PowerMode, SystemMetrics};

#[derive(Debug, Clone)]
pub enum InferenceRequest {
    SystemAnalysis {
        event: &'static str,
        metrics: SystemMetrics,
    },
    SchedulerOptimization {
        current_hints: SchedulerHints,
    },
    PowerManagement {
        metrics: SystemMetrics,
    },
    PanicAnalysis {
        location: &'static str,
        line: u32,
        message: String,
    },
}

#[derive(Debug, Clone)]
pub enum InferenceResponse {
    SchedulerUpdate(SchedulerHints),
    PowerModeChange(PowerMode),
    SystemCommand(String),
    DiagnosticInfo(String),
}