#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

use uefi::prelude::*;
use uefi::proto::loaded_image::LoadedImage;
use uefi::table::Runtime;
use core::panic::PanicInfo;

mod serial;
mod boot_info;
mod ai;
mod mm;
mod sys;

use boot_info::BootInfo;

/// Global system table for kernel use
static mut SYSTEM_TABLE: Option<SystemTable<Boot>> = None;

#[entry]
fn kernel_main(image_handle: Handle, system_table: SystemTable<Boot>) -> Status {
    // Don't use uefi_services as it conflicts with our allocator
    
    // Save system table for global access
    unsafe {
        SYSTEM_TABLE = Some(system_table.unsafe_clone());
    }
    
    // Initialize serial logging
    serial::init();
    serial_println!("🧠 SentientOS Kernel v0.1.0 - AI-First Operating System");
    serial_println!("🚀 Kernel EFI started successfully");
    
    // Get bootinfo address from command line
    let boot_info_addr = match get_boot_info_address(&system_table) {
        Some(addr) => addr,
        None => {
            serial_println!("❌ Failed to get bootinfo address from command line");
            return Status::INVALID_PARAMETER;
        }
    };
    
    serial_println!("📍 BootInfo address: 0x{:016x}", boot_info_addr);
    
    // Parse BootInfo
    let boot_info = unsafe { boot_info::parse_boot_info(boot_info_addr) };
    
    serial_println!("✅ BootInfo parsed successfully");
    serial_println!("📊 System Configuration:");
    serial_println!("  CPU: {} ({} cores)", boot_info.hardware.cpu_vendor, boot_info.hardware.cpu_features.cores);
    serial_println!("  Memory: {} MB", boot_info.hardware.total_memory / (1024 * 1024));
    serial_println!("  Model: {} at 0x{:016x}", boot_info.model.name, boot_info.model.memory_address);
    serial_println!("  Model size: {} MB", boot_info.model.size_bytes / (1024 * 1024));
    
    // Initialize memory management with UEFI memory map
    mm::init(&system_table, &boot_info);
    
    // Initialize AI subsystem with the loaded model
    match ai::init(&boot_info) {
        Ok(_) => serial_println!("✅ AI subsystem initialized"),
        Err(e) => {
            serial_println!("❌ Failed to initialize AI: {}", e);
            return Status::DEVICE_ERROR;
        }
    }
    
    // Initialize system management
    sys::init(unsafe { system_table.unsafe_clone() }, &boot_info);
    
    // Exit boot services and take control
    serial_println!("🔄 Preparing to exit boot services...");
    
    let (runtime_table, _mmap) = system_table.exit_boot_services();
    
    serial_println!("✅ Boot services exited, kernel has full control");
    
    // Enter kernel main loop
    kernel_runtime_loop(runtime_table, boot_info);
}

fn get_boot_info_address(system_table: &SystemTable<Boot>) -> Option<u64> {
    let boot_services = system_table.boot_services();
    
    // Get loaded image protocol to access command line
    let loaded_image = boot_services
        .open_protocol_exclusive::<LoadedImage>(boot_services.image_handle())
        .ok()?;
    
    // Parse command line for bootinfo=0x...
    let load_options_cstr = loaded_image.load_options_as_cstr16().ok()?;
    // Convert to String by collecting the u16 values
    let load_options_vec: alloc::vec::Vec<u16> = load_options_cstr.iter().map(|&ch| u16::from(ch)).collect();
    let load_options = alloc::string::String::from_utf16_lossy(&load_options_vec);
    
    for arg in load_options.split_whitespace() {
        if let Some(addr_str) = arg.strip_prefix("bootinfo=0x") {
            if let Ok(addr) = u64::from_str_radix(addr_str, 16) {
                return Some(addr);
            }
        }
    }
    
    None
}

fn kernel_runtime_loop(_runtime_table: SystemTable<Runtime>, _boot_info: &'static BootInfo) -> ! {
    serial_println!("🎯 Entering AI-driven kernel runtime");
    
    // Main kernel loop
    loop {
        // Process AI inference requests
        ai::process_pending_inferences();
        
        // Let AI analyze system state and make decisions
        sys::ai_system_tick();
        
        // Handle any pending tasks
        sys::process_tasks();
        
        // Power management based on AI hints
        if ai::should_enter_low_power() {
            x86_64::instructions::hlt();
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("🔴 KERNEL PANIC: {}", info);
    
    // Try to have AI analyze the panic
    ai::analyze_panic(info);
    
    // Halt
    loop {
        x86_64::instructions::hlt();
    }
}