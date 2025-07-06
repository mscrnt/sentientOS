#!/bin/bash

# Test Ollama streaming
curl -X POST http://192.168.69.197:11434/api/generate \
  -H "Content-Type: application/json" \
  -d '{
    "model": "llama3.2:latest",
    "prompt": "Improve this Rust dependency check for a service manager:\n\n```rust\nfn check_dependencies(&self, config: &ServiceConfig) -> bool {\n    for dep in &config.dependencies {\n        if let Some(info) = self.process_manager.get_service_info(dep) {\n            if info.status != ServiceStatus::Running {\n                return false;\n            }\n        } else {\n            return false;\n        }\n    }\n    true\n}\n```\n\nProvide: 1) Circular dependency detection, 2) Topological sort for start order",
    "stream": true,
    "options": {
      "temperature": 0.3,
      "num_predict": 600
    }
  }' \
  --no-buffer \
  2>/dev/null | while IFS= read -r line; do
    echo "$line" | jq -r '.response' 2>/dev/null | tr -d '\n'
done
echo # New line at end