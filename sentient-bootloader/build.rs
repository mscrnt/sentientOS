use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::Path;

fn main() -> Result<()> {
    let out_dir = env::var("OUT_DIR").context("OUT_DIR not set")?;
    let target_dir = Path::new(&out_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let efi_src = target_dir.join("sentient-bootloader.efi");
    let efi_dst = Path::new("esp/EFI/BOOT/BOOTX64.EFI");

    if efi_src.exists() {
        fs::create_dir_all(efi_dst.parent().unwrap())?;
        fs::copy(&efi_src, &efi_dst).context(format!(
            "Failed to copy {} to {}",
            efi_src.display(),
            efi_dst.display()
        ))?;
    }

    Ok(())
}
