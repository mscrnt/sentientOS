#!/usr/bin/env python3
"""
Main entry point for SentientOS autonomous execution
"""

import argparse
import asyncio
import json
import logging
import sys
from pathlib import Path
from datetime import datetime

# Add sentient-core to path
sys.path.insert(0, str(Path(__file__).parent))

from planner.planner import SentientPlanner
from executor.executor import SentientExecutor
from executor.sentient_loop import SentientLoop, LoopConfig
from executor.guardrails import GuardrailSystem, ResourceLimits, SafetyPolicy

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


def setup_argparse():
    """Setup command line arguments"""
    parser = argparse.ArgumentParser(
        description="SentientOS Autonomous Goal Execution"
    )
    
    parser.add_argument(
        "--goal", "-g",
        type=str,
        required=True,
        help="Natural language goal to achieve"
    )
    
    parser.add_argument(
        "--max-steps", "-m",
        type=int,
        default=50,
        help="Maximum execution steps (default: 50)"
    )
    
    parser.add_argument(
        "--verbose", "-v",
        action="store_true",
        help="Show detailed execution information"
    )
    
    parser.add_argument(
        "--dry-run", "-d",
        action="store_true",
        help="Show plan without executing"
    )
    
    parser.add_argument(
        "--save-trace", "-t",
        action="store_true",
        help="Save execution trace for analysis"
    )
    
    parser.add_argument(
        "--config", "-c",
        type=str,
        help="Path to configuration file"
    )
    
    return parser


async def execute_goal(goal: str, args):
    """Execute a goal autonomously"""
    
    # Configure based on arguments
    if args.verbose:
        logging.getLogger().setLevel(logging.DEBUG)
    
    # Create configuration
    loop_config = LoopConfig(
        max_steps=args.max_steps,
        max_replanning=3,
        max_duration_seconds=300,
        confidence_threshold=0.5
    )
    
    # Create components
    planner = SentientPlanner()
    executor = SentientExecutor()
    guardrails = GuardrailSystem()
    
    # Create control loop
    loop = SentientLoop(
        planner=planner,
        executor=executor,
        config=loop_config
    )
    
    # Register guardrail callback
    def on_violation(violation):
        logger.warning(f"Guardrail: {violation.message}")
    
    guardrails.register_callback(on_violation)
    
    # Dry run - just show plan
    if args.dry_run:
        plan = planner.plan_goal(goal)
        print("\n📋 Execution Plan:")
        print(f"Goal: {goal}")
        print(f"Steps ({len(plan.steps)}):")
        
        for i, step in enumerate(plan.steps, 1):
            deps = f" (depends on: {', '.join(step.dependencies)})" if step.dependencies else ""
            print(f"  {i}. [{step.action_type.value}] {step.description}{deps}")
            if step.tool_hint:
                print(f"     Tool: {step.tool_hint}")
        
        return {"status": "dry_run", "plan": plan.to_dict()}
    
    # Execute goal
    logger.info(f"Starting execution for goal: {goal}")
    
    try:
        # Run the control loop
        result = await loop.run_goal(goal)
        
        # Display results
        print("\n📊 Execution Results:")
        print(f"  Status: {result['status']}")
        
        if result.get('metrics'):
            metrics = result['metrics']
            print(f"  Steps: {metrics['total_steps']} total, {metrics['successful_steps']} successful")
            print(f"  Success Rate: {metrics['success_rate']:.1%}")
            print(f"  Duration: {metrics['duration_ms']/1000:.1f}s")
            
            if metrics['replanning_count'] > 0:
                print(f"  Replanning: {metrics['replanning_count']} times")
        
        if result.get('reward') is not None:
            print(f"  Reward: {result['reward']:.2f}")
        
        # Save trace if requested
        if args.save_trace:
            trace_path = Path("logs/sentient_trace.jsonl")
            trace_path.parent.mkdir(exist_ok=True)
            
            with open(trace_path, 'a') as f:
                trace_entry = {
                    "timestamp": datetime.now().isoformat(),
                    "goal": goal,
                    "result": result
                }
                f.write(json.dumps(trace_entry) + '\n')
            
            print(f"\n💾 Trace saved to: {trace_path}")
        
        return result
        
    except Exception as e:
        logger.error(f"Execution failed: {e}")
        return {"status": "error", "error": str(e)}


def main():
    """Main entry point"""
    parser = setup_argparse()
    args = parser.parse_args()
    
    # Print header
    print("🧠 SentientOS Autonomous Execution Engine")
    print("=" * 50)
    
    # Run async execution
    result = asyncio.run(execute_goal(args.goal, args))
    
    # Exit with appropriate code
    if result['status'] in ['succeeded', 'dry_run']:
        sys.exit(0)
    else:
        sys.exit(1)


if __name__ == "__main__":
    main()