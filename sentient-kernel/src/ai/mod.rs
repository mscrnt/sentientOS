use crate::boot_info::{BootInfo, InferenceConfig};
use crate::serial_println;
use alloc::string::String;
use alloc::collections::VecDeque;
use spin::Mutex;
use core::panic::PanicInfo;

mod runtime;
mod inference;
mod scheduler;

use runtime::ModelRuntime;
pub use inference::{InferenceRequest, InferenceResponse};
pub use scheduler::SchedulerHints;

static AI_SUBSYSTEM: Mutex<Option<AISubsystem>> = Mutex::new(None);

pub fn try_get_ai_subsystem() -> Result<&'static Mutex<Option<AISubsystem>>, String> {
    if AI_SUBSYSTEM.lock().is_some() {
        Ok(&AI_SUBSYSTEM)
    } else {
        Err(String::from("AI subsystem not initialized"))
    }
}

pub struct AISubsystem {
    runtime: ModelRuntime,
    config: InferenceConfig,
    request_queue: VecDeque<InferenceRequest>,
    scheduler_hints: SchedulerHints,
    inference_count: u64,
    power_mode: PowerMode,
}

impl AISubsystem {
    pub fn request_inference(&mut self, request: InferenceRequest) -> Result<InferenceResponse, String> {
        self.runtime.infer(&request, &self.config)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PowerMode {
    Performance,
    Balanced,
    LowPower,
}

pub fn init(boot_info: &BootInfo) -> Result<(), String> {
    serial_println!("🧠 Initializing AI subsystem...");
    
    let model_info = &boot_info.model;
    let config = &boot_info.config;
    
    // Validate model
    if model_info.memory_address == 0 {
        return Err(String::from("Model not loaded in memory"));
    }
    
    let runtime = ModelRuntime::new(model_info)?;
    
    serial_println!("  Model: {} ({})", model_info.name, model_info.format);
    serial_println!("  Location: 0x{:016x}", model_info.memory_address);
    serial_println!("  Size: {} MB", model_info.size_bytes / (1024 * 1024));
    serial_println!("  Runtime: {:?}", config.runtime);
    
    let subsystem = AISubsystem {
        runtime,
        config: config.clone(),
        request_queue: VecDeque::new(),
        scheduler_hints: SchedulerHints::default(),
        inference_count: 0,
        power_mode: PowerMode::Balanced,
    };
    
    *AI_SUBSYSTEM.lock() = Some(subsystem);
    
    // Submit initial system analysis request
    submit_inference(InferenceRequest::SystemAnalysis {
        event: "kernel_boot",
        metrics: SystemMetrics::default(),
    })?;
    
    Ok(())
}

pub fn submit_inference(request: InferenceRequest) -> Result<(), String> {
    let mut subsystem = AI_SUBSYSTEM.lock();
    match subsystem.as_mut() {
        Some(ai) => {
            ai.request_queue.push_back(request);
            Ok(())
        }
        None => Err(String::from("AI engine not initialized")),
    }
}

pub fn process_pending_inferences() {
    let mut subsystem = AI_SUBSYSTEM.lock();
    if let Some(ai) = subsystem.as_mut() {
        if let Some(request) = ai.request_queue.pop_front() {
            match ai.runtime.infer(&request, &ai.config) {
                Ok(response) => {
                    ai.inference_count += 1;
                    handle_inference_response(response, ai);
                }
                Err(e) => {
                    serial_println!("⚠️ Inference failed: {}", e);
                }
            }
        }
    }
}

fn handle_inference_response(response: InferenceResponse, subsystem: &mut AISubsystem) {
    match response {
        InferenceResponse::SchedulerUpdate(hints) => {
            serial_println!("📊 AI: Updating scheduler hints");
            subsystem.scheduler_hints = hints;
        }
        InferenceResponse::PowerModeChange(mode) => {
            serial_println!("⚡ AI: Changing power mode to {:?}", mode);
            subsystem.power_mode = mode;
        }
        InferenceResponse::SystemCommand(cmd) => {
            serial_println!("🤖 AI: System command: {}", cmd);
            execute_ai_command(&cmd);
        }
        InferenceResponse::DiagnosticInfo(info) => {
            serial_println!("🔍 AI Diagnostic: {}", info);
        }
    }
}

fn execute_ai_command(cmd: &str) {
    match cmd {
        "optimize_memory" => crate::mm::run_memory_optimizer(),
        "reduce_power" => set_power_mode(PowerMode::LowPower),
        "boost_performance" => set_power_mode(PowerMode::Performance),
        _ => serial_println!("⚠️ Unknown AI command: {}", cmd),
    }
}

pub fn should_enter_low_power() -> bool {
    AI_SUBSYSTEM.lock()
        .as_ref()
        .map(|ai| matches!(ai.power_mode, PowerMode::LowPower))
        .unwrap_or(false)
}

pub fn get_scheduler_hints() -> SchedulerHints {
    AI_SUBSYSTEM.lock()
        .as_ref()
        .map(|ai| ai.scheduler_hints.clone())
        .unwrap_or_default()
}

pub fn analyze_panic(panic_info: &PanicInfo) {
    if let Some(location) = panic_info.location() {
        let panic_data = alloc::format!(
            "Panic at {}:{}:{} - {:?}",
            location.file(),
            location.line(),
            location.column(),
            panic_info.message()
        );
        
        serial_println!("🔍 AI: Analyzing panic: {}", panic_data);
        
        let _ = submit_inference(InferenceRequest::PanicAnalysis {
            location: "kernel", // Use static string to avoid lifetime issues
            line: location.line(),
            message: panic_data,
        });
        
        // Try to process the analysis immediately
        process_pending_inferences();
    }
}

fn set_power_mode(mode: PowerMode) {
    if let Some(ai) = AI_SUBSYSTEM.lock().as_mut() {
        ai.power_mode = mode;
    }
}

#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub uptime_ms: u64,
    pub free_memory: u64,
    pub task_count: u32,
    pub interrupt_count: u64,
}

impl Default for SystemMetrics {
    fn default() -> Self {
        SystemMetrics {
            uptime_ms: 0,
            free_memory: 0,
            task_count: 1,
            interrupt_count: 0,
        }
    }
}