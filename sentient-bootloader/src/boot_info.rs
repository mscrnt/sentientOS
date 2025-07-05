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
