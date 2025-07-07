#!/usr/bin/env python3
"""
Phase 6: Validate Python Dependencies
"""

import sys
import importlib

dependencies = [
    ("torch", "PyTorch"),
    ("psutil", "System monitoring"),
    ("yaml", "YAML parsing"),
    ("toml", "TOML parsing"),
    ("numpy", "Numerical computing"),
    ("asyncio", "Async operations"),
    ("aiofiles", "Async file operations"),
    ("pandas", "Data analysis"),
    ("sklearn", "Machine learning"),
    ("seaborn", "Statistical plotting"),
    ("matplotlib", "Plotting library")
]

print("🔍 Validating Python Dependencies")
print("=" * 50)

all_passed = True
missing = []

for module_name, description in dependencies:
    try:
        if module_name == "yaml":
            importlib.import_module("yaml")
        elif module_name == "sklearn":
            importlib.import_module("sklearn")
        else:
            importlib.import_module(module_name)
        print(f"[PASS] {module_name:<15} - {description}")
    except ImportError:
        print(f"[FAIL] {module_name:<15} - {description}")
        missing.append(module_name)
        all_passed = False

if not all_passed:
    print(f"\n❌ Missing dependencies: {', '.join(missing)}")
    print("\nTo install missing dependencies:")
    print("pip install -r requirements.txt")
    sys.exit(1)
else:
    print("\n✅ All Python dependencies validated!")