use std::env;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    
    // Compile assembly
    cc::Build::new()
        .file("src/arch/x86_64/boot.s")
        .compile("boot");
    
    // Link the assembly
    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static=boot");
    
    // Rerun if assembly changes
    println!("cargo:rerun-if-changed=src/arch/x86_64/boot.s");
}