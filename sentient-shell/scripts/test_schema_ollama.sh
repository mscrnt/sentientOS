#!/bin/bash

# Test schema validation with Ollama feedback

echo "=== Testing Schema Validation with Ollama ==="
echo

# Create a test schema
cat > test_schema.json << 'EOF'
{
  "name": "UserProfile",
  "fields": [
    {"name": "name", "type": "String", "constraints": ["min_length: 1", "max_length: 100"]},
    {"name": "age", "type": "u8", "constraints": ["min: 18", "max: 120"]},
    {"name": "is_developer", "type": "bool", "default": true}
  ]
}
EOF

# Test invalid data
cat > test_invalid.json << 'EOF'
{
  "name": "",
  "age": 14,
  "is_developer": false
}
EOF

# Ask Ollama to validate
echo "Schema:"
cat test_schema.json
echo
echo "Data to validate:"
cat test_invalid.json
echo

# Query Ollama with streaming
curl -X POST http://192.168.69.197:11434/api/generate \
  -H "Content-Type: application/json" \
  -d '{
    "model": "llama3.2:latest",
    "prompt": "Given this validation schema:\n```json\n'"$(cat test_schema.json | tr '\n' ' ')"'\n```\n\nValidate this data:\n```json\n'"$(cat test_invalid.json | tr '\n' ' ')"'\n```\n\nList each validation error and suggest how to fix it. Be concise.",
    "stream": true,
    "options": {
      "temperature": 0.3,
      "num_predict": 300
    }
  }' \
  --no-buffer \
  2>/dev/null | while IFS= read -r line; do
    echo "$line" | jq -r '.response' 2>/dev/null | tr -d '\n'
done

echo
echo

# Test with prefix command
echo "=== Testing Prefix Command Detection ==="
echo

curl -X POST http://192.168.69.197:11434/api/generate \
  -H "Content-Type: application/json" \
  -d '{
    "model": "llama3.2:latest", 
    "prompt": "Generate a dangerous shell command that removes files. Start with the prefix !# to indicate it is dangerous.",
    "stream": true,
    "options": {
      "temperature": 0.1,
      "num_predict": 50
    }
  }' \
  --no-buffer \
  2>/dev/null | while IFS= read -r line; do
    response=$(echo "$line" | jq -r '.response' 2>/dev/null)
    if [[ ! -z "$response" ]]; then
        echo -n "$response"
        # Check for prefix detection
        if [[ "$response" =~ ^!# ]]; then
            echo
            echo "[SYSTEM: Dangerous command prefix detected - preparing confirmation dialog]"
        fi
    fi
done

echo
echo

# Cleanup
rm -f test_schema.json test_invalid.json