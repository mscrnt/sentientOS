#!/bin/bash

set -e

echo "🔨 Building SentientOS Kernel..."

# Clean previous builds
rm -rf target/
rm -f kernel.efi

# Build the kernel
cargo build --release --target x86_64-unknown-uefi

# Copy the kernel EFI file
cp target/x86_64-unknown-uefi/release/kernel.efi .

echo "✅ Kernel built successfully: kernel.efi"
echo ""
echo "To test with the bootloader:"
echo "1. Copy kernel.efi to sentient-bootloader/esp/EFI/sentient/"
echo "2. Run the bootloader's test script"