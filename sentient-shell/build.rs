use std::env;

fn main() {
    // If building for UEFI target, create shell.efi
    let target = env::var("TARGET").unwrap_or_default();
    
    if target.contains("uefi") {
        println!("cargo:rerun-if-changed=src/main.rs");
        
        // Set up UEFI-specific build flags
        println!("cargo:rustc-link-arg=/ENTRY:efi_main");
        println!("cargo:rustc-link-arg=/SUBSYSTEM:EFI_APPLICATION");
    }
}