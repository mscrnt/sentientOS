#!/bin/bash
set -e

# Create a bootable disk image for SentientOS
echo "Creating SentientOS boot disk..."

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "esp" ]; then
    echo "Error: This script must be run from the sentient-bootloader directory"
    exit 1
fi

# Check for required tools
if ! command -v mkfs.fat &> /dev/null; then
    echo "Error: mkfs.fat not found. Please install dosfstools"
    exit 1
fi

if ! command -v mmd &> /dev/null; then
    echo "Error: mtools not found. Please install mtools"
    exit 1
fi

# Check for debug mode
DEBUG_MODE=""
if [ "$1" = "--debug" ]; then
    DEBUG_MODE="--features serial-debug"
    echo "Debug mode enabled - serial output will be available"
fi

# Build if needed
if [ ! -f "target/x86_64-unknown-uefi/debug/sentient-bootloader.efi" ]; then
    echo "Building bootloader..."
    cargo build --target x86_64-unknown-uefi $DEBUG_MODE
fi

if [ ! -f "../sentient-kernel/target/x86_64-unknown-uefi/debug/sentient-kernel.efi" ]; then
    echo "Building kernel..."
    (cd ../sentient-kernel && cargo build --target x86_64-unknown-uefi $DEBUG_MODE)
fi

# Create disk image
echo "Creating 128MB disk image..."
dd if=/dev/zero of=boot.img bs=1M count=128 status=none

# Format as FAT32
echo "Formatting as FAT32..."
mkfs.fat -F 32 -n SENTIENT boot.img > /dev/null

# Create directory structure
echo "Creating EFI directory structure..."
mmd -i boot.img ::/EFI
mmd -i boot.img ::/EFI/BOOT

# Copy files
echo "Copying bootloader..."
mcopy -i boot.img target/x86_64-unknown-uefi/debug/sentient-bootloader.efi ::/EFI/BOOT/BOOTX64.EFI

echo "Copying kernel..."
mcopy -i boot.img ../sentient-kernel/target/x86_64-unknown-uefi/debug/sentient-kernel.efi ::/kernel.efi

# Copy other files if they exist
if [ -f "esp/startup.nsh" ]; then
    echo "Copying startup script..."
    mcopy -i boot.img esp/startup.nsh ::/startup.nsh
fi

if [ -f "esp/inference_config.json" ]; then
    echo "Copying inference config..."
    mcopy -i boot.img esp/inference_config.json ::/inference_config.json
fi

if [ -f "esp/neuro_model.gguf" ]; then
    echo "Copying AI model..."
    mcopy -i boot.img esp/neuro_model.gguf ::/neuro_model.gguf
fi

echo "Boot disk created: boot.img"
echo ""
echo "To test with QEMU:"
if [ "$1" = "--debug" ]; then
    echo "  qemu-system-x86_64 -m 4096 -bios /usr/share/ovmf/OVMF.fd -drive format=raw,file=boot.img -serial mon:stdio -nographic"
else
    echo "  qemu-system-x86_64 -m 4096 -bios /usr/share/ovmf/OVMF.fd -drive format=raw,file=boot.img -nographic"
    echo ""
    echo "Note: Serial output is disabled. Use './create-boot-disk.sh --debug' to enable serial debugging."
fi