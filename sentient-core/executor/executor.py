#!/usr/bin/env python3
"""
SentientAct: Autonomous Executor for SentientOS

This module executes plan steps using the RL router and manages
the execution lifecycle including observations, retries, and feedback.
"""

import asyncio
import json
import logging
import time
from typing import Dict, Any, Optional, List, Tuple
from dataclasses import dataclass, field
from enum import Enum
from datetime import datetime
import uuid

# Import planning types
import sys
import os
sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from planner.planner import PlanStep, ExecutionPlan, ActionType

logger = logging.getLogger(__name__)


class ExecutionStatus(Enum):
    """Status of a step execution"""
    PENDING = "pending"
    RUNNING = "running"
    SUCCESS = "success"
    FAILED = "failed"
    SKIPPED = "skipped"
    TIMEOUT = "timeout"


@dataclass
class StepResult:
    """Result of executing a single step"""
    step_id: str
    status: ExecutionStatus
    output: Any = None
    error: Optional[str] = None
    start_time: datetime = field(default_factory=datetime.now)
    end_time: Optional[datetime] = None
    duration_ms: Optional[int] = None
    tool_used: Optional[str] = None
    rl_confidence: Optional[float] = None
    retry_count: int = 0
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "step_id": self.step_id,
            "status": self.status.value,
            "output": self.output,
            "error": self.error,
            "start_time": self.start_time.isoformat(),
            "end_time": self.end_time.isoformat() if self.end_time else None,
            "duration_ms": self.duration_ms,
            "tool_used": self.tool_used,
            "rl_confidence": self.rl_confidence,
            "retry_count": self.retry_count
        }


@dataclass
class ExecutionContext:
    """Context maintained during execution"""
    plan: ExecutionPlan
    results: Dict[str, StepResult] = field(default_factory=dict)
    outputs: Dict[str, Any] = field(default_factory=dict)
    state: Dict[str, Any] = field(default_factory=dict)
    trace_id: str = field(default_factory=lambda: str(uuid.uuid4()))
    
    def get_completed_steps(self) -> List[str]:
        """Get list of successfully completed steps"""
        return [
            step_id for step_id, result in self.results.items()
            if result.status == ExecutionStatus.SUCCESS
        ]
    
    def get_last_output(self) -> Any:
        """Get output from the last completed step"""
        completed = self.get_completed_steps()
        if completed:
            last_step = completed[-1]
            return self.outputs.get(last_step)
        return None


class ToolInterface:
    """Interface for tool execution"""
    
    async def execute_tool(self, tool_name: str, inputs: Dict[str, Any]) -> Dict[str, Any]:
        """Execute a tool and return results"""
        # This would integrate with the actual tool framework
        logger.info(f"Executing tool: {tool_name} with inputs: {inputs}")
        
        # Simulated tool execution
        tool_map = {
            "memory_check": self._memory_check,
            "memory_clean": self._memory_clean,
            "log_fetch": self._log_fetch,
            "log_filter": self._log_filter,
            "disk_check": self._disk_check,
            "cpu_monitor": self._cpu_monitor,
        }
        
        if tool_name in tool_map:
            return await tool_map[tool_name](inputs)
        else:
            return {"error": f"Unknown tool: {tool_name}"}
    
    async def _memory_check(self, inputs: Dict) -> Dict[str, Any]:
        """Simulated memory check"""
        await asyncio.sleep(0.5)  # Simulate work
        return {
            "total_memory": 16384,
            "used_memory": 12288,
            "free_memory": 4096,
            "usage_percent": 75
        }
    
    async def _memory_clean(self, inputs: Dict) -> Dict[str, Any]:
        """Simulated memory cleanup"""
        await asyncio.sleep(1.0)  # Simulate work
        return {
            "freed_memory": 2048,
            "success": True
        }
    
    async def _log_fetch(self, inputs: Dict) -> Dict[str, Any]:
        """Simulated log fetching"""
        await asyncio.sleep(0.3)
        return {
            "log_entries": [
                {"timestamp": "2024-01-01T10:00:00", "level": "ERROR", "message": "Connection timeout"},
                {"timestamp": "2024-01-01T10:05:00", "level": "INFO", "message": "Service started"},
                {"timestamp": "2024-01-01T10:10:00", "level": "ERROR", "message": "Database error"},
            ],
            "count": 3
        }
    
    async def _log_filter(self, inputs: Dict) -> Dict[str, Any]:
        """Simulated log filtering"""
        await asyncio.sleep(0.2)
        filter_pattern = inputs.get("filter", "error")
        # Filter previous output
        return {
            "filtered_entries": [
                {"timestamp": "2024-01-01T10:00:00", "level": "ERROR", "message": "Connection timeout"},
                {"timestamp": "2024-01-01T10:10:00", "level": "ERROR", "message": "Database error"},
            ],
            "count": 2
        }
    
    async def _disk_check(self, inputs: Dict) -> Dict[str, Any]:
        """Simulated disk check"""
        await asyncio.sleep(0.4)
        return {
            "total_disk": 512000,
            "used_disk": 256000,
            "free_disk": 256000,
            "usage_percent": 50
        }
    
    async def _cpu_monitor(self, inputs: Dict) -> Dict[str, Any]:
        """Simulated CPU monitoring"""
        await asyncio.sleep(0.6)
        return {
            "cpu_percent": 45,
            "core_count": 8,
            "load_average": [2.5, 2.1, 1.8]
        }


class RLRouter:
    """Interface to the RL routing system"""
    
    async def select_tool(self, step: PlanStep, context: Dict[str, Any]) -> Tuple[str, float]:
        """Select tool using RL policy"""
        # This would call the actual RL router
        # For now, return tool hint with confidence
        if step.tool_hint:
            return step.tool_hint, 0.85
        
        # Fallback heuristic
        tool_map = {
            "check": "memory_check",
            "clean": "memory_clean",
            "fetch": "log_fetch",
            "filter": "log_filter",
            "monitor": "cpu_monitor"
        }
        
        for keyword, tool in tool_map.items():
            if keyword in step.description.lower():
                return tool, 0.6
        
        return "unknown", 0.0


class SentientExecutor:
    """Main executor for autonomous plan execution"""
    
    def __init__(self, 
                 rl_router: Optional[RLRouter] = None,
                 tool_interface: Optional[ToolInterface] = None,
                 max_parallel: int = 3,
                 step_timeout: int = 30):
        self.rl_router = rl_router or RLRouter()
        self.tool_interface = tool_interface or ToolInterface()
        self.max_parallel = max_parallel
        self.step_timeout = step_timeout
        self.execution_semaphore = asyncio.Semaphore(max_parallel)
    
    async def execute_plan(self, plan: ExecutionPlan) -> ExecutionContext:
        """Execute a complete plan autonomously"""
        logger.info(f"Starting execution of plan: {plan.plan_id}")
        context = ExecutionContext(plan=plan)
        
        # Main execution loop
        while True:
            # Get executable steps
            executable = plan.get_executable_steps(context.get_completed_steps())
            
            if not executable:
                # Check if we're done or stuck
                if self._is_plan_complete(context):
                    logger.info("Plan execution completed")
                    break
                elif self._is_plan_stuck(context):
                    logger.warning("Plan execution stuck - no executable steps")
                    break
            
            # Execute available steps (up to max_parallel)
            tasks = []
            for step in executable[:self.max_parallel]:
                task = asyncio.create_task(self._execute_step(step, context))
                tasks.append(task)
            
            # Wait for at least one to complete
            if tasks:
                done, pending = await asyncio.wait(tasks, return_when=asyncio.FIRST_COMPLETED)
                
                # Cancel remaining tasks if needed
                for task in pending:
                    task.cancel()
        
        return context
    
    async def _execute_step(self, step: PlanStep, context: ExecutionContext) -> StepResult:
        """Execute a single step"""
        async with self.execution_semaphore:
            logger.info(f"Executing step: {step.step_id} - {step.description}")
            
            result = StepResult(
                step_id=step.step_id,
                status=ExecutionStatus.RUNNING,
                start_time=datetime.now()
            )
            
            try:
                # Update context
                context.results[step.step_id] = result
                
                # Route based on action type
                if step.action_type == ActionType.EXECUTE:
                    await self._execute_tool_step(step, context, result)
                elif step.action_type == ActionType.QUERY:
                    await self._execute_query_step(step, context, result)
                elif step.action_type == ActionType.CONDITION:
                    await self._execute_condition_step(step, context, result)
                elif step.action_type == ActionType.STOP:
                    result.status = ExecutionStatus.SUCCESS
                    result.output = "Goal completed"
                else:
                    result.status = ExecutionStatus.SKIPPED
                    result.output = f"Unsupported action type: {step.action_type}"
                
            except asyncio.TimeoutError:
                result.status = ExecutionStatus.TIMEOUT
                result.error = f"Step timed out after {self.step_timeout}s"
                logger.error(f"Step {step.step_id} timed out")
                
            except Exception as e:
                result.status = ExecutionStatus.FAILED
                result.error = str(e)
                logger.error(f"Step {step.step_id} failed: {e}")
            
            finally:
                result.end_time = datetime.now()
                result.duration_ms = int((result.end_time - result.start_time).total_seconds() * 1000)
                context.results[step.step_id] = result
                
                # Log execution trace
                await self._log_execution_trace(step, result, context)
            
            return result
    
    async def _execute_tool_step(self, step: PlanStep, context: ExecutionContext, result: StepResult):
        """Execute a tool-based step"""
        # Select tool using RL
        tool_name, confidence = await self.rl_router.select_tool(step, context.state)
        result.tool_used = tool_name
        result.rl_confidence = confidence
        
        if confidence < 0.3:
            result.status = ExecutionStatus.FAILED
            result.error = f"No suitable tool found (confidence: {confidence})"
            return
        
        # Prepare inputs
        inputs = dict(step.inputs)
        
        # Chain outputs from dependencies
        for dep_id in step.dependencies:
            if dep_id in context.outputs:
                inputs[f"{dep_id}_output"] = context.outputs[dep_id]
        
        # Execute tool with timeout
        try:
            tool_result = await asyncio.wait_for(
                self.tool_interface.execute_tool(tool_name, inputs),
                timeout=self.step_timeout
            )
            
            if "error" in tool_result:
                result.status = ExecutionStatus.FAILED
                result.error = tool_result["error"]
            else:
                result.status = ExecutionStatus.SUCCESS
                result.output = tool_result
                context.outputs[step.step_id] = tool_result
                
        except asyncio.TimeoutError:
            raise  # Re-raise to be caught by outer handler
    
    async def _execute_query_step(self, step: PlanStep, context: ExecutionContext, result: StepResult):
        """Execute a query/LLM step"""
        # This would call the LLM for analysis/summarization
        result.status = ExecutionStatus.SUCCESS
        result.output = {"summary": "Simulated query result"}
        context.outputs[step.step_id] = result.output
    
    async def _execute_condition_step(self, step: PlanStep, context: ExecutionContext, result: StepResult):
        """Execute a conditional step"""
        # Evaluate condition based on inputs and previous outputs
        threshold = step.inputs.get("memory_threshold", 80)
        
        # Get previous output
        prev_output = context.get_last_output()
        if prev_output and "usage_percent" in prev_output:
            needs_action = prev_output["usage_percent"] > threshold
            result.output = {"condition_met": needs_action, "threshold": threshold}
            result.status = ExecutionStatus.SUCCESS
            context.outputs[step.step_id] = result.output
        else:
            result.status = ExecutionStatus.FAILED
            result.error = "Missing required input for condition evaluation"
    
    def _is_plan_complete(self, context: ExecutionContext) -> bool:
        """Check if plan execution is complete"""
        # Plan is complete if there's a successful STOP step
        for step in context.plan.steps:
            if step.action_type == ActionType.STOP:
                result = context.results.get(step.step_id)
                return result and result.status == ExecutionStatus.SUCCESS
        return False
    
    def _is_plan_stuck(self, context: ExecutionContext) -> bool:
        """Check if plan execution is stuck"""
        # Check if all remaining steps have failed
        incomplete_steps = [
            s for s in context.plan.steps 
            if s.step_id not in context.get_completed_steps()
        ]
        
        failed_count = sum(
            1 for s in incomplete_steps
            if s.step_id in context.results and 
            context.results[s.step_id].status == ExecutionStatus.FAILED
        )
        
        return failed_count == len(incomplete_steps) and failed_count > 0
    
    async def _log_execution_trace(self, step: PlanStep, result: StepResult, context: ExecutionContext):
        """Log execution trace for RL training"""
        trace_entry = {
            "trace_id": context.trace_id,
            "plan_id": context.plan.plan_id,
            "step_id": step.step_id,
            "timestamp": datetime.now().isoformat(),
            "action_type": step.action_type.value,
            "tool_used": result.tool_used,
            "rl_confidence": result.rl_confidence,
            "status": result.status.value,
            "duration_ms": result.duration_ms,
            "error": result.error,
            "context_state": context.state
        }
        
        # This would append to the RL trace log
        logger.debug(f"Execution trace: {json.dumps(trace_entry)}")


async def demo_execution():
    """Demonstrate autonomous execution"""
    from planner.planner import SentientPlanner
    
    # Create planner and executor
    planner = SentientPlanner()
    executor = SentientExecutor()
    
    # Create a plan
    goal = "Check memory usage and clean if needed"
    plan = planner.plan_goal(goal)
    
    print(f"\n🎯 Goal: {goal}")
    print(f"📋 Plan: {plan.plan_id}")
    for step in plan.steps:
        print(f"  - {step.step_id}: {step.description}")
    
    # Execute the plan
    print("\n🚀 Starting execution...")
    context = await executor.execute_plan(plan)
    
    # Show results
    print("\n📊 Execution Results:")
    for step_id, result in context.results.items():
        status_emoji = "✅" if result.status == ExecutionStatus.SUCCESS else "❌"
        print(f"  {status_emoji} {step_id}: {result.status.value}")
        if result.output:
            print(f"     Output: {json.dumps(result.output, indent=2)}")
        if result.error:
            print(f"     Error: {result.error}")


if __name__ == "__main__":
    asyncio.run(demo_execution())