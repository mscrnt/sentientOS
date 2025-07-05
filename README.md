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
│   │   └── ...
│   └── esp/                # EFI System Partition
│       ├── EFI/BOOT/       # BOOTX64.EFI location
│       └── inference_config.json
└── README.md
```

## Building

### Prerequisites
- Rust nightly toolchain
- `x86_64-unknown-uefi` target
- QEMU (for testing)
- OVMF UEFI firmware

### Build Instructions
```bash
cd sentient-bootloader
cargo +nightly build --release --target x86_64-unknown-uefi
```

### Testing with QEMU
```bash
qemu-system-x86_64 \
  -m 4096 \
  -bios /usr/share/ovmf/OVMF.fd \
  -drive format=raw,file=fat:rw:esp \
  -serial mon:stdio \
  -display none
```

## Boot Process

1. **🔵 BOOT Phase**: Initialize serial logging and UEFI services
2. **🟣 AI Phase**: Detect CPU features, GPU devices, and system memory
3. **🟡 LOAD Phase**: Load configuration and AI model from ESP
4. **🟢 EXEC Phase**: Prepare system state and launch kernel

## Configuration

The bootloader reads `inference_config.json` from the ESP:

```json
{
  "model_path": "\\neuro_model.gguf",
  "runtime": "Hybrid",
  "batch_size": 1,
  "context_length": 4096,
  "temperature": 0.7
}
```

## License

This project is currently under development. License TBD.

## Contributing

SentientOS is in early development. Contributions and feedback are welcome!

---

🤖 *Building the future of AI-native computing*