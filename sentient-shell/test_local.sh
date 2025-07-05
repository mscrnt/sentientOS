#!/bin/bash
set -e

echo "🧪 Testing SentientShell with local Ollama server..."
echo "================================================"

# Check if Ollama is accessible
echo -n "📡 Checking Ollama connectivity... "
if curl -s http://192.168.69.197:11434/api/tags > /dev/null; then
    echo "✅ Connected"
else
    echo "❌ Failed to connect to Ollama at http://192.168.69.197:11434"
    echo "   Please ensure Ollama is running on your local network"
    exit 1
fi

# List available models
echo ""
echo "📦 Available Ollama models:"
curl -s http://192.168.69.197:11434/api/tags | jq -r '.models[].name' | sed 's/^/   - /'

# Build the shell without serial support for testing
echo ""
echo "🔨 Building sentient-shell..."
cd "$(dirname "$0")"
cargo build --release --no-default-features --features local-inference

# Run integration tests
echo ""
echo "🧪 Running integration tests..."
echo "   (This will connect to your local Ollama server)"
echo ""

# Run only the ignored tests (which are the integration tests)
RUST_TEST_THREADS=1 cargo test --release -- --ignored --nocapture

echo ""
echo "✅ Local testing complete!"
echo ""
echo "To run individual tests:"
echo "  cargo test test_ollama_connection -- --ignored --nocapture"
echo "  cargo test test_ollama_generate -- --ignored --nocapture"
echo ""
echo "To run the shell interactively:"
echo "  cargo run --release"