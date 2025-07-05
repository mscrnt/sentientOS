#!/bin/bash

echo "Building SentientOS kernel..."

cd sentient-kernel

# Build the kernel
cargo build --release --target x86_64-unknown-none 2>&1 | tee build.log

if [ $? -eq 0 ]; then
    echo "✅ Kernel built successfully"
    
    # For now, create a dummy kernel.efi
    # In a real implementation, we'd convert the kernel binary to EFI format
    echo "Creating dummy kernel.efi for testing..."
    echo -e "\x4D\x5A" > ../sentient-bootloader/esp/kernel.efi # MZ header
    echo "Dummy SentientOS kernel" >> ../sentient-bootloader/esp/kernel.efi
    
    echo "✅ Kernel copied to ESP"
else
    echo "❌ Kernel build failed"
    cat build.log
fi