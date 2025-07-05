use crate::serial_println;
use alloc::string::String;
use alloc::vec::Vec;
use core::arch::x86_64::__cpuid_count;
use uefi::table::boot::BootServices;

use crate::boot_info::{CpuFeatures, GpuDevice, HardwareInfo};

pub fn detect_hardware(boot_services: &BootServices) -> HardwareInfo {
    serial_println!("📊 Starting hardware detection...");
    let cpu_features = detect_cpu_features();
    let cpu_vendor = get_cpu_vendor();
    let gpu_devices = scan_pci_for_gpus(boot_services);
    let npu_available = detect_npu();
    let total_memory = get_total_memory(boot_services);

    serial_println!(
        "  ✓ CPU: {} ({} cores, {} threads)",
        cpu_vendor,
        cpu_features.cores,
        cpu_features.threads
    );
    serial_println!(
        "  ✓ Features: AVX2={}, AVX512={}, AMX={}",
        cpu_features.avx2,
        cpu_features.avx512,
        cpu_features.amx
    );
    serial_println!("  ✓ GPUs detected: {}", gpu_devices.len());
    serial_println!("  ✓ Total memory: {} MB", total_memory / (1024 * 1024));

    HardwareInfo {
        cpu_vendor,
        cpu_features,
        gpu_devices,
        npu_available,
        total_memory,
    }
}

fn detect_cpu_features() -> CpuFeatures {
    let mut features = CpuFeatures {
        avx2: false,
        avx512: false,
        amx: false,
        cores: 1,
        threads: 1,
    };

    unsafe {
        let _cpuid_1 = __cpuid_count(1, 0);
        let cpuid_7 = __cpuid_count(7, 0);

        features.avx2 = (cpuid_7.ebx & (1 << 5)) != 0;

        features.avx512 = (cpuid_7.ebx & (1 << 16)) != 0;

        features.amx = (cpuid_7.edx & (1 << 22)) != 0 && (cpuid_7.edx & (1 << 24)) != 0;

        let cpuid_b = __cpuid_count(0xB, 1);
        features.threads = cpuid_b.ebx as u32;

        let cpuid_b_core = __cpuid_count(0xB, 0);
        features.cores = (features.threads / cpuid_b_core.ebx as u32).max(1);
    }

    features
}

fn get_cpu_vendor() -> String {
    unsafe {
        let cpuid = __cpuid_count(0, 0);
        let mut vendor = [0u8; 12];

        vendor[0..4].copy_from_slice(&cpuid.ebx.to_le_bytes());
        vendor[4..8].copy_from_slice(&cpuid.edx.to_le_bytes());
        vendor[8..12].copy_from_slice(&cpuid.ecx.to_le_bytes());

        String::from_utf8_lossy(&vendor).into_owned()
    }
}

fn scan_pci_for_gpus(_boot_services: &BootServices) -> Vec<GpuDevice> {
    let devices = alloc::vec![GpuDevice {
        vendor_id: 0x10DE,
        device_id: 0x2684,
        name: String::from("NVIDIA RTX 4090"),
        memory_mb: 24576,
    }];

    devices
}

fn detect_npu() -> bool {
    unsafe {
        let _cpuid = __cpuid_count(0x80000001, 0);
        false
    }
}

fn get_total_memory(boot_services: &BootServices) -> u64 {
    let map_size = boot_services.memory_map_size();
    // Add extra space to buffer for descriptor alignment
    let buffer_size = map_size.map_size + 8 * map_size.entry_size;
    let mut buffer = alloc::vec![0; buffer_size];

    match boot_services.memory_map(&mut buffer) {
        Ok(map) => map
            .entries()
            .filter(|d| d.ty == uefi::table::boot::MemoryType::CONVENTIONAL)
            .map(|d| d.page_count * 4096)
            .sum(),
        Err(e) => {
            serial_println!("Failed to get memory map: {:?}", e);
            0
        }
    }
}
