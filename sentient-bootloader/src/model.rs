use uefi::prelude::*;
use uefi::table::boot::{AllocateType, BootServices, MemoryType};
use alloc::vec::Vec;
use alloc::string::String;
use sha2::{Sha256, Digest};
use log::info;
use crate::{serial_println};

use crate::boot_info::ModelInfo;

const GGUF_MAGIC: u32 = 0x46554747;

#[repr(C)]
struct GgufHeader {
    magic: u32,
    version: u32,
    tensor_count: u64,
    metadata_kv_count: u64,
}

pub fn validate_and_prepare(boot_services: &BootServices, model_data: Vec<u8>) -> Result<ModelInfo, Status> {
    if model_data.len() < core::mem::size_of::<GgufHeader>() {
        return Err(Status::INVALID_PARAMETER);
    }
    
    let header = unsafe {
        &*(model_data.as_ptr() as *const GgufHeader)
    };
    
    if header.magic != GGUF_MAGIC {
        info!("Invalid GGUF magic: 0x{:08x}", header.magic);
        return Err(Status::INCOMPATIBLE_VERSION);
    }
    
    info!("GGUF model detected: version {}, tensors: {}", header.version, header.tensor_count);
    serial_println!("✅ GGUF model validated: v{}, {} tensors", header.version, header.tensor_count);
    
    let checksum = calculate_checksum(&model_data);
    
    let pages_needed = (model_data.len() + 4095) / 4096;
    let memory_address = boot_services.allocate_pages(
        AllocateType::AnyPages,
        MemoryType::RESERVED,
        pages_needed
    ).map_err(|_| Status::OUT_OF_RESOURCES)? as u64;
    
    unsafe {
        let dest = memory_address as *mut u8;
        core::ptr::copy_nonoverlapping(model_data.as_ptr(), dest, model_data.len());
    }
    
    info!("Model loaded at address: 0x{:016x}", memory_address);
    serial_println!("💾 Model loaded at: 0x{:016x} ({} pages)", memory_address, pages_needed);
    
    Ok(ModelInfo {
        name: String::from("neuro_model"),
        format: String::from("GGUF"),
        size_bytes: model_data.len() as u64,
        quantization: detect_quantization(&model_data),
        memory_address,
        checksum: Some(checksum),
    })
}

fn calculate_checksum(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    
    for chunk in data.chunks(1024 * 1024) {
        hasher.update(chunk);
    }
    
    let result = hasher.finalize();
    
    let mut checksum = String::with_capacity(64);
    for byte in result {
        checksum.push_str(&alloc::format!("{:02x}", byte));
    }
    
    checksum
}

fn detect_quantization(_data: &[u8]) -> String {
    String::from("Q4_K_M")
}