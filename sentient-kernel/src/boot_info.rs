use crate::serial_println;
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootInfo {
    pub hardware: HardwareInfo,
    pub model: ModelInfo,
    pub config: InferenceConfig,
    pub memory_map: Vec<MemoryRegion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub cpu_vendor: String,
    pub cpu_features: CpuFeatures,
    pub gpu_devices: Vec<GpuDevice>,
    pub npu_available: bool,
    pub total_memory: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuFeatures {
    pub avx2: bool,
    pub avx512: bool,
    pub amx: bool,
    pub cores: u32,
    pub threads: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuDevice {
    pub vendor_id: u16,
    pub device_id: u16,
    pub name: String,
    pub memory_mb: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub format: String,
    pub size_bytes: u64,
    pub quantization: String,
    pub memory_address: u64,
    pub checksum: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    pub model_path: String,
    pub runtime: RuntimeType,
    pub batch_size: u32,
    pub context_length: u32,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeType {
    Cpu,
    Gpu,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRegion {
    pub start: u64,
    pub size: u64,
    pub region_type: MemoryType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryType {
    Conventional,
    Reserved,
    AcpiReclaimable,
    AcpiNvs,
    BootServices,
    RuntimeServices,
    ModelStorage,
}

/// Parse BootInfo from physical address provided by bootloader
pub unsafe fn parse_boot_info(addr: u64) -> &'static BootInfo {
    serial_println!("Parsing BootInfo from 0x{:016x}", addr);

    // Read up to 64KB for the JSON data
    let max_size = 65536;
    let data = core::slice::from_raw_parts(addr as *const u8, max_size);

    // Find null terminator
    let mut json_len = 0;
    for (i, &byte) in data.iter().enumerate().take(max_size) {
        if byte == 0 {
            json_len = i;
            break;
        }
    }

    if json_len == 0 {
        panic!("Empty BootInfo at address 0x{:016x}", addr);
    }

    let json_str = core::str::from_utf8_unchecked(&data[..json_len]);
    serial_println!("BootInfo JSON size: {} bytes", json_len);

    // Parse JSON into static storage
    static mut BOOT_INFO_STORAGE: Option<BootInfo> = None;

    match serde_json::from_str(json_str) {
        Ok(boot_info) => {
            BOOT_INFO_STORAGE = Some(boot_info);
            // Use addr_of! to avoid the warning about mutable static reference
            let ptr = core::ptr::addr_of!(BOOT_INFO_STORAGE);
            unsafe { (*ptr).as_ref().unwrap() }
        }
        Err(e) => {
            panic!("Failed to parse BootInfo JSON: {:?}", e);
        }
    }
}

impl BootInfo {
    #[allow(dead_code)]
    pub fn validate_model(&self) -> bool {
        self.model.memory_address != 0 && self.model.size_bytes > 0
    }

    #[allow(dead_code)]
    pub fn has_ai_acceleration(&self) -> bool {
        self.hardware.cpu_features.avx512
            || self.hardware.cpu_features.amx
            || !self.hardware.gpu_devices.is_empty()
            || self.hardware.npu_available
    }
}
