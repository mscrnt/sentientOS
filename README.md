# SentientOS

[![QEMU Boot Test](https://github.com/mscrnt/sentientOS/actions/workflows/qemu-test.yml/badge.svg)](https://github.com/mscrnt/sentientOS/actions/workflows/qemu-test.yml)

🧠 **AI-centric operating system with UEFI bootloader for next-gen AI runtime**

## Overview

SentientOS is a bare-metal AI substrate designed to optimize hardware for AI inference and training workloads. It features an AI-aware UEFI bootloader that can detect hardware capabilities, load AI models directly into memory, and configure the system for optimal AI performance.

## Features

### 🚀 AI-Centric UEFI Bootloader
- **Hardware Detection**: Automatic detection of CPU features (AVX2, AVX512, AMX), GPU capabilities, and available memory
- **Model Loading**: Direct loading of GGUF format AI models into reserved memory during boot
- **Serial Logging**: Full boot process monitoring via serial console for remote debugging
- **Boot Phase Tracking**: Color-coded boot phases (BOOT → AI → LOAD → EXEC) with status indicators

### 🧠 AI-Powered Kernel
- **Early Memory Management**: Custom allocator with UEFI boot services integration
- **AI Subsystem**: Built-in AI inference engine with model management
- **System Analysis**: AI-driven system monitoring and optimization
- **Integrated Shell**: Direct kernel shell interface for debugging
- **Security-First Design**: Serial console disabled by default for production security

### 🐚 SentientShell
- **AI Integration**: Native support for Ollama and Stable Diffusion APIs
- **Smart Commands**: AI-enhanced command processing and suggestions
- **Local Inference**: Run AI models directly on system resources
- **WebUI Support**: Integration with Stable Diffusion WebUI for image generation

### 🔧 Technical Specifications
- Written in Rust with `no_std` for minimal overhead
- UEFI 2.x compliant
- Supports x86_64 architecture
- Remote monitoring via COM1 serial port (38400 baud, 8N1)
- JSON configuration for runtime parameters

## Project Structure

```
SentientOS/
├── sentient-bootloader/     # UEFI bootloader
│   ├── src/
│   │   ├── main.rs         # UEFI entry point
│   │   ├── hardware.rs     # Hardware detection
│   │   ├── model.rs        # AI model loading
│   │   ├── serial.rs       # Serial communication
│   │   └── launch.rs       # Kernel launching
│   └── esp/                # EFI System Partition
│       ├── EFI/BOOT/       # BOOTX64.EFI location
│       └── startup.nsh     # UEFI shell startup
├── sentient-kernel/         # AI-centric kernel
│   ├── src/
│   │   ├── main.rs         # Kernel entry point
│   │   ├── ai/             # AI subsystem
│   │   ├── mm/             # Memory management
│   │   ├── sys/            # System services
│   │   └── shell.rs        # Built-in shell interface
│   └── Cargo.toml
├── sentient-shell/          # AI-powered shell
│   ├── src/
│   │   ├── main.rs         # Shell entry point
│   │   ├── ai.rs           # Ollama/SD integration
│   │   ├── commands.rs     # Shell commands
│   │   └── inference.rs    # AI inference handling
│   └── tests/
│       ├── ollama_integration.rs
│       └── sd_integration.rs
└── README.md
```

## Building

### Prerequisites
- Rust nightly toolchain
- `x86_64-unknown-uefi` target
- QEMU (for testing)
- OVMF UEFI firmware

### Build Instructions

#### Build Bootloader
```bash
cd sentient-bootloader
cargo build --target x86_64-unknown-uefi
# For debugging with serial output:
# cargo build --target x86_64-unknown-uefi --features serial-debug
```

#### Build Kernel
```bash
cd sentient-kernel
cargo build --target x86_64-unknown-uefi
# For debugging with serial output:
# cargo build --target x86_64-unknown-uefi --features serial-debug
```

#### Build Shell
```bash
cd sentient-shell
cargo build --release
```

### Testing

#### Automated Testing (Recommended)
The project uses GitHub Actions for continuous integration. Every push automatically:
- Builds the bootloader and kernel
- Creates a bootable disk image
- Tests the full boot chain in QEMU
- Runs code quality checks

View the latest test results: [![QEMU Boot Test](https://github.com/mscrnt/sentientOS/actions/workflows/qemu-test.yml/badge.svg)](https://github.com/mscrnt/sentientOS/actions/workflows/qemu-test.yml)

#### Local Testing with QEMU
For local development, you can test the boot process:

```bash
# Build everything
cd sentient-bootloader && cargo build --target x86_64-unknown-uefi
cd ../sentient-kernel && cargo build --target x86_64-unknown-uefi
cd ../sentient-shell && cargo build --release

# Create boot disk (requires mtools)
cd ../sentient-bootloader
./scripts/create-boot-disk.sh

# Run in QEMU (requires OVMF firmware)
qemu-system-x86_64 \
  -m 4096 \
  -bios /usr/share/ovmf/OVMF.fd \
  -drive format=raw,file=boot.img \
  -nographic
```

## Boot Process

1. **🔵 BOOT Phase**: Initialize serial logging and UEFI services
2. **🟣 AI Phase**: Detect CPU features, GPU devices, and system memory
3. **🟡 LOAD Phase**: Load configuration and AI model from ESP
4. **🟢 EXEC Phase**: Prepare system state and launch kernel
5. **🚀 KERNEL Phase**: Initialize AI subsystem and memory management
6. **🐚 SHELL Phase**: Launch SentientShell with AI capabilities

## Configuration

### Bootloader Configuration
The bootloader passes system information to the kernel via JSON:
```json
{
  "hardware": {
    "cpu_vendor": "AMD",
    "cpu_features": {
      "avx2": true,
      "avx512": false,
      "cores": 16
    },
    "total_memory": 4294967296
  },
  "model": {
    "name": "Neural Core v1.0",
    "memory_address": "0x100000000",
    "size_bytes": 536870912
  }
}
```

### AI Service Configuration
Supports integration with:
- **Ollama**: Local LLM server (default: http://localhost:11434)
- **Stable Diffusion WebUI**: Image generation (default: http://localhost:7860)

## Security

### Serial Console
By default, SentientOS builds with the serial console **disabled** to prevent potential security vulnerabilities. Serial ports can provide:
- Unauthorized debug access
- Potential for information disclosure
- Bypass of normal security controls

To enable serial output for debugging:
```bash
cargo build --features serial-debug
```

**⚠️ WARNING**: Never deploy production systems with `serial-debug` enabled!

## License

This project is currently under development. License TBD.

## Contributing

SentientOS is in early development. Contributions and feedback are welcome!

---

🤖 *Building the future of AI-native computing*