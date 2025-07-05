use crate::ai::{InferenceRequest, InferenceResponse};
use crate::serial_println;
use core::panic::PanicInfo;

pub fn handle_panic(info: &PanicInfo) -> ! {
    serial_println!("\r\n\r\n💥💥💥 KERNEL PANIC 💥💥💥");

    // Extract panic location
    let (file, line) = if let Some(location) = info.location() {
        (location.file(), location.line())
    } else {
        ("unknown", 0)
    };

    // Extract panic message
    let message = alloc::format!("{}", info);

    serial_println!("📍 Location: {}:{}", file, line);
    serial_println!("💬 Message: {}", message);

    // Try to get AI analysis of the panic
    serial_println!("\r\n🤖 Requesting AI panic analysis...");

    let request = InferenceRequest::PanicAnalysis {
        location: "kernel", // Use static string to avoid lifetime issues
        line,
        message: message.clone(),
    };

    // Try to get AI subsystem to analyze if it's still functional
    if let Ok(ai_mutex) = crate::ai::try_get_ai_subsystem() {
        if let Some(ai) = ai_mutex.lock().as_mut() {
            if let Ok(response) = ai.request_inference(request) {
                match response {
                    InferenceResponse::DiagnosticInfo(analysis) => {
                        serial_println!("\r\n🔍 AI Analysis:\r\n{}", analysis);
                    }
                    _ => {
                        serial_println!("❌ Unexpected response from AI");
                    }
                }
            } else {
                serial_println!("❌ AI inference failed");
            }
        } else {
            serial_println!("❌ AI subsystem not initialized");
        }
    } else {
        serial_println!("❌ AI subsystem unavailable");
    }

    // Print stack trace if available
    serial_println!("\r\n📚 Stack trace:");
    serial_println!("  [Stack unwinding not implemented]");

    // Final message
    serial_println!("\r\n🛑 System halted. Please restart.");

    // Halt the system
    loop {
        x86_64::instructions::hlt();
    }
}
