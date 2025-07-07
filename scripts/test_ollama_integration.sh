#!/bin/bash
# Test Ollama integration with SentientShell

echo "🧪 Testing Ollama Integration"
echo "============================"

# Set Ollama URL
export OLLAMA_URL="http://192.168.69.197:11434"

echo -e "\n1️⃣ Checking Ollama server..."
curl -s "${OLLAMA_URL}/api/tags" | jq '.models[].name' 2>/dev/null || echo "❌ Ollama server not reachable"

echo -e "\n2️⃣ Testing shell commands..."

# Test status command
echo -e "\n📊 Testing 'status' command:"
echo "status" | /mnt/d/Projects/SentientOS/sentient-shell/target/release/sentient-shell 2>/dev/null | grep -A5 "Ollama" || echo "Shell not built"

# Test ask command
echo -e "\n🤖 Testing 'ask' command:"
echo "ask What is RAM?" | /mnt/d/Projects/SentientOS/sentient-shell/target/release/sentient-shell 2>/dev/null | head -20

# Test models command
echo -e "\n📋 Testing 'models' command:"
echo "models" | /mnt/d/Projects/SentientOS/sentient-shell/target/release/sentient-shell 2>/dev/null | head -20

# Test llm routing
echo -e "\n🧠 Testing LLM routing:"
echo "llm route test 'What is virtual memory?'" | /mnt/d/Projects/SentientOS/sentient-shell/target/release/sentient-shell 2>/dev/null | head -20

echo -e "\n✅ Test complete!"
echo "If you see connection errors, make sure:"
echo "1. Ollama is running at ${OLLAMA_URL}"
echo "2. The shell is built: cd sentient-shell && cargo build --release"