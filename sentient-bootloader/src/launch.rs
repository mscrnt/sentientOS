use crate::serial_println;
use alloc::vec::Vec;
use log::info;
use uefi::prelude::*;
use uefi::proto::loaded_image::LoadedImage;
use uefi::table::boot::{BootServices, LoadImageSource};

use crate::boot_info::BootInfo;

pub fn launch_kernel(
    boot_services: &BootServices,
    image_handle: Handle,
    kernel_data: Vec<u8>,
    boot_info: BootInfo,
) -> Status {
    info!("Preparing to launch kernel...");
    serial_println!("🚀 Preparing kernel launch...");

    let boot_info_addr = match store_boot_info(boot_services, boot_info) {
        Ok(addr) => addr,
        Err(e) => {
            serial_println!("❌ Failed to store boot info: {:?}", e);
            return e;
        }
    };

    let kernel_handle = match boot_services.load_image(
        image_handle,
        LoadImageSource::FromBuffer {
            buffer: &kernel_data,
            file_path: None,
        },
    ) {
        Ok(handle) => handle,
        Err(e) => {
            serial_println!("❌ Failed to load kernel image: {:?}", e);
            return e.status();
        }
    };

    {
        let mut loaded_image =
            match boot_services.open_protocol_exclusive::<LoadedImage>(kernel_handle) {
                Ok(img) => img,
                Err(e) => {
                    serial_println!("❌ Failed to open loaded image: {:?}", e);
                    return e.status();
                }
            };

        let args = alloc::format!("bootinfo=0x{:016x}", boot_info_addr);
        let args_vec = crate::str_to_cstr16(&args);

        unsafe {
            loaded_image
                .set_load_options(args_vec.as_ptr() as *const u8, (args_vec.len() * 2) as u32);
        }
    }

    info!("Starting kernel image...");
    serial_println!(
        "🎯 Starting kernel with bootinfo at 0x{:016x}",
        boot_info_addr
    );

    match boot_services.start_image(kernel_handle) {
        Ok(_) => {
            info!("Kernel returned successfully");
            serial_println!("✅ Kernel returned successfully");
            Status::SUCCESS
        }
        Err(e) => {
            info!("Kernel failed to start: {:?}", e);
            serial_println!("❌ Kernel failed to start: {:?}", e);
            e.status()
        }
    }
}

fn store_boot_info(boot_services: &BootServices, boot_info: BootInfo) -> Result<u64, Status> {
    let serialized = serde_json::to_vec(&boot_info).map_err(|_| Status::OUT_OF_RESOURCES)?;

    let pages_needed = (serialized.len() + 4095) / 4096;
    let address = boot_services
        .allocate_pages(
            uefi::table::boot::AllocateType::AnyPages,
            uefi::table::boot::MemoryType::RUNTIME_SERVICES_DATA,
            pages_needed,
        )
        .map_err(|_| Status::OUT_OF_RESOURCES)? as u64;

    unsafe {
        let dest = address as *mut u8;
        core::ptr::copy_nonoverlapping(serialized.as_ptr(), dest, serialized.len());
    }

    info!("BootInfo stored at: 0x{:016x}", address);
    serial_println!(
        "📦 BootInfo stored at: 0x{:016x} ({} bytes)",
        address,
        serialized.len()
    );
    Ok(address)
}
