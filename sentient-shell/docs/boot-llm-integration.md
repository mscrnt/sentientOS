# Boot LLM Integration for SentientOS

## Overview

The boot-level LLM provides AI capabilities during early boot, recovery mode, and offline scenarios. It uses the phi-2 model (2.7B parameters, ~1.6GB) as a lightweight, efficient fallback when the main AI services are unavailable.

## Architecture

### 1. Boot Model Storage
- Location: `/boot/phi.Q8_0.gguf`
- Format: GGUF (quantized for efficiency)
- Size: ~1.6GB
- Context: 2048 tokens

### 2. Kernel Integration
- Module: `sentient-kernel-core/src/llm/bootmod.rs`
- API: `/llm/boot` kernel endpoint
- Memory limit: 512MB
- Safe mode: Enabled by default

### 3. Shell Integration
- Module: `sentient-shell/src/boot_llm.rs`
- Automatic fallback when main LLM offline
- Prefix command support (!@, !~)
- Environment: `SENTIENT_BOOT_MODE=1`

## Features

### Safe Mode Commands
- `help` - Show available commands
- `status` - System status
- `validate <cmd>` - Check command safety
- `safe-mode` - Enter restricted mode
- `recovery` - Start recovery process

### Security Features
1. **Command Validation**: Detects dangerous patterns (rm -rf, format, etc.)
2. **Memory Limits**: Capped at 512MB
3. **Network Isolation**: No external connections
4. **Safe Mode Default**: Restrictive by default

## Usage

### Testing Boot Mode
```bash
# Force boot mode
export SENTIENT_BOOT_MODE=1

# Test commands
echo "ask help" | sentient-shell
echo "ask validate rm -rf /" | sentient-shell
```

### Integration with AI Router
The boot model is registered as a provider with lower priority:
- Provider: "boot"
- Model: "phi-2-q8_0"
- Priority: 1 (lower than online models)
- Capabilities: text generation, validation, safety checks

### Fallback Behavior
1. Check if main LLM (Ollama) is online
2. If offline or timeout, use boot LLM
3. If boot model unavailable, use rule-based fallback
4. Log warnings for debugging

## Installation

1. Download model:
```bash
./tools/install-boot-phi.sh
```

2. Update bootloader config:
```toml
[bootmodel]
id = "phi"
path = "/phi.Q8_0.gguf"
context_length = 2048
runtime = "Hybrid"
preload = true
safe_mode = true
```

3. Rebuild with boot support:
```bash
cargo build --features boot-llm
```

## Future Enhancements

1. **Hardware Acceleration**: Add GPU/NPU support for faster inference
2. **Model Compression**: Further quantization (Q4_0) for smaller size
3. **Extended Commands**: Add more recovery and diagnostic capabilities
4. **Multi-Model Support**: Allow swapping between different boot models
5. **Persistent Context**: Save/restore conversation state across reboots