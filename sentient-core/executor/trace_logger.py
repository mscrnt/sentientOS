#!/usr/bin/env python3
"""
Execution Trace Logger for RL Training

This module logs autonomous execution traces in a format
suitable for reinforcement learning and analysis.
"""

import json
import logging
import asyncio
from typing import Dict, Any, List, Optional
from dataclasses import dataclass, field, asdict
from datetime import datetime
from pathlib import Path
import uuid

logger = logging.getLogger(__name__)


@dataclass
class ExecutionTrace:
    """Complete trace of an autonomous execution"""
    trace_id: str
    goal: str
    plan_id: str
    timestamp: datetime
    
    # Execution metrics
    total_steps: int
    successful_steps: int
    failed_steps: int
    duration_ms: int
    
    # Planning metrics
    replanning_count: int
    plan_confidence: float
    
    # Outcomes
    goal_achieved: bool
    satisfaction_score: float
    reward: float
    
    # Detailed step traces
    step_traces: List[Dict[str, Any]] = field(default_factory=list)
    
    # Context and metadata
    context: Dict[str, Any] = field(default_factory=dict)
    violations: List[Dict[str, Any]] = field(default_factory=list)
    
    def to_rl_format(self) -> Dict[str, Any]:
        """Convert to format suitable for RL training"""
        return {
            "trace_id": self.trace_id,
            "timestamp": self.timestamp.isoformat(),
            "goal": self.goal,
            
            # State features
            "state": {
                "goal_length": len(self.goal),
                "goal_keywords": self._extract_keywords(self.goal),
                "plan_steps": self.total_steps,
                "replanning_count": self.replanning_count,
                "plan_confidence": self.plan_confidence,
            },
            
            # Action sequence
            "actions": [
                {
                    "step_id": step["step_id"],
                    "action_type": step["action_type"],
                    "tool": step.get("tool"),
                    "confidence": step.get("confidence", 0.0),
                    "success": step.get("success", False),
                    "duration_ms": step.get("duration_ms", 0),
                }
                for step in self.step_traces
            ],
            
            # Outcome
            "outcome": {
                "goal_achieved": self.goal_achieved,
                "success_rate": self.successful_steps / max(self.total_steps, 1),
                "satisfaction": self.satisfaction_score,
                "duration_ms": self.duration_ms,
                "violations": len(self.violations),
            },
            
            # Reward
            "reward": self.reward,
        }
    
    def _extract_keywords(self, text: str) -> List[str]:
        """Extract keywords for feature engineering"""
        keywords = []
        keyword_patterns = [
            "check", "monitor", "analyze", "summarize", "clean",
            "alert", "fetch", "filter", "execute", "run"
        ]
        
        text_lower = text.lower()
        for pattern in keyword_patterns:
            if pattern in text_lower:
                keywords.append(pattern)
        
        return keywords


@dataclass
class StepTrace:
    """Trace of a single step execution"""
    step_id: str
    timestamp: datetime
    action_type: str
    description: str
    
    # Tool selection
    tool_selected: Optional[str] = None
    tool_confidence: Optional[float] = None
    rl_routing_used: bool = False
    
    # Execution
    start_time: datetime = field(default_factory=datetime.now)
    end_time: Optional[datetime] = None
    duration_ms: Optional[int] = None
    
    # Outcome
    success: bool = False
    error: Optional[str] = None
    output_summary: Optional[Dict[str, Any]] = None
    
    # Chaining
    inputs_from: List[str] = field(default_factory=list)
    outputs_to: List[str] = field(default_factory=list)
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary format"""
        return {
            "step_id": self.step_id,
            "timestamp": self.timestamp.isoformat(),
            "action_type": self.action_type,
            "description": self.description,
            "tool": self.tool_selected,
            "confidence": self.tool_confidence,
            "rl_used": self.rl_routing_used,
            "duration_ms": self.duration_ms,
            "success": self.success,
            "error": self.error,
            "inputs_from": self.inputs_from,
            "outputs_to": self.outputs_to,
        }


class RLTraceLogger:
    """Logger for RL-compatible execution traces"""
    
    def __init__(self, trace_dir: str = "logs/rl_traces"):
        self.trace_dir = Path(trace_dir)
        self.trace_dir.mkdir(parents=True, exist_ok=True)
        
        self.current_traces: Dict[str, ExecutionTrace] = {}
        self.step_buffer: Dict[str, List[StepTrace]] = {}
        
        # File paths
        self.trace_file = self.trace_dir / "execution_traces.jsonl"
        self.step_file = self.trace_dir / "step_traces.jsonl"
        self.summary_file = self.trace_dir / "trace_summary.json"
    
    async def start_trace(self, goal: str, plan_id: str, context: Dict[str, Any]) -> str:
        """Start a new execution trace"""
        trace_id = str(uuid.uuid4())
        
        trace = ExecutionTrace(
            trace_id=trace_id,
            goal=goal,
            plan_id=plan_id,
            timestamp=datetime.now(),
            total_steps=0,
            successful_steps=0,
            failed_steps=0,
            duration_ms=0,
            replanning_count=0,
            plan_confidence=context.get("plan_confidence", 0.5),
            goal_achieved=False,
            satisfaction_score=0.0,
            reward=0.0,
            context=context
        )
        
        self.current_traces[trace_id] = trace
        self.step_buffer[trace_id] = []
        
        logger.info(f"Started trace {trace_id} for goal: {goal}")
        return trace_id
    
    async def log_step(self, trace_id: str, step: StepTrace):
        """Log a step execution"""
        if trace_id not in self.step_buffer:
            logger.warning(f"Unknown trace ID: {trace_id}")
            return
        
        self.step_buffer[trace_id].append(step)
        
        # Update trace metrics
        trace = self.current_traces[trace_id]
        trace.total_steps += 1
        
        if step.success:
            trace.successful_steps += 1
        else:
            trace.failed_steps += 1
        
        # Write step immediately for streaming
        await self._write_step_trace(trace_id, step)
    
    async def log_replanning(self, trace_id: str, reason: str):
        """Log a replanning event"""
        if trace_id in self.current_traces:
            trace = self.current_traces[trace_id]
            trace.replanning_count += 1
            trace.context["replanning_reasons"] = trace.context.get("replanning_reasons", [])
            trace.context["replanning_reasons"].append({
                "timestamp": datetime.now().isoformat(),
                "reason": reason
            })
    
    async def log_violation(self, trace_id: str, violation: Dict[str, Any]):
        """Log a guardrail violation"""
        if trace_id in self.current_traces:
            self.current_traces[trace_id].violations.append({
                "timestamp": datetime.now().isoformat(),
                **violation
            })
    
    async def complete_trace(self, 
                           trace_id: str,
                           goal_achieved: bool,
                           satisfaction_score: float,
                           duration_ms: int):
        """Complete an execution trace"""
        if trace_id not in self.current_traces:
            logger.warning(f"Unknown trace ID: {trace_id}")
            return
        
        trace = self.current_traces[trace_id]
        
        # Update final metrics
        trace.goal_achieved = goal_achieved
        trace.satisfaction_score = satisfaction_score
        trace.duration_ms = duration_ms
        
        # Calculate reward
        trace.reward = self._calculate_reward(trace)
        
        # Add step traces
        trace.step_traces = [step.to_dict() for step in self.step_buffer[trace_id]]
        
        # Write complete trace
        await self._write_execution_trace(trace)
        
        # Update summary
        await self._update_summary(trace)
        
        # Cleanup
        del self.current_traces[trace_id]
        del self.step_buffer[trace_id]
        
        logger.info(f"Completed trace {trace_id}: achieved={goal_achieved}, reward={trace.reward:.2f}")
    
    def _calculate_reward(self, trace: ExecutionTrace) -> float:
        """Calculate reward for RL training"""
        reward = 0.0
        
        # Goal achievement
        if trace.goal_achieved:
            reward += 1.0
        else:
            reward -= 0.5
        
        # Efficiency bonus/penalty
        if trace.total_steps > 0:
            success_rate = trace.successful_steps / trace.total_steps
            reward += 0.3 * (success_rate - 0.5)  # Bonus for >50% success
        
        # Time penalty
        if trace.duration_ms > 60000:  # >1 minute
            reward -= 0.1
        
        # Replanning penalty
        reward -= 0.1 * trace.replanning_count
        
        # Violation penalty
        reward -= 0.2 * len(trace.violations)
        
        # Satisfaction bonus
        reward += 0.2 * trace.satisfaction_score
        
        return max(-2.0, min(2.0, reward))  # Clip to [-2, 2]
    
    async def _write_execution_trace(self, trace: ExecutionTrace):
        """Write execution trace to file"""
        rl_format = trace.to_rl_format()
        
        async with asyncio.Lock():
            with open(self.trace_file, 'a') as f:
                f.write(json.dumps(rl_format) + '\n')
    
    async def _write_step_trace(self, trace_id: str, step: StepTrace):
        """Write step trace to file"""
        step_data = {
            "trace_id": trace_id,
            **step.to_dict()
        }
        
        async with asyncio.Lock():
            with open(self.step_file, 'a') as f:
                f.write(json.dumps(step_data) + '\n')
    
    async def _update_summary(self, trace: ExecutionTrace):
        """Update summary statistics"""
        summary_path = self.summary_file
        
        # Load existing summary
        if summary_path.exists():
            with open(summary_path, 'r') as f:
                summary = json.load(f)
        else:
            summary = {
                "total_traces": 0,
                "successful_goals": 0,
                "average_reward": 0.0,
                "average_steps": 0.0,
                "average_duration_ms": 0.0,
                "goal_types": {}
            }
        
        # Update statistics
        n = summary["total_traces"]
        summary["total_traces"] = n + 1
        
        if trace.goal_achieved:
            summary["successful_goals"] += 1
        
        # Running averages
        summary["average_reward"] = (n * summary["average_reward"] + trace.reward) / (n + 1)
        summary["average_steps"] = (n * summary["average_steps"] + trace.total_steps) / (n + 1)
        summary["average_duration_ms"] = (n * summary["average_duration_ms"] + trace.duration_ms) / (n + 1)
        
        # Goal type tracking
        goal_type = self._classify_goal(trace.goal)
        summary["goal_types"][goal_type] = summary["goal_types"].get(goal_type, 0) + 1
        
        # Write updated summary
        with open(summary_path, 'w') as f:
            json.dump(summary, f, indent=2)
    
    def _classify_goal(self, goal: str) -> str:
        """Classify goal type for analysis"""
        goal_lower = goal.lower()
        
        if any(kw in goal_lower for kw in ["check", "monitor", "status"]):
            return "monitoring"
        elif any(kw in goal_lower for kw in ["clean", "fix", "repair"]):
            return "maintenance"
        elif any(kw in goal_lower for kw in ["analyze", "summarize", "report"]):
            return "analysis"
        elif any(kw in goal_lower for kw in ["alert", "notify", "warn"]):
            return "alerting"
        else:
            return "other"
    
    async def get_statistics(self) -> Dict[str, Any]:
        """Get current statistics"""
        if self.summary_file.exists():
            with open(self.summary_file, 'r') as f:
                return json.load(f)
        return {}


# Integration with main execution loop
class TraceIntegration:
    """Helper to integrate tracing with execution"""
    
    def __init__(self, logger: RLTraceLogger):
        self.logger = logger
        self.active_trace_id: Optional[str] = None
    
    async def __aenter__(self):
        return self
    
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        if self.active_trace_id and exc_type:
            # Log failure on exception
            await self.logger.complete_trace(
                self.active_trace_id,
                goal_achieved=False,
                satisfaction_score=0.0,
                duration_ms=0
            )
    
    async def start(self, goal: str, plan_id: str, context: Dict[str, Any]) -> str:
        """Start tracing"""
        self.active_trace_id = await self.logger.start_trace(goal, plan_id, context)
        return self.active_trace_id
    
    async def log_step(self, step: StepTrace):
        """Log a step"""
        if self.active_trace_id:
            await self.logger.log_step(self.active_trace_id, step)
    
    async def complete(self, goal_achieved: bool, satisfaction: float, duration_ms: int):
        """Complete tracing"""
        if self.active_trace_id:
            await self.logger.complete_trace(
                self.active_trace_id,
                goal_achieved,
                satisfaction,
                duration_ms
            )


async def demo_trace_logging():
    """Demonstrate trace logging"""
    logger = RLTraceLogger()
    
    # Simulate an execution
    trace_id = await logger.start_trace(
        goal="Check system memory and clean if needed",
        plan_id="plan_20240101_120000",
        context={"plan_confidence": 0.85}
    )
    
    # Log steps
    step1 = StepTrace(
        step_id="step_1",
        timestamp=datetime.now(),
        action_type="execute",
        description="Check memory usage",
        tool_selected="memory_check",
        tool_confidence=0.9,
        rl_routing_used=True,
        success=True,
        duration_ms=250
    )
    await logger.log_step(trace_id, step1)
    
    step2 = StepTrace(
        step_id="step_2",
        timestamp=datetime.now(),
        action_type="condition",
        description="Evaluate if cleanup needed",
        success=True,
        duration_ms=50,
        inputs_from=["step_1"]
    )
    await logger.log_step(trace_id, step2)
    
    step3 = StepTrace(
        step_id="step_3",
        timestamp=datetime.now(),
        action_type="execute",
        description="Clean memory",
        tool_selected="memory_clean",
        tool_confidence=0.85,
        rl_routing_used=True,
        success=True,
        duration_ms=1500,
        inputs_from=["step_2"]
    )
    await logger.log_step(trace_id, step3)
    
    # Complete trace
    await logger.complete_trace(
        trace_id=trace_id,
        goal_achieved=True,
        satisfaction_score=0.9,
        duration_ms=1800
    )
    
    # Show statistics
    stats = await logger.get_statistics()
    print("📊 Trace Statistics:")
    print(json.dumps(stats, indent=2))


if __name__ == "__main__":
    asyncio.run(demo_trace_logging())