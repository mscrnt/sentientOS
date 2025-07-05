# SentientOS CI/CD Documentation

## Overview

This directory contains GitHub Actions workflows for automated testing of the SentientOS bootloader and kernel.

## Workflows

### qemu-test.yml

The main CI workflow that:
1. Builds the UEFI bootloader and kernel from source
2. Creates a bootable disk image with proper UEFI structure
3. Runs the image in QEMU with OVMF firmware
4. Validates the boot sequence by analyzing serial output
5. Uploads artifacts for debugging

**Triggers:**
- Push to `main` or `develop` branches
- Pull requests to `main`
- Manual workflow dispatch

**Key Features:**
- Headless QEMU execution with serial output capture
- Comprehensive boot phase validation
- Artifact upload for failed builds
- Code quality checks (formatting and clippy)

## Boot Validation

The CI validates these critical boot phases:

1. **Serial Initialization** - UART communication established
2. **Bootloader Start** - UEFI bootloader begins execution
3. **Hardware Detection** - CPU, memory, and devices enumerated
4. **AI Model Loading** - GGUF model loaded into memory
5. **Kernel Loading** - kernel.efi found and loaded
6. **Kernel Execution** - Control transferred to kernel
7. **Memory Management** - Kernel heap initialized
8. **AI Subsystem** - AI runtime initialized
9. **Runtime Entry** - Main kernel loop started

## Local Testing

To run the same tests locally:

```bash
# Build and test complete system
./test_system.sh

# Validate boot output
./scripts/validate-boot.sh serial.log

# Verbose validation
./scripts/validate-boot.sh serial.log true
```

## Debugging Failed Builds

If the CI fails:

1. Check the "Analyze boot output" step for missing phases
2. Download the `serial-output` artifact for full logs
3. Download the `boot-artifacts` for the disk image

Common issues:
- Missing kernel.efi in ESP
- Bootloader can't find kernel
- Memory allocation failures
- Model loading errors

## Dependencies

The CI environment includes:
- Rust nightly with UEFI target
- QEMU with x86_64 system emulation
- OVMF UEFI firmware
- mtools for FAT filesystem manipulation
- LLVM toolchain

## Performance

Typical CI run times:
- Build: ~2-3 minutes
- Boot test: ~30 seconds
- Total workflow: ~5 minutes

## Future Enhancements

- [ ] Add memory leak detection
- [ ] Performance regression testing
- [ ] Multi-architecture support (ARM64)
- [ ] Integration tests for AI subsystem
- [ ] Code coverage reporting