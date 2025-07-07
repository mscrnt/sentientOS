#!/usr/bin/env python3
"""
Validate configuration files
"""

import toml
import yaml
import json
from pathlib import Path

print("🔍 Validating Configuration Files")
print("=" * 50)

# Check TOML files
toml_files = [
    ("config/router_config.toml", "Router configuration"),
    ("config/tool_registry.toml", "Tool registry")
]

for file_path, description in toml_files:
    try:
        with open(file_path) as f:
            config = toml.load(f)
        
        # Specific validations
        if "router_config" in file_path:
            assert "router" in config, "Missing 'router' section"
            assert "models" in config, "Missing 'models' section"
            assert len(config["models"]) > 0, "No models defined"
            print(f"[PASS] {file_path:<30} - {description} (found {len(config['models'])} models)")
        elif "tool_registry" in file_path:
            assert "tools" in config, "Missing 'tools' section"
            assert len(config["tools"]) >= 3, "Less than 3 tools defined"
            print(f"[PASS] {file_path:<30} - {description} (found {len(config['tools'])} tools)")
    except Exception as e:
        print(f"[FAIL] {file_path:<30} - {description}: {str(e)}")

# Check YAML files
yaml_files = [
    ("config/conditions.yaml", "Tool conditions"),
    ("config/rewards.yaml", "Reward configuration")
]

for file_path, description in yaml_files:
    try:
        with open(file_path) as f:
            config = yaml.safe_load(f)
        
        if "conditions" in file_path:
            assert "conditions" in config, "Missing 'conditions' section"
            print(f"[PASS] {file_path:<30} - {description} (found {len(config['conditions'])} conditions)")
        elif "rewards" in file_path:
            assert "rewards" in config, "Missing 'rewards' section"
            print(f"[PASS] {file_path:<30} - {description}")
    except Exception as e:
        print(f"[FAIL] {file_path:<30} - {description}: {str(e)}")

print("\n✅ Configuration validation complete!")