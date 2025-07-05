#!/bin/bash

set -e

echo "🚀 Testing SentientOS Complete System (Bootloader + Kernel)"
echo "=============================================="

# Build bootloader
echo "🔨 Building bootloader..."
cd sentient-bootloader
cargo build --release --target x86_64-unknown-uefi
cp target/x86_64-unknown-uefi/release/sentient-bootloader.efi esp/EFI/BOOT/BOOTX64.EFI

# Build kernel
echo "🔨 Building kernel..."
cd ../sentient-kernel
cargo build --release --target x86_64-unknown-uefi
cp target/x86_64-unknown-uefi/release/kernel.efi ../sentient-bootloader/esp/

# Run QEMU
echo "🖥️  Starting QEMU..."
cd ../sentient-bootloader

# Create OVMF vars copy
cp /usr/share/OVMF/OVMF_VARS.fd ./

# Run QEMU with the ESP
qemu-system-x86_64 \
    -nodefaults \
    -vga std \
    -machine q35,accel=kvm:tcg \
    -m 4096 \
    -cpu host \
    -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE.fd \
    -drive if=pflash,format=raw,file=./OVMF_VARS.fd \
    -drive format=raw,file=fat:rw:esp \
    -serial mon:stdio \
    -display gtk