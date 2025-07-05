use crate::boot_info::{ModelInfo, InferenceConfig};
use alloc::vec::Vec;
use alloc::collections::VecDeque;
use spin::Mutex;
use log::{info, warn};
use core::panic::PanicInfo;

pub mod runtime;
pub mod scheduler;
pub mod inference;

use runtime::ModelRuntime;
use inference::{InferenceRequest, InferenceResponse};

static AI_SUBSYSTEM: Mutex<Option<AiSubsystem>> = Mutex::new(None);

pub struct AiSubsystem {
    runtime: ModelRuntime,
    config: InferenceConfig,
    request_queue: VecDeque<InferenceRequest>,
    scheduler_hints: SchedulerHints,
}

#[derive(Default)]
pub struct SchedulerHints {
    pub priority_boost: Vec<(usize, u8)>, // (pid, priority)
    pub memory_pressure: f32,
    pub power_mode: PowerMode,
}

#[derive(Clone, Copy, PartialEq)]
pub enum PowerMode {
    Performance,
    Balanced,
    PowerSave,
}

impl Default for PowerMode {
    fn default() -> Self {
        PowerMode::Balanced
    }
}

pub fn init(model_info: &ModelInfo, config: &InferenceConfig) {
    info!("🧠 Initializing AI subsystem...");
    info!("  Model: {} ({})", model_info.name, model_info.format);
    info!("  Size: {} MB", model_info.size_bytes / (1024 * 1024));
    info!("  Runtime: {:?}", config.runtime);
    
    let runtime = match ModelRuntime::new(model_info) {
        Ok(rt) => rt,
        Err(e) => {
            warn!("Failed to initialize model runtime: {}", e);
            return;
        }
    };
    
    let subsystem = AiSubsystem {
        runtime,
        config: config.clone(),
        request_queue: VecDeque::new(),
        scheduler_hints: SchedulerHints::default(),
    };
    
    *AI_SUBSYSTEM.lock() = Some(subsystem);
    
    info!("✅ AI subsystem initialized successfully");
}

pub fn submit_inference(request: InferenceRequest) -> Result<(), &'static str> {
    let mut subsystem = AI_SUBSYSTEM.lock();
    match subsystem.as_mut() {
        Some(ai) => {
            ai.request_queue.push_back(request);
            Ok(())
        }
        None => Err("AI subsystem not initialized"),
    }
}

pub fn process_pending_requests() {
    let mut subsystem = AI_SUBSYSTEM.lock();
    if let Some(ai) = subsystem.as_mut() {
        if let Some(request) = ai.request_queue.pop_front() {
            // Process inference request
            match ai.runtime.infer(&request, &ai.config) {
                Ok(response) => {
                    handle_inference_response(response);
                }
                Err(e) => {
                    warn!("Inference failed: {}", e);
                }
            }
        }
    }
}

pub fn update_scheduler_hints() {
    let mut subsystem = AI_SUBSYSTEM.lock();
    if let Some(ai) = subsystem.as_mut() {
        // Let AI analyze system state and provide scheduling hints
        let system_metrics = gather_system_metrics();
        
        // Simple heuristic for now - AI can be more sophisticated
        ai.scheduler_hints.memory_pressure = system_metrics.memory_usage;
        
        if system_metrics.cpu_usage > 0.8 {
            ai.scheduler_hints.power_mode = PowerMode::PowerSave;
        } else if system_metrics.cpu_usage < 0.3 {
            ai.scheduler_hints.power_mode = PowerMode::Performance;
        } else {
            ai.scheduler_hints.power_mode = PowerMode::Balanced;
        }
    }
}

pub fn get_scheduler_hints() -> SchedulerHints {
    AI_SUBSYSTEM.lock()
        .as_ref()
        .map(|ai| ai.scheduler_hints.clone())
        .unwrap_or_default()
}

pub fn log_panic_for_analysis(panic_info: &PanicInfo) {
    // Store panic information for AI to analyze patterns
    if let Some(location) = panic_info.location() {
        let panic_data = alloc::format!(
            "Panic at {}:{}:{} - {}",
            location.file(),
            location.line(),
            location.column(),
            panic_info.message().unwrap_or(&format_args!("no message"))
        );
        
        // Submit for AI analysis if possible
        let request = InferenceRequest::SystemAnalysis {
            event_type: "kernel_panic",
            data: panic_data,
        };
        
        let _ = submit_inference(request);
    }
}

fn handle_inference_response(response: InferenceResponse) {
    match response {
        InferenceResponse::Text(text) => {
            info!("AI Response: {}", text);
        }
        InferenceResponse::SystemCommand(cmd) => {
            info!("AI System Command: {}", cmd);
            // Execute AI-suggested system commands safely
        }
        InferenceResponse::SchedulerHint(hint) => {
            // Update scheduler based on AI recommendation
            if let Some(ai) = AI_SUBSYSTEM.lock().as_mut() {
                ai.scheduler_hints = hint;
            }
        }
    }
}

#[derive(Default)]
struct SystemMetrics {
    cpu_usage: f32,
    memory_usage: f32,
    io_wait: f32,
}

fn gather_system_metrics() -> SystemMetrics {
    // TODO: Implement actual metrics gathering
    SystemMetrics::default()
}

impl Clone for SchedulerHints {
    fn clone(&self) -> Self {
        SchedulerHints {
            priority_boost: self.priority_boost.clone(),
            memory_pressure: self.memory_pressure,
            power_mode: self.power_mode,
        }
    }
}