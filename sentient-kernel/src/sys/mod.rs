use crate::ai::{InferenceRequest, InferenceResponse, SystemMetrics};
use crate::boot_info::BootInfo;
use crate::serial_println;
use spin::Mutex;
use uefi::prelude::*;
use uefi::CStr16;

pub mod panic;
pub mod time;

static BOOT_TIME: Mutex<u64> = Mutex::new(0);

pub fn init(mut system_table: SystemTable<Boot>, _boot_info: &BootInfo) {
    serial_println!("🔧 Initializing system services...");

    // Record boot time
    *BOOT_TIME.lock() = get_time_ms();

    // Initialize subsystems
    time::init();

    // Print banner
    print_banner(_boot_info, &mut system_table);

    serial_println!("✅ System services initialized");
}

pub fn print_banner(boot_info: &BootInfo, system_table: &mut SystemTable<Boot>) {
    let stdout = system_table.stdout();

    // Clear screen
    stdout.clear().unwrap_or(());

    // Set color to bright cyan
    stdout
        .set_color(
            uefi::proto::console::text::Color::Cyan,
            uefi::proto::console::text::Color::Black,
        )
        .unwrap_or(());

    // Print banner
    stdout.output_string(cstr16!("\r\n")).unwrap_or(());
    stdout
        .output_string(cstr16!(
            "  ███████╗███████╗███╗   ██╗████████╗██╗███████╗███╗   ██╗████████╗\r\n"
        ))
        .unwrap_or(());
    stdout
        .output_string(cstr16!(
            "  ██╔════╝██╔════╝████╗  ██║╚══██╔══╝██║██╔════╝████╗  ██║╚══██╔══╝\r\n"
        ))
        .unwrap_or(());
    stdout
        .output_string(cstr16!(
            "  ███████╗█████╗  ██╔██╗ ██║   ██║   ██║█████╗  ██╔██╗ ██║   ██║   \r\n"
        ))
        .unwrap_or(());
    stdout
        .output_string(cstr16!(
            "  ╚════██║██╔══╝  ██║╚██╗██║   ██║   ██║██╔══╝  ██║╚██╗██║   ██║   \r\n"
        ))
        .unwrap_or(());
    stdout
        .output_string(cstr16!(
            "  ███████║███████╗██║ ╚████║   ██║   ██║███████╗██║ ╚████║   ██║   \r\n"
        ))
        .unwrap_or(());
    stdout
        .output_string(cstr16!(
            "  ╚══════╝╚══════╝╚═╝  ╚═══╝   ╚═╝   ╚═╝╚══════╝╚═╝  ╚═══╝   ╚═╝   \r\n"
        ))
        .unwrap_or(());
    stdout.output_string(cstr16!("\r\n")).unwrap_or(());

    // Reset color
    stdout
        .set_color(
            uefi::proto::console::text::Color::LightGray,
            uefi::proto::console::text::Color::Black,
        )
        .unwrap_or(());

    // Print kernel info
    stdout
        .output_string(cstr16!("  AI-Centric Kernel v0.1.0\r\n"))
        .unwrap_or(());
    stdout.output_string(cstr16!("  Model: ")).unwrap_or(());

    // Convert model name to UTF-16 and print character by character
    for ch in boot_info.model.name.chars() {
        let s = alloc::format!("{ch}");
        let utf16: alloc::vec::Vec<u16> = s.encode_utf16().collect();
        let mut buf = alloc::vec::Vec::with_capacity(utf16.len() + 1);
        buf.extend_from_slice(&utf16);
        buf.push(0); // null terminator
        if let Ok(cstr) = CStr16::from_u16_with_nul(&buf) {
            stdout.output_string(cstr).unwrap_or(());
        }
    }

    stdout.output_string(cstr16!("\r\n")).unwrap_or(());
    stdout.output_string(cstr16!("  Memory: ")).unwrap_or(());

    // Print memory size
    let mem_mb = boot_info.hardware.total_memory / (1024 * 1024);
    let mem_str = alloc::format!("{mem_mb} MB");
    for ch in mem_str.chars() {
        let s = alloc::format!("{ch}");
        let utf16: alloc::vec::Vec<u16> = s.encode_utf16().collect();
        let mut buf = alloc::vec::Vec::with_capacity(utf16.len() + 1);
        buf.extend_from_slice(&utf16);
        buf.push(0); // null terminator
        if let Ok(cstr) = CStr16::from_u16_with_nul(&buf) {
            stdout.output_string(cstr).unwrap_or(());
        }
    }

    stdout.output_string(cstr16!("\r\n\r\n")).unwrap_or(());
}

pub fn get_system_metrics() -> SystemMetrics {
    let uptime = get_time_ms() - *BOOT_TIME.lock();

    SystemMetrics {
        uptime_ms: uptime,
        free_memory: crate::mm::get_free_memory(),
        task_count: 1, // Just kernel for now
        interrupt_count: 0,
    }
}

pub fn get_time_ms() -> u64 {
    time::get_time_ms()
}

#[allow(dead_code)]
pub fn get_uptime_ms() -> u64 {
    get_time_ms() - *BOOT_TIME.lock()
}

#[allow(dead_code)]
pub fn get_task_count() -> u32 {
    1 // Just kernel for now
}

static mut LAST_AI_TICK: u64 = 0;
const AI_TICK_INTERVAL_MS: u64 = 5000; // Only run every 5 seconds

pub fn ai_system_tick() {
    // Rate limit AI system ticks
    unsafe {
        let current_time = get_system_metrics().uptime_ms;
        if current_time - LAST_AI_TICK < AI_TICK_INTERVAL_MS {
            return;
        }
        LAST_AI_TICK = current_time;
    }
    
    // Submit periodic system analysis
    let metrics = get_system_metrics();
    let request = InferenceRequest::SystemAnalysis {
        event: "system_tick",
        metrics,
    };

    if let Err(e) = crate::ai::submit_inference(request) {
        serial_println!("⚠️ Failed to submit system analysis: {}", e);
    }
}

pub fn process_tasks() {
    // In a real OS, this would process task queue
    // For now, just process AI responses
    crate::ai::process_pending_inferences();
}

#[allow(dead_code)]
pub fn shutdown(_status: Status) -> ! {
    serial_println!("🛑 System shutdown requested");

    // We can't shutdown without runtime services
    // Just halt the CPU
    loop {
        x86_64::instructions::hlt();
    }
}

#[allow(dead_code)]
pub fn handle_ai_response(response: InferenceResponse) {
    match response {
        InferenceResponse::SystemCommand(cmd) => {
            serial_println!("🤖 AI Command: {}", cmd);

            match cmd.as_str() {
                "optimize_memory" => {
                    crate::mm::run_memory_optimizer();
                }
                _ => {
                    serial_println!("  ⚠️  Unknown command: {}", cmd);
                }
            }
        }

        InferenceResponse::DiagnosticInfo(info) => {
            serial_println!("🔍 AI Diagnostic: {}", info);
        }

        InferenceResponse::SchedulerUpdate(hints) => {
            serial_println!("📅 Scheduler update: quantum={}ms", hints.time_quantum_ms);
        }

        InferenceResponse::PowerModeChange(mode) => {
            serial_println!("⚡ Power mode: {:?}", mode);
        }
    }
}
