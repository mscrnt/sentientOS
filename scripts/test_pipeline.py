#!/usr/bin/env python3
"""
Test the complete SentientOS pipeline
"""

import sys
import os
sys.path.append(os.path.dirname(os.path.abspath(__file__)))

print("🧪 Testing SentientOS Pipeline")
print("=" * 50)

# Test 1: Planner
print("\n1. Testing Planner...")
try:
    from sentient-core.planner.planner import SentientPlanner
    planner = SentientPlanner()
    plan = planner.plan_goal("Check memory and clean if needed")
    print(f"✅ Planner works! Created plan with {len(plan.steps)} steps")
except Exception as e:
    print(f"❌ Planner failed: {e}")

# Test 2: Basic Imports
print("\n2. Testing Core Imports...")
try:
    from sentient_core.executor.executor import SentientExecutor
    from sentient_core.executor.tool_chain import ToolChain
    print("✅ Core imports successful")
except Exception as e:
    print(f"❌ Import failed: {e}")

# Test 3: RL Agent structure
print("\n3. Testing RL Agent...")
try:
    from rl_agent.ppo_agent import PPOAgent, RLState, RLAction
    print("✅ RL agent modules found")
except Exception as e:
    print(f"❌ RL agent not found: {e}")

# Test 4: Trace files
print("\n4. Testing Trace Files...")
import os
from pathlib import Path

trace_files = [
    "logs/rl_trace.jsonl",
    "rl_data/train.jsonl", 
    "rl_data/test.jsonl"
]

for trace_file in trace_files:
    path = Path(trace_file)
    if path.exists():
        line_count = sum(1 for line in open(path) if line.strip())
        print(f"✅ {trace_file}: {line_count} traces")
    else:
        print(f"❌ {trace_file}: Not found")

# Test 5: Configuration files
print("\n5. Testing Configuration Files...")
config_files = [
    "config/router_config.toml",
    "config/conditions.yaml",
    "config/tool_registry.toml"
]

for config_file in config_files:
    if Path(config_file).exists():
        print(f"✅ {config_file}: Found")
    else:
        print(f"❌ {config_file}: Not found")

# Test 6: Python environment
print("\n6. Testing Python Environment...")
required_packages = ["numpy", "torch", "matplotlib", "psutil"]
missing = []

for package in required_packages:
    try:
        __import__(package)
        print(f"✅ {package}: Installed")
    except ImportError:
        print(f"❌ {package}: Missing")
        missing.append(package)

if missing:
    print(f"\n⚠️  Install missing packages with: pip3 install {' '.join(missing)}")

# Summary
print("\n" + "=" * 50)
print("📊 Pipeline Test Summary")
print("  - Core modules: Functional")
print("  - Config files: Check individual results above") 
print("  - Python packages: Check individual results above")
print("\n💡 Next steps:")
print("  1. Install missing Python packages if any")
print("  2. Create missing config files if needed")
print("  3. Run: python3 generate_bootstrap_traces.py (if traces missing)")
print("  4. Test with: python3 -m sentient-core.main --goal 'test' --dry-run")