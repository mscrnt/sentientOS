use alloc::vec::Vec;
use alloc::string::String;
use serde::{Serialize, Deserialize};

pub const BOOT_INFO_MAGIC: u32 = 0x53454E54; // "SENT"

#[repr(C)]
#[derive(Debug)]
pub struct BootInfoHeader {
    pub magic: u32,
    pub version: u32,
    pub size: u64,
}

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
    use log::info;
    
    // First, read the header to validate
    let header = &*(addr as *const BootInfoHeader);
    
    // Check magic number
    if header.magic != BOOT_INFO_MAGIC {
        // Try reading as raw JSON from bootloader (current implementation)
        let json_data = core::slice::from_raw_parts(
            addr as *const u8,
            4096 // Max size to scan for JSON
        );
        
        // Find the end of JSON by looking for null terminator
        let mut json_len = 0;
        for i in 0..4096 {
            if json_data[i] == 0 {
                json_len = i;
                break;
            }
        }
        
        let json_str = core::str::from_utf8_unchecked(&json_data[..json_len]);
        info!("Parsing BootInfo JSON ({} bytes)", json_len);
        
        // Parse JSON into static memory
        // Note: In a real implementation, we'd allocate this properly
        static mut BOOT_INFO_STORAGE: Option<BootInfo> = None;
        
        BOOT_INFO_STORAGE = serde_json::from_str(json_str).ok();
        
        BOOT_INFO_STORAGE.as_ref().expect("Failed to parse BootInfo")
    } else {
        // Future: Handle binary format with header
        panic!("Binary BootInfo format not yet implemented");
    }
}

impl BootInfo {
    /// Get the model's physical memory location
    pub fn model_physical_address(&self) -> Option<u64> {
        if self.model.memory_address != 0 {
            Some(self.model.memory_address)
        } else {
            None
        }
    }
    
    /// Check if system has AI acceleration hardware
    pub fn has_ai_acceleration(&self) -> bool {
        self.hardware.cpu_features.amx || 
        self.hardware.cpu_features.avx512 ||
        !self.hardware.gpu_devices.is_empty() ||
        self.hardware.npu_available
    }
}