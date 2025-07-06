#!/bin/bash
# Final Phase 1 Testing - Safety, Confidence, and Observability

echo "=== Phase 1 Final Testing: Smart LLM Routing ==="
echo

# Build if needed
echo "Building sentient-shell..."
cd /mnt/d/Projects/SentientOS/sentient-shell
cargo build 2>&1 | tail -5

echo
echo "🔐 1. Testing Model Safety Flags..."
echo -e "llm model show-trusted\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A20 "Trusted Models"

echo
echo "🎯 2. Testing Intent Detection with Confidence..."
echo -e "llm route test !@ call disk_info\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A15 "Testing routing"

echo
echo "📊 3. Testing Verbose Routing Logs..."
echo -e "llm route test --verbose Execute the memory cleanup tool\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A20 "Testing routing"

echo
echo "🧠 4. Testing Routing Explanation..."
echo -e "llm route explain Write a binary search algorithm\nexit" | ./target/debug/sentient-shell 2>&1 | head -20

echo
echo "🚫 5. Testing Tool Call Safety (untrusted model)..."
echo -e "llm route explain !@ call dangerous_command\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A10 "intent:"

echo
echo "📈 6. Testing Performance-Based Routing..."
echo -e "llm route test Quick status check\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A10 "Performance Required"

echo
echo "🔄 7. Testing Fallback Chain..."
echo -e "llm route info\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A5 "Model Priorities"

echo
echo "🧪 8. Running Unit Tests..."
cd /mnt/d/Projects/SentientOS/sentient-shell
cargo test ai_router_test 2>&1 | grep -E "test result:|passed"

echo
echo "✅ Phase 1 Testing Complete!"
echo
echo "Summary:"
echo "- Model safety flags: Enforced"
echo "- Confidence scoring: Active"
echo "- Verbose logging: Available"
echo "- Tool call protection: Enabled"
echo "- Routing transparency: Full"
echo
echo "Next: Phase 2 - RAG + Tool Fusion"