#!/bin/bash

# Local CI test script - runs the same tests as GitHub Actions
# This helps debug CI failures locally

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}🧪 Running local CI tests for SentientOS${NC}"
echo "========================================"

# Check dependencies
echo -e "\n${BLUE}📋 Checking dependencies...${NC}"
MISSING_DEPS=()

command -v cargo >/dev/null 2>&1 || MISSING_DEPS+=("cargo")
command -v qemu-system-x86_64 >/dev/null 2>&1 || MISSING_DEPS+=("qemu-system-x86_64")
command -v mkfs.vfat >/dev/null 2>&1 || MISSING_DEPS+=("mkfs.vfat (dosfstools)")

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
    MISSING_DEPS+=("ovmf")
fi

if [ ${#MISSING_DEPS[@]} -gt 0 ]; then
    echo -e "${RED}❌ Missing dependencies:${NC}"
    for dep in "${MISSING_DEPS[@]}"; do
        echo "  - $dep"
    done
    echo -e "\n${YELLOW}Install with: sudo apt-get install qemu-system-x86 ovmf dosfstools${NC}"
    exit 1
fi

echo -e "${GREEN}✅ All dependencies found${NC}"

# Clean previous artifacts
echo -e "\n${BLUE}🧹 Cleaning previous build artifacts...${NC}"
rm -f sentient-bootloader/boot.img
rm -f sentient-bootloader/serial.log
rm -f sentient-bootloader/OVMF_*.fd

# Build bootloader
echo -e "\n${BLUE}🔨 Building bootloader...${NC}"
cd sentient-bootloader
cargo build --release --target x86_64-unknown-uefi
if [ ! -f "target/x86_64-unknown-uefi/release/sentient-bootloader.efi" ]; then
    echo -e "${RED}❌ Bootloader build failed${NC}"
    exit 1
fi
cp target/x86_64-unknown-uefi/release/sentient-bootloader.efi esp/EFI/BOOT/BOOTX64.EFI
echo -e "${GREEN}✅ Bootloader built${NC}"

# Build kernel
echo -e "\n${BLUE}🔨 Building kernel...${NC}"
cd ../sentient-kernel
cargo build --release --target x86_64-unknown-uefi
if [ ! -f "target/x86_64-unknown-uefi/release/kernel.efi" ]; then
    echo -e "${RED}❌ Kernel build failed${NC}"
    exit 1
fi
cp target/x86_64-unknown-uefi/release/kernel.efi ../sentient-bootloader/esp/kernel.efi
echo -e "${GREEN}✅ Kernel built${NC}"

# Create disk image
echo -e "\n${BLUE}💾 Creating bootable disk image...${NC}"
cd ../sentient-bootloader
dd if=/dev/zero of=boot.img bs=1M count=128 2>/dev/null
mkfs.vfat -F 32 boot.img >/dev/null

# Mount and copy files
mkdir -p mnt
if command -v sudo >/dev/null 2>&1; then
    sudo mount -o loop boot.img mnt
    sudo cp -r esp/* mnt/
    sudo mkdir -p mnt/EFI/BOOT
    sudo umount mnt
else
    # For systems without sudo (like containers)
    mcopy -i boot.img -s esp/* ::
fi
rmdir mnt 2>/dev/null || true
echo -e "${GREEN}✅ Disk image created${NC}"

# Run QEMU
echo -e "\n${BLUE}🚀 Running QEMU boot test...${NC}"
cp "$OVMF_CODE" ./OVMF_CODE.fd
cp "$OVMF_VARS" ./OVMF_VARS.fd 2>/dev/null || cp "$OVMF_CODE" ./OVMF_VARS.fd

timeout 30s qemu-system-x86_64 \
    -machine q35,accel=tcg \
    -m 4096 \
    -cpu max \
    -drive if=pflash,format=raw,readonly=on,file=OVMF_CODE.fd \
    -drive if=pflash,format=raw,file=OVMF_VARS.fd \
    -drive format=raw,file=boot.img \
    -nographic \
    -serial file:serial.log \
    -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
    || QEMU_EXIT=$?

echo -e "${GREEN}✅ QEMU test completed (exit code: ${QEMU_EXIT:-0})${NC}"

# Validate boot
echo -e "\n${BLUE}📊 Validating boot sequence...${NC}"
cd ..
./scripts/validate-boot.sh sentient-bootloader/serial.log

# Summary
echo -e "\n${BLUE}📈 Test Summary${NC}"
echo "==============="
if [ -f "sentient-bootloader/serial.log" ]; then
    echo -e "${GREEN}✅ Serial log captured${NC}"
    echo "   View with: cat sentient-bootloader/serial.log"
else
    echo -e "${RED}❌ No serial log captured${NC}"
fi

if [ -f "sentient-bootloader/boot.img" ]; then
    echo -e "${GREEN}✅ Boot image created${NC}"
    echo "   Size: $(du -h sentient-bootloader/boot.img | cut -f1)"
else
    echo -e "${RED}❌ Boot image not created${NC}"
fi

echo -e "\n${GREEN}🎉 Local CI test completed!${NC}"