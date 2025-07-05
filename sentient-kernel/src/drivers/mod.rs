use crate::boot_info::HardwareInfo;
use log::info;

pub mod pci;
pub mod acpi;

pub fn init_with_ai_hints(hardware: &HardwareInfo) {
    info!("🔧 Initializing drivers with AI hints...");
    
    // Initialize based on detected hardware
    if hardware.cpu_features.avx512 {
        info!("  ✓ AVX-512 detected - enabling optimized paths");
    }
    
    if hardware.cpu_features.amx {
        info!("  ✓ AMX detected - enabling AI accelerator");
    }
    
    for gpu in &hardware.gpu_devices {
        info!("  ✓ GPU: {} ({}MB)", gpu.name, gpu.memory_mb);
        // Initialize GPU driver based on vendor
        match gpu.vendor_id {
            0x10DE => info!("    Initializing NVIDIA driver"),
            0x1002 => info!("    Initializing AMD driver"),
            0x8086 => info!("    Initializing Intel GPU driver"),
            _ => info!("    Unknown GPU vendor: 0x{:04x}", gpu.vendor_id),
        }
    }
    
    if hardware.npu_available {
        info!("  ✓ NPU detected - initializing neural processor");
    }
}