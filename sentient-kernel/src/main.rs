#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(allocator_api)]
#![feature(const_mut_refs)]
#![feature(custom_test_frameworks)]

extern crate alloc;

use core::panic::PanicInfo;
use log::info;

mod allocator;
mod boot_info;
mod interrupts;
mod logger;
mod memory;
mod serial;
mod vga;
mod ai;
mod drivers;

use boot_info::BootInfo;

/// Kernel entry point called by bootloader
#[no_mangle]
pub extern "C" fn kernel_main(boot_info_addr: u64) -> ! {
    // Initialize serial for early debugging
    serial::init();
    
    // Set up logging
    logger::init().expect("Failed to initialize logger");
    
    info!("🧠 SentientOS Kernel v0.1 - AI-First Operating System");
    info!("Received BootInfo at: 0x{:016x}", boot_info_addr);
    
    // Parse and validate BootInfo from bootloader
    let boot_info = unsafe { boot_info::parse_boot_info(boot_info_addr) };
    
    info!("📊 System Information:");
    info!("  CPU: {} ({} cores)", boot_info.hardware.cpu_vendor, boot_info.hardware.cpu_features.cores);
    info!("  Memory: {} MB", boot_info.hardware.total_memory / (1024 * 1024));
    info!("  AI Model: {} ({} bytes)", boot_info.model.name, boot_info.model.size_bytes);
    
    // Initialize memory management
    memory::init(&boot_info);
    
    // Initialize heap allocator
    allocator::init_heap();
    
    // Initialize interrupts
    interrupts::init();
    
    // Initialize VGA for visual output
    vga::init();
    vga::print_banner();
    
    // Initialize AI subsystem with loaded model
    ai::init(&boot_info.model, &boot_info.config);
    
    // Initialize hardware drivers based on AI recommendations
    drivers::init_with_ai_hints(&boot_info.hardware);
    
    // Start the AI-aware scheduler
    info!("🚀 Starting AI-aware scheduler...");
    
    // Main kernel loop
    loop {
        // Process AI inference requests
        ai::process_pending_requests();
        
        // Let AI influence scheduling decisions
        ai::update_scheduler_hints();
        
        // Handle system tasks
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial::println!("🔴 KERNEL PANIC: {}", info);
    
    // Attempt to save panic info for AI analysis
    ai::log_panic_for_analysis(info);
    
    loop {
        x86_64::instructions::hlt();
    }
}