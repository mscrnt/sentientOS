#!/bin/bash
# Install phi model as boot-level AI for SentientOS

set -e

BOOT_DIR="/mnt/d/Projects/SentientOS/sentient-bootloader/esp"
MODEL_PATH="$BOOT_DIR/phi.Q8_0.gguf"
MODEL_URL="https://huggingface.co/TheBloke/phi-2-GGUF/resolve/main/phi-2.Q8_0.gguf"
SHA256_EXPECTED="d56bc96b5e9ecd4bedb970b3b16cf7bb1b15f965de88dcb4f9cf8db7a45f2c01"

echo "🧠 Installing phi-2 model for SentientOS boot..."

# Create backup of current boot model if exists
if [ -f "$BOOT_DIR/neuro_model.gguf" ]; then
    echo "📦 Backing up existing model..."
    cp "$BOOT_DIR/neuro_model.gguf" "$BOOT_DIR/neuro_model.gguf.backup"
fi

# Download phi model
echo "📥 Downloading phi-2 Q8_0 model (1.6GB)..."
if command -v wget &> /dev/null; then
    wget -O "$MODEL_PATH" "$MODEL_URL"
elif command -v curl &> /dev/null; then
    curl -L -o "$MODEL_PATH" "$MODEL_URL"
else
    echo "❌ Neither wget nor curl found. Please install one."
    exit 1
fi

# Verify download
if [ ! -f "$MODEL_PATH" ]; then
    echo "❌ Download failed!"
    exit 1
fi

# Check file size (should be around 1.6GB)
SIZE=$(stat -c%s "$MODEL_PATH" 2>/dev/null || stat -f%z "$MODEL_PATH" 2>/dev/null)
if [ "$SIZE" -lt 1000000000 ]; then
    echo "❌ Downloaded file too small: $SIZE bytes"
    exit 1
fi

# Generate checksum
echo "🔐 Generating checksum..."
sha256sum "$MODEL_PATH" > "$BOOT_DIR/phi.sha256"

# Create symlink for compatibility
ln -sf "phi.Q8_0.gguf" "$BOOT_DIR/boot_model.gguf"

echo "✅ Phi model installed successfully!"
echo "📍 Location: $MODEL_PATH"
echo "📏 Size: $(du -h "$MODEL_PATH" | cut -f1)"
echo ""
echo "Next steps:"
echo "1. Update bootloader.toml configuration"
echo "2. Rebuild kernel with boot model support"
echo "3. Test with: echo '!@ help' | sentient-shell"