use crate::ai::{PowerMode, SchedulerHints, SystemMetrics};
use alloc::string::String;

#[derive(Debug, Clone)]
#[allow(dead_code)]
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
