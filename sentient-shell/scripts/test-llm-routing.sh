#!/bin/bash
# Test LLM Routing Framework

echo "=== Testing LLM Routing Framework ==="
echo

# Build if needed
echo "Building sentient-shell..."
cd /mnt/d/Projects/SentientOS/sentient-shell
cargo build 2>&1 | tail -5

echo
echo "1. List routing rules..."
echo -e "llm route list\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A20 "Routing Rules"

echo
echo "2. Show model information..."
echo -e "llm model list\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A20 "Available Models"

echo
echo "3. Test routing for tool call..."
echo -e "llm route test !@ call disk_info\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A10 "Testing routing"

echo
echo "4. Test routing for code generation..."
echo -e "llm route test Write a function to sort an array\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A10 "Testing routing"

echo
echo "5. Test routing for system analysis..."
echo -e "llm route test Analyze system memory usage\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A10 "Testing routing"

echo
echo "6. Show specific model info..."
echo -e "llm model info phi2_local\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A15 "Model:"

echo
echo "✅ LLM routing tests complete"