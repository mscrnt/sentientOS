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

# Find OVMF firmware
OVMF_CODE=""
OVMF_VARS=""
for dir in /usr/share/OVMF /usr/share/ovmf /usr/share/qemu; do
    if [ -f "$dir/OVMF_CODE.fd" ]; then
        OVMF_CODE="$dir/OVMF_CODE.fd"
        OVMF_VARS="$dir/OVMF_VARS.fd"
        break
    elif [ -f "$dir/OVMF.fd" ]; then
        OVMF_CODE="$dir/OVMF.fd"
        OVMF_VARS="$dir/OVMF.fd"
        break
    fi
done

if [ -z "$OVMF_CODE" ]; then
    echo "❌ OVMF firmware not found. Please install ovmf package."
    exit 1
fi

# Create OVMF vars copy
cp "$OVMF_VARS" ./OVMF_VARS.fd 2>/dev/null || cp "$OVMF_CODE" ./OVMF_VARS.fd

# Run QEMU with the ESP
qemu-system-x86_64 \
    -nodefaults \
    -vga std \
    -machine q35,accel=kvm:tcg \
    -m 4096 \
    -cpu host \
    -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
    -drive if=pflash,format=raw,file=./OVMF_VARS.fd \
    -drive format=raw,file=fat:rw:esp \
    -serial mon:stdio \
    -display gtk