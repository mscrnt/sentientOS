#![no_std]
#![no_main]

extern crate alloc;

use uefi::prelude::*;
use uefi::proto::console::text::{Color, Output};
use log::info;
use alloc::vec::Vec;

#[macro_use]
mod serial;
mod boot_info;
mod color;
mod config;
mod file_loader;
mod hardware;
mod launch;
mod model;

use crate::color::{print_phase, Phase};

fn str_to_cstr16(s: &str) -> alloc::vec::Vec<u16> {
    let mut result = alloc::vec::Vec::with_capacity(s.len() + 1);
    for ch in s.encode_utf16() {
        result.push(ch);
    }
    result.push(0);
    result
}

#[entry]
fn main(image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    // Initialize serial port first for early debugging
    serial::init_serial();
    serial_println!("📡 Serial initialized at 38400 8N1");
    
    uefi_services::init(&mut system_table).unwrap();
    
    let system_table_clone = unsafe { system_table.unsafe_clone() };
    let boot_services = system_table_clone.boot_services();
    let stdout = system_table.stdout();
    
    print_banner(stdout);
    serial_println!("╔═══════════════════════════════════════════╗");
    serial_println!("║        SentientOS AI Bootloader v0.1      ║");
    serial_println!("║    Next-Gen AI-Aware UEFI Boot System     ║");
    serial_println!("╚═══════════════════════════════════════════╝");
    serial_println!();
    
    print_phase(stdout, Phase::Boot, "Initializing SentientOS bootloader...");
    
    print_phase(stdout, Phase::Ai, "Detecting AI hardware capabilities...");
    let hw_info = hardware::detect_hardware(boot_services);
    info!("Hardware detection complete: {:?}", hw_info);
    
    print_phase(stdout, Phase::Load, "Loading AI model and configuration...");
    let esp_volume = match file_loader::get_esp_volume(boot_services) {
        Ok(vol) => vol,
        Err(e) => {
            print_phase(stdout, Phase::Error, "Failed to find ESP!");
            serial_println!("❌ ESP volume not found: {:?}", e);
            return e;
        }
    };
    
    let config = match file_loader::load_config(boot_services, &esp_volume) {
        Ok(cfg) => cfg,
        Err(e) => {
            print_phase(stdout, Phase::Error, "Failed to load config!");
            serial_println!("❌ Config load failed: {:?}", e);
            return Status::LOAD_ERROR;
        }
    };
    
    let model_data = match file_loader::load_model(boot_services, &esp_volume, &config.model_path) {
        Ok(data) => data,
        Err(e) => {
            print_phase(stdout, Phase::Error, "Failed to load AI model!");
            serial_println!("❌ Model load failed: {:?}", e);
            return Status::LOAD_ERROR;
        }
    };
    
    let model_info = match model::validate_and_prepare(boot_services, model_data) {
        Ok(info) => info,
        Err(e) => {
            print_phase(stdout, Phase::Error, "Failed to prepare model!");
            serial_println!("❌ Model preparation failed: {:?}", e);
            return e;
        }
    };
    
    print_phase(stdout, Phase::Exec, "Preparing to launch kernel...");
    
    let boot_info = boot_info::BootInfo {
        hardware: hw_info,
        model: model_info,
        config,
        memory_map: Vec::new(),
    };
    
    let kernel_data = match file_loader::load_kernel(boot_services, &esp_volume) {
        Ok(data) => data,
        Err(e) => {
            print_phase(stdout, Phase::Error, "Failed to load kernel!");
            serial_println!("❌ Kernel load failed: {:?}", e);
            return Status::LOAD_ERROR;
        }
    };
    
    launch::launch_kernel(boot_services, image_handle, kernel_data, boot_info)
}

fn print_banner(stdout: &mut Output) {
    stdout.clear().unwrap();
    stdout.set_color(Color::Cyan, Color::Black).unwrap();
    
    stdout.output_string(cstr16!(
        "\r\n╔═══════════════════════════════════════════╗\r\n"
    )).unwrap();
    stdout.output_string(cstr16!(
        "║        SentientOS AI Bootloader v0.1      ║\r\n"
    )).unwrap();
    stdout.output_string(cstr16!(
        "║    Next-Gen AI-Aware UEFI Boot System     ║\r\n"
    )).unwrap();
    stdout.output_string(cstr16!(
        "╚═══════════════════════════════════════════╝\r\n\r\n"
    )).unwrap();
    
    stdout.reset(false).unwrap();
}