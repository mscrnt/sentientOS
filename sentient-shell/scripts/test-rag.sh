#!/bin/bash
# Test RAG functionality

echo "=== Testing RAG System ==="
echo

# Build if needed
echo "Building sentient-shell..."
cd /mnt/d/Projects/SentientOS/sentient-shell
cargo build 2>&1 | tail -5

echo
echo "1. Initializing RAG system..."
echo -e "rag init\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A2 "RAG"

echo
echo "2. Indexing documentation..."
echo -e "rag index docs\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A2 "Index"

echo
echo "3. Testing basic query..."
echo -e "rag query How does HiveFix recovery work?\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A10 "Query:"

echo
echo "4. Testing with prefix command..."
echo -e "!@ rag query What is the boot sequence?\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A10 "Query:"

echo
echo "5. Checking statistics..."
echo -e "rag stats\nexit" | ./target/debug/sentient-shell 2>&1 | grep -A5 "Statistics"

echo
echo "✅ RAG tests complete"