#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

use core::panic::PanicInfo;
use uefi::prelude::*;
use uefi::proto::loaded_image::LoadedImage;
use uefi::table::Runtime;

mod ai;
mod boot_info;
mod mm;
mod serial;
mod shell;
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
    serial_println!(
        "  CPU: {} ({} cores)",
        boot_info.hardware.cpu_vendor,
        boot_info.hardware.cpu_features.cores
    );
    serial_println!(
        "  Memory: {} MB",
        boot_info.hardware.total_memory / (1024 * 1024)
    );
    serial_println!(
        "  Model: {} at 0x{:016x}",
        boot_info.model.name,
        boot_info.model.memory_address
    );
    serial_println!(
        "  Model size: {} MB",
        boot_info.model.size_bytes / (1024 * 1024)
    );

    // Initialize memory management with UEFI memory map
    mm::init(&system_table, boot_info);

    // Initialize AI subsystem with the loaded model
    match ai::init(boot_info) {
        Ok(_) => serial_println!("✅ AI subsystem initialized"),
        Err(e) => {
            serial_println!("❌ Failed to initialize AI: {}", e);
            return Status::DEVICE_ERROR;
        }
    }

    // Initialize system management
    sys::init(unsafe { system_table.unsafe_clone() }, boot_info);

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
    let load_options_cstr = match loaded_image.load_options_as_cstr16() {
        Ok(opts) => opts,
        Err(_) => {
            serial_println!("⚠️ No load options found");
            return None;
        }
    };

    // Parse without allocation - convert to iterator and look for pattern
    let chars_iter = load_options_cstr.iter();
    let mut addr = 0u64;
    let mut found = false;
    
    // State machine for parsing "bootinfo=0x<hex>"
    let mut state = 0; // 0: searching, 1-11: matching "bootinfo=0x", 12+: parsing hex
    let pattern = "bootinfo=0x";
    
    for ch in chars_iter {
        let c = u16::from(*ch) as u8; // Convert Char16 to u8
        
        if state < pattern.len() {
            // Matching pattern
            if c == pattern.as_bytes()[state] {
                state += 1;
                if state == pattern.len() {
                    // Found the pattern, start parsing hex
                    found = true;
                }
            } else {
                // Reset if no match
                state = 0;
                // Check if this char starts the pattern
                if c == b'b' {
                    state = 1;
                }
            }
        } else if found {
            // Parsing hex digits
            if c >= b'0' && c <= b'9' {
                addr = addr * 16 + (c - b'0') as u64;
            } else if c >= b'a' && c <= b'f' {
                addr = addr * 16 + (c - b'a' + 10) as u64;
            } else if c >= b'A' && c <= b'F' {
                addr = addr * 16 + (c - b'A' + 10) as u64;
            } else {
                // End of hex number
                break;
            }
        }
    }
    
    if found && addr > 0 {
        serial_println!("📋 Found bootinfo address: 0x{:x}", addr);
        Some(addr)
    } else {
        None
    }
}

fn kernel_runtime_loop(_runtime_table: SystemTable<Runtime>, _boot_info: &'static BootInfo) -> ! {
    serial_println!("🎯 Entering AI-driven kernel runtime");

    // Initialize a simple shell state without allocation
    let mut shell_started = false;

    // Main kernel loop
    loop {
        // Start shell after first iteration (when memory is ready)
        if !shell_started {
            serial_println!("🐚 Starting SentientShell...");
            serial_println!("{}", shell::SHELL_BANNER);
            serial_println!("Type 'help' for available commands.\n");
            crate::serial::_print(format_args!("sentient> "));
            shell_started = true;
        }

        // Check for serial input - handle commands directly without allocation
        if let Some(ch) = serial::try_read_char() {
            shell::handle_input_simple(ch);
        }

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
