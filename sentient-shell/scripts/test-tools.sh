#!/bin/bash
# Test Tool Use Framework

echo "=== Testing Tool Use Framework ==="
echo

# Build if needed
echo "Building sentient-shell..."
cd /mnt/d/Projects/SentientOS/sentient-shell
cargo build 2>&1 | tail -5

echo
echo "1. List available tools..."
echo -e "tool list\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A20 "Available tools:"

echo
echo "2. Get info about a specific tool..."
echo -e "tool info disk_info\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A10 "Tool:"

echo
echo "3. Execute a simple tool..."
echo -e "tool call disk_info\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A10 "Tool:"

echo
echo "4. Search for tools..."
echo -e "tool search memory\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A5 "matching"

echo
echo "5. Test AI response with tool calls..."
cat << 'EOF' > /tmp/test-tool-ai.txt
ask Please show me the disk usage using the disk_info tool
exit
EOF
cat /tmp/test-tool-ai.txt | ./target/debug/sentient-shell 2>&1 | grep -A15 "Response:"

echo
echo "✅ Tool tests complete"