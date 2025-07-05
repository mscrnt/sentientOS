# SentientShell - AI-Native CLI for SentientOS

SentientShell is the interactive command-line interface for SentientOS, providing direct access to AI capabilities through both local and remote inference engines.

## Features

- **Interactive CLI** - Full command-line interface with command history and editing
- **AI Integration** - Direct integration with Ollama and Stable Diffusion servers
- **Local Inference** - Optional ONNX runtime for offline AI capabilities
- **Serial Support** - Works over UART for kernel/QEMU integration

## Commands

- `help` - Show available commands
- `status` - Display system status and AI connectivity
- `ask <prompt>` - Query the AI language model
- `models` - List available AI models
- `image <prompt>` - Generate an image from text description
- `exit` - Exit the shell

## Building

### For Native Terminal
```bash
cargo build --release
```

### For UEFI/Kernel Integration
The shell is built into the kernel for SentientOS. See the kernel documentation.

## Running

### Standalone Mode
```bash
./target/release/sentient-shell
```

### Serial Mode (for kernel integration)
```bash
SENTIENT_SERIAL=1 SENTIENT_SERIAL_PORT=/dev/ttyS0 ./target/release/sentient-shell
```

## AI Endpoints Configuration

SentientShell connects to local AI services for inference:

### Ollama Server (LLM)
- **URL**: `http://192.168.69.197:11434`
- **Preferred Model**: `deepseek-v2`
- **Fallback**: First available model

To test locally:
```bash
# Start Ollama server
ollama serve

# Pull the model
ollama pull deepseek-v2

# Test the shell
./target/release/sentient-shell
> ask What is the meaning of life?
```

### Stable Diffusion WebUI
- **URL**: `http://192.168.69.197:7860`
- **API**: `/sdapi/v1/txt2img`

To test locally:
```bash
# Start SD WebUI with API enabled
python launch.py --api

# Test image generation
./target/release/sentient-shell
> image A beautiful sunset over mountains
```

## Local Inference (Optional)

Enable local ONNX inference:
```bash
cargo build --release --features local-inference

# Set model path
export SENTIENT_LOCAL_MODEL=models/tiny_llm.onnx
```

## CI Testing

The GitHub Actions CI tests basic shell functionality without requiring external AI endpoints:
- Shell startup and banner
- Help command
- Status display
- Model listing (shows kernel AI info)
- Exit handling

Full AI integration testing must be done locally with the configured endpoints.

## Development

### Adding New Commands

1. Add command parsing in `execute_command()` in `main.rs`
2. Implement handler in `commands.rs`
3. Update help text
4. Add CI test if applicable (non-network dependent)

### Testing with Kernel

The shell is integrated into the SentientOS kernel and starts automatically after boot:
```bash
# Build and run kernel with shell
cd /path/to/SentientOS
./build.sh
qemu-system-x86_64 -bios OVMF.fd -drive file=boot.img,format=raw -serial stdio
```

## Architecture

```
SentientShell
├── Serial I/O Layer (UART communication)
├── Command Parser (input handling)
├── AI Client
│   ├── Ollama Integration (LLM)
│   └── Stable Diffusion Integration (Image Gen)
├── Local Inference (ONNX)
└── Kernel Integration (when built into SentientOS)
```