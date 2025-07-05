#!/bin/bash

# Build the kernel
cargo build --release --target x86_64-unknown-none

# Convert to EFI format
# For now, we'll create a simple EFI stub that loads our kernel
objcopy -O binary target/x86_64-unknown-none/release/sentient-kernel kernel.bin

# Create a minimal EFI stub
cat > efi_stub.s << 'EOF'
.section .text
.global _start

_start:
    # EFI entry point
    # RCX = ImageHandle
    # RDX = SystemTable
    
    # For now, just jump to our kernel
    # In a real implementation, we'd properly set up the environment
    mov $0x200000, %rax
    jmp *%rax
EOF

# Assemble the stub
as -o efi_stub.o efi_stub.s
ld -o kernel.efi -T efi_link.ld efi_stub.o

# Copy to ESP
cp kernel.efi ../sentient-bootloader/esp/kernel.efi

echo "Kernel EFI built and copied to ESP"