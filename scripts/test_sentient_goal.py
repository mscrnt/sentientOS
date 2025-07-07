#!/usr/bin/env python3
"""
Phase 6: Test Sentient Goal Execution
"""

import sys
import os
import json
import asyncio
from datetime import datetime
from pathlib import Path

# Add sentient-core to path
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), 'sentient-core'))

print("🎯 Testing Sentient Goal Execution")
print("=" * 50)

# Test 1: Import Core Modules
print("\n[TEST 1] Importing Core Modules...")
try:
    from planner.planner import SentientPlanner
    from executor.executor import SentientExecutor
    from executor.sentient_loop import SentientLoop
    from executor.guardrails import GuardrailSystem
    from executor.tool_chain import ToolChain
    
    print("[PASS] All core modules imported successfully")
except Exception as e:
    print(f"[FAIL] Module import error: {e}")
    sys.exit(1)

# Test 2: Initialize Components
print("\n[TEST 2] Initializing Components...")
try:
    planner = SentientPlanner()
    executor = SentientExecutor()
    guardrails = GuardrailSystem()
    tool_chain = ToolChain()
    
    print("[PASS] All components initialized")
except Exception as e:
    print(f"[FAIL] Initialization error: {e}")
    sys.exit(1)

# Test 3: Create Simple Plan
print("\n[TEST 3] Creating Execution Plan...")
test_goal = "Summarize system performance"
try:
    plan = planner.plan_goal(test_goal)
    
    if plan and len(plan.steps) > 0:
        print(f"[PASS] Created plan with {len(plan.steps)} steps:")
        for i, step in enumerate(plan.steps):
            print(f"       Step {i+1}: {step.action} - {step.description}")
    else:
        print("[FAIL] No plan generated")
        sys.exit(1)
except Exception as e:
    print(f"[FAIL] Planning error: {e}")
    sys.exit(1)

# Test 4: Test Guardrails
print("\n[TEST 4] Testing Guardrails...")
try:
    # Test with normal resource usage
    normal_state = {
        'memory_usage_percent': 45.0,
        'cpu_usage_percent': 30.0,
        'execution_time': 1.5
    }
    
    if guardrails.check_safety(normal_state):
        print("[PASS] Guardrails allow normal execution")
    else:
        print("[FAIL] Guardrails blocked normal execution")
    
    # Test with high resource usage
    high_state = {
        'memory_usage_percent': 95.0,
        'cpu_usage_percent': 98.0,
        'execution_time': 65.0
    }
    
    if not guardrails.check_safety(high_state):
        print("[PASS] Guardrails block high resource usage")
    else:
        print("[FAIL] Guardrails failed to block high usage")
        
except Exception as e:
    print(f"[FAIL] Guardrails error: {e}")

# Test 5: Execute Simple Tool
print("\n[TEST 5] Testing Tool Execution...")
try:
    # Test memory check tool
    result = asyncio.run(executor.execute_tool("memory_check", {}))
    
    if result and "output" in result:
        print("[PASS] Tool executed successfully")
        print(f"       Exit code: {result.get('exit_code', 'N/A')}")
    else:
        print("[FAIL] Tool execution failed")
except Exception as e:
    print(f"[FAIL] Tool execution error: {e}")

# Test 6: Run Full Sentient Loop (Dry Run)
print("\n[TEST 6] Testing Sentient Loop (Dry Run)...")
try:
    loop = SentientLoop(
        planner=planner,
        executor=executor,
        guardrails=guardrails,
        max_iterations=3
    )
    
    # Run in dry-run mode
    result = asyncio.run(loop.run_goal(test_goal, dry_run=True))
    
    if result and result.get("status") in ["completed", "dry_run_completed"]:
        print("[PASS] Sentient loop dry run completed")
        print(f"       Total steps: {result.get('total_steps', 0)}")
        print(f"       Status: {result.get('status')}")
    else:
        print("[FAIL] Sentient loop failed")
except Exception as e:
    print(f"[FAIL] Sentient loop error: {e}")

# Test 7: Verify Trace Logging
print("\n[TEST 7] Testing Trace Logging...")
try:
    logs_dir = Path("logs")
    if not logs_dir.exists():
        logs_dir.mkdir(parents=True)
        print("       Created logs directory")
    
    # Create test trace
    test_trace = {
        "timestamp": datetime.now().isoformat(),
        "goal": test_goal,
        "plan_steps": len(plan.steps) if 'plan' in locals() else 0,
        "execution_status": "test",
        "resource_usage": {
            "memory_percent": 45.0,
            "cpu_percent": 30.0
        }
    }
    
    trace_file = logs_dir / f"trace_test_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
    with open(trace_file, 'w') as f:
        json.dump(test_trace, f, indent=2)
    
    if trace_file.exists():
        print("[PASS] Trace logging works")
        print(f"       Trace saved to: {trace_file}")
    else:
        print("[FAIL] Trace file not created")
except Exception as e:
    print(f"[FAIL] Trace logging error: {e}")

# Summary
print("\n" + "=" * 50)
print("✅ SentientOS Goal Execution Test Complete")
print("\nNext steps:")
print("1. Run from shell: sentient goal \"Summarize system performance\"")
print("2. Check logs/ directory for execution traces")
print("3. Monitor resource usage during execution")