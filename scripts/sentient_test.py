#!/usr/bin/env python3
"""
Direct test of sentient goal execution without Rust shell
"""

import sys
import asyncio
import argparse
from pathlib import Path

# Add sentient-core to path
sys.path.insert(0, str(Path(__file__).parent / 'sentient-core'))

from planner.planner import SentientPlanner
from executor.executor import SentientExecutor
from executor.sentient_loop import SentientLoop
from executor.guardrails import GuardrailSystem

async def run_goal(goal: str, dry_run: bool = False, verbose: bool = False):
    """Execute a sentient goal"""
    print(f"🎯 Executing goal: {goal}")
    print("=" * 60)
    
    # Initialize components
    planner = SentientPlanner()
    executor = SentientExecutor()
    guardrails = GuardrailSystem()
    
    # Create sentient loop
    loop = SentientLoop(
        planner=planner,
        executor=executor
    )
    
    # Run the goal
    try:
        result = await loop.run_goal(goal)
        
        print("\n✅ Goal execution completed!")
        print(f"Status: {result.get('status', 'unknown')}")
        print(f"Total steps: {result.get('total_steps', 0)}")
        
        if verbose and 'execution_trace' in result:
            print("\nExecution trace:")
            for step in result['execution_trace']:
                print(f"  - {step}")
                
        return result
        
    except Exception as e:
        print(f"\n❌ Goal execution failed: {e}")
        return None

def main():
    parser = argparse.ArgumentParser(description='Test SentientOS goal execution')
    parser.add_argument('goal', help='Goal to execute')
    parser.add_argument('--dry-run', action='store_true', help='Perform dry run only')
    parser.add_argument('--verbose', '-v', action='store_true', help='Verbose output')
    
    args = parser.parse_args()
    
    # Run the goal
    asyncio.run(run_goal(args.goal, args.dry_run, args.verbose))

if __name__ == '__main__':
    main()