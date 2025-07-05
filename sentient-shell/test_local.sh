#!/bin/bash
# Local testing script for SentientShell with AI endpoints
# This requires Ollama and SD WebUI running on the local network

set -e

echo "🧪 SentientShell Local AI Testing"
echo "================================="
echo ""

# Check if Ollama is accessible
echo -n "Checking Ollama server... "
if curl -s http://192.168.69.197:11434/api/tags > /dev/null 2>&1; then
    echo "✓ Connected"
else
    echo "✗ Not accessible"
    echo "Please ensure Ollama is running at http://192.168.69.197:11434"
    exit 1
fi

# Check if SD WebUI is accessible
echo -n "Checking SD WebUI server... "
if curl -s http://192.168.69.197:7860/sdapi/v1/sd-models > /dev/null 2>&1; then
    echo "✓ Connected"
else
    echo "✗ Not accessible"
    echo "Please ensure SD WebUI is running at http://192.168.69.197:7860"
    exit 1
fi

echo ""
echo "Building SentientShell..."
cargo build --release

echo ""
echo "Running test commands..."
echo ""

# Create test script
cat > test_ai_commands.txt << 'EOF'
help
status
models
ask What is SentientOS and why is it revolutionary?
image A futuristic AI-powered operating system visualization
exit
EOF

# Run shell with test commands
echo "--- Shell Output ---"
./target/release/sentient-shell < test_ai_commands.txt
echo "--- End Output ---"

echo ""
echo "✅ Local AI testing complete!"
echo ""
echo "To run interactive session:"
echo "  ./target/release/sentient-shell"