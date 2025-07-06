#!/bin/bash
# Test boot LLM functionality

echo "=== Testing Boot LLM Integration ==="
echo

# Set boot mode environment variable
export SENTIENT_BOOT_MODE=1

echo "1. Testing in boot mode (offline)..."
echo -e "ask help\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A5 "Boot LLM"

echo
echo "2. Testing validation commands..."
echo -e "ask validate rm -rf /\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A2 "DANGEROUS"

echo
echo "3. Testing safe mode..."
echo -e "ask safe-mode\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A3 "safe mode"

echo
echo "4. Testing prefix commands with boot LLM..."
echo -e "!@ service status\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A2 "Boot LLM"

echo
echo "5. Testing fallback when main LLM offline..."
# Kill ollama if running
pkill ollama 2>/dev/null || true
sleep 2

echo -e "ask What is the meaning of life?\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A2 "Boot LLM"

echo
echo "✅ Boot LLM tests complete"

# Cleanup
unset SENTIENT_BOOT_MODE