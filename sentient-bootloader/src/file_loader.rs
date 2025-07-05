use crate::serial_println;
use alloc::string::String;
use alloc::vec::Vec;
use log::info;
use uefi::prelude::*;
use uefi::proto::media::file::{File, FileAttribute, FileInfo, FileMode};
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::table::boot::BootServices;
use uefi::CStr16;
use uefi::Identify;

use crate::boot_info::InferenceConfig;

pub fn get_esp_volume(boot_services: &BootServices) -> Result<Handle, Status> {
    let handles = boot_services
        .locate_handle_buffer(uefi::table::boot::SearchType::ByProtocol(
            &SimpleFileSystem::GUID,
        ))
        .map_err(|e| e.status())?;

    for handle in handles.iter() {
        if let Ok(_sfs) = boot_services.open_protocol_exclusive::<SimpleFileSystem>(*handle) {
            return Ok(*handle);
        }
    }

    Err(Status::NOT_FOUND)
}

pub fn load_config(
    boot_services: &BootServices,
    esp_handle: &Handle,
) -> Result<InferenceConfig, Status> {
    let mut sfs = boot_services
        .open_protocol_exclusive::<SimpleFileSystem>(*esp_handle)
        .map_err(|e| e.status())?;
    let mut root = sfs.open_volume().map_err(|e| e.status())?;

    let config_path = cstr16!("\\inference_config.json");
    let file = root
        .open(config_path, FileMode::Read, FileAttribute::empty())
        .map_err(|e| e.status())?;

    let mut file = file.into_regular_file().unwrap();

    let info_size = file
        .get_info::<FileInfo>(&mut [])
        .unwrap_err()
        .data()
        .unwrap();
    let mut info_buffer = Vec::<u8>::with_capacity(info_size);
    info_buffer.resize(info_size, 0);

    let file_info = file
        .get_info::<FileInfo>(&mut info_buffer)
        .map_err(|e| e.status())?;
    let file_size = file_info.file_size() as usize;

    let mut buffer = Vec::<u8>::with_capacity(file_size);
    buffer.resize(file_size, 0);

    file.read(&mut buffer).map_err(|e| e.status())?;

    let config_str = String::from_utf8_lossy(&buffer);
    let config: InferenceConfig =
        serde_json::from_str(&config_str).map_err(|_| Status::INVALID_PARAMETER)?;

    info!("Loaded config: {:?}", config);
    serial_println!(
        "📄 Loaded inference config: runtime={:?}, context={}",
        config.runtime,
        config.context_length
    );
    Ok(config)
}

pub fn load_model(
    boot_services: &BootServices,
    esp_handle: &Handle,
    model_path: &str,
) -> Result<Vec<u8>, Status> {
    let mut sfs = boot_services
        .open_protocol_exclusive::<SimpleFileSystem>(*esp_handle)
        .map_err(|e| e.status())?;
    let mut root = sfs.open_volume().map_err(|e| e.status())?;

    let path_vec = crate::str_to_cstr16(model_path);
    let path_cstr = CStr16::from_u16_with_nul(&path_vec).unwrap();

    let file = root
        .open(path_cstr, FileMode::Read, FileAttribute::empty())
        .map_err(|e| e.status())?;

    let mut file = file.into_regular_file().unwrap();

    let info_size = file
        .get_info::<FileInfo>(&mut [])
        .unwrap_err()
        .data()
        .unwrap();
    let mut info_buffer = Vec::<u8>::with_capacity(info_size);
    info_buffer.resize(info_size, 0);

    let file_info = file
        .get_info::<FileInfo>(&mut info_buffer)
        .map_err(|e| e.status())?;
    let file_size = file_info.file_size() as usize;

    let mut buffer = Vec::<u8>::with_capacity(file_size);
    buffer.resize(file_size, 0);

    let mut offset = 0;
    while offset < file_size {
        let chunk_size = (file_size - offset).min(1024 * 1024);
        file.read(&mut buffer[offset..offset + chunk_size])
            .map_err(|e| e.status())?;
        offset += chunk_size;
    }

    info!("Loaded model: {} bytes", file_size);
    serial_println!(
        "🧠 Loaded AI model: {} bytes from {}",
        file_size,
        model_path
    );
    Ok(buffer)
}

pub fn load_kernel(boot_services: &BootServices, esp_handle: &Handle) -> Result<Vec<u8>, Status> {
    let mut sfs = boot_services
        .open_protocol_exclusive::<SimpleFileSystem>(*esp_handle)
        .map_err(|e| e.status())?;
    let mut root = sfs.open_volume().map_err(|e| e.status())?;

    let kernel_path = cstr16!("\\kernel.efi");
    let file = root
        .open(kernel_path, FileMode::Read, FileAttribute::empty())
        .map_err(|e| e.status())?;

    let mut file = file.into_regular_file().unwrap();

    let info_size = file
        .get_info::<FileInfo>(&mut [])
        .unwrap_err()
        .data()
        .unwrap();
    let mut info_buffer = Vec::<u8>::with_capacity(info_size);
    info_buffer.resize(info_size, 0);

    let file_info = file
        .get_info::<FileInfo>(&mut info_buffer)
        .map_err(|e| e.status())?;
    let file_size = file_info.file_size() as usize;

    let mut buffer = Vec::<u8>::with_capacity(file_size);
    buffer.resize(file_size, 0);

    file.read(&mut buffer).map_err(|e| e.status())?;

    info!("Loaded kernel: {} bytes", file_size);
    serial_println!("🔧 Loaded kernel.efi: {} bytes", file_size);
    Ok(buffer)
}
