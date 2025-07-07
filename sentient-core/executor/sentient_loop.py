#!/usr/bin/env python3
"""
SentientLoop: Self-Reflective Control Flow for SentientOS

This module implements the autonomous control loop that observes,
decides, and adapts execution based on results and goals.
"""

import asyncio
import logging
import json
from typing import Dict, Any, Optional, List, Tuple
from dataclasses import dataclass, field
from enum import Enum
from datetime import datetime, timedelta
import uuid

# Import dependencies
import sys
import os
sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from planner.planner import SentientPlanner, ExecutionPlan, PlanStep, ActionType
from .executor import SentientExecutor, ExecutionContext, ExecutionStatus

logger = logging.getLogger(__name__)


class LoopState(Enum):
    """State of the control loop"""
    PLANNING = "planning"
    EXECUTING = "executing"
    OBSERVING = "observing"
    DECIDING = "deciding"
    SUCCEEDED = "succeeded"
    FAILED = "failed"
    HALTED = "halted"


@dataclass
class LoopMetrics:
    """Metrics tracked during loop execution"""
    start_time: datetime = field(default_factory=datetime.now)
    end_time: Optional[datetime] = None
    total_steps: int = 0
    successful_steps: int = 0
    failed_steps: int = 0
    replanning_count: int = 0
    total_duration_ms: Optional[int] = None
    
    def calculate_success_rate(self) -> float:
        if self.total_steps == 0:
            return 0.0
        return self.successful_steps / self.total_steps


@dataclass 
class LoopConfig:
    """Configuration for the control loop"""
    max_steps: int = 100
    max_replanning: int = 3
    max_duration_seconds: int = 300
    confidence_threshold: float = 0.5
    backtrack_on_failure: bool = True
    adaptive_timeout: bool = True
    reward_success: float = 1.0
    penalty_failure: float = -0.5
    penalty_timeout: float = -0.3
    penalty_excessive_steps: float = -0.1


class ObservationEngine:
    """Engine for observing and analyzing execution results"""
    
    def analyze_execution(self, context: ExecutionContext) -> Dict[str, Any]:
        """Analyze execution context to extract insights"""
        analysis = {
            "completed_steps": len(context.get_completed_steps()),
            "total_steps": len(context.plan.steps),
            "failures": {},
            "bottlenecks": [],
            "recommendations": []
        }
        
        # Analyze failures
        for step_id, result in context.results.items():
            if result.status == ExecutionStatus.FAILED:
                analysis["failures"][step_id] = {
                    "error": result.error,
                    "retry_count": result.retry_count,
                    "tool_used": result.tool_used
                }
        
        # Identify bottlenecks (slow steps)
        slow_threshold_ms = 5000
        for step_id, result in context.results.items():
            if result.duration_ms and result.duration_ms > slow_threshold_ms:
                analysis["bottlenecks"].append({
                    "step_id": step_id,
                    "duration_ms": result.duration_ms
                })
        
        # Generate recommendations
        if len(analysis["failures"]) > 0:
            analysis["recommendations"].append("Consider alternative tools for failed steps")
        
        if len(analysis["bottlenecks"]) > 0:
            analysis["recommendations"].append("Optimize slow steps or add parallelization")
        
        return analysis
    
    def check_goal_satisfaction(self, goal: str, context: ExecutionContext) -> Tuple[bool, float]:
        """Check if the goal has been satisfied"""
        # Simple heuristic - check if STOP step succeeded
        for step in context.plan.steps:
            if step.action_type == ActionType.STOP:
                result = context.results.get(step.step_id)
                if result and result.status == ExecutionStatus.SUCCESS:
                    return True, 1.0
        
        # Check partial satisfaction based on completion rate
        completed = len(context.get_completed_steps())
        total = len(context.plan.steps)
        satisfaction = completed / total if total > 0 else 0.0
        
        return satisfaction >= 0.8, satisfaction


class DecisionEngine:
    """Engine for making decisions based on observations"""
    
    def __init__(self, config: LoopConfig):
        self.config = config
    
    def decide_next_action(self, 
                          observation: Dict[str, Any],
                          metrics: LoopMetrics,
                          context: ExecutionContext) -> Tuple[str, Dict[str, Any]]:
        """Decide next action based on observations"""
        
        # Check termination conditions
        if self._should_halt(metrics):
            return "halt", {"reason": "exceeded_limits"}
        
        # Check if goal is satisfied
        if observation.get("goal_satisfied", False):
            return "succeed", {"satisfaction": observation.get("satisfaction", 1.0)}
        
        # Check if we need to replan
        failure_rate = len(observation["failures"]) / max(observation["total_steps"], 1)
        if failure_rate > 0.5 and metrics.replanning_count < self.config.max_replanning:
            return "replan", {"reason": "high_failure_rate", "failures": observation["failures"]}
        
        # Check if we're stuck
        if self._is_stuck(context):
            if self.config.backtrack_on_failure:
                return "backtrack", {"reason": "no_progress"}
            else:
                return "halt", {"reason": "stuck"}
        
        # Continue execution
        return "continue", {}
    
    def _should_halt(self, metrics: LoopMetrics) -> bool:
        """Check if we should halt execution"""
        # Check step limit
        if metrics.total_steps >= self.config.max_steps:
            logger.warning(f"Exceeded max steps: {self.config.max_steps}")
            return True
        
        # Check time limit
        elapsed = datetime.now() - metrics.start_time
        if elapsed.total_seconds() > self.config.max_duration_seconds:
            logger.warning(f"Exceeded max duration: {self.config.max_duration_seconds}s")
            return True
        
        return False
    
    def _is_stuck(self, context: ExecutionContext) -> bool:
        """Check if execution is stuck"""
        # Get recent results
        recent_results = list(context.results.values())[-5:]
        if len(recent_results) < 3:
            return False
        
        # Check if all recent steps failed
        all_failed = all(r.status == ExecutionStatus.FAILED for r in recent_results)
        return all_failed


class AdaptationEngine:
    """Engine for adapting execution based on experience"""
    
    def __init__(self):
        self.step_performance: Dict[str, List[float]] = {}
        self.tool_performance: Dict[str, List[float]] = {}
    
    def update_performance(self, step_id: str, tool: Optional[str], success: bool, duration_ms: int):
        """Update performance metrics"""
        performance = 1.0 if success else 0.0
        
        # Penalize slow steps
        if duration_ms > 10000:  # >10 seconds
            performance *= 0.8
        
        # Update step performance
        if step_id not in self.step_performance:
            self.step_performance[step_id] = []
        self.step_performance[step_id].append(performance)
        
        # Update tool performance
        if tool:
            if tool not in self.tool_performance:
                self.tool_performance[tool] = []
            self.tool_performance[tool].append(performance)
    
    def get_step_confidence(self, step_id: str) -> float:
        """Get confidence for a step based on history"""
        if step_id not in self.step_performance:
            return 0.5  # Default confidence
        
        performances = self.step_performance[step_id][-10:]  # Last 10
        return sum(performances) / len(performances)
    
    def adapt_timeout(self, step: PlanStep, base_timeout: int) -> int:
        """Adapt timeout based on step history"""
        confidence = self.get_step_confidence(step.step_id)
        
        if confidence < 0.3:
            # Low confidence - increase timeout
            return int(base_timeout * 1.5)
        elif confidence > 0.8:
            # High confidence - can reduce timeout
            return int(base_timeout * 0.8)
        
        return base_timeout


class SentientLoop:
    """Main control loop for autonomous execution"""
    
    def __init__(self,
                 planner: Optional[SentientPlanner] = None,
                 executor: Optional[SentientExecutor] = None,
                 config: Optional[LoopConfig] = None):
        self.planner = planner or SentientPlanner()
        self.executor = executor or SentientExecutor()
        self.config = config or LoopConfig()
        self.observation_engine = ObservationEngine()
        self.decision_engine = DecisionEngine(self.config)
        self.adaptation_engine = AdaptationEngine()
        self.state = LoopState.PLANNING
        self.metrics = LoopMetrics()
    
    async def run_goal(self, goal: str, initial_context: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
        """Run the control loop to achieve a goal"""
        logger.info(f"🎯 Starting SentientLoop for goal: {goal}")
        loop_id = str(uuid.uuid4())
        
        # Initialize
        plan = None
        context = None
        final_result = {
            "loop_id": loop_id,
            "goal": goal,
            "status": "unknown",
            "metrics": None,
            "trace": []
        }
        
        try:
            while self.state not in [LoopState.SUCCEEDED, LoopState.FAILED, LoopState.HALTED]:
                logger.info(f"Loop state: {self.state.value}")
                
                if self.state == LoopState.PLANNING:
                    # Create or refine plan
                    if plan is None:
                        plan = self.planner.plan_goal(goal, initial_context)
                    else:
                        # Refine existing plan
                        execution_state = {
                            "completed_steps": context.get_completed_steps() if context else [],
                            "failures": self.observation_engine.analyze_execution(context)["failures"],
                            "context": initial_context
                        }
                        plan = self.planner.refine_plan(plan, execution_state)
                        self.metrics.replanning_count += 1
                    
                    logger.info(f"Created plan with {len(plan.steps)} steps")
                    self.state = LoopState.EXECUTING
                
                elif self.state == LoopState.EXECUTING:
                    # Execute the plan
                    context = await self.executor.execute_plan(plan)
                    
                    # Update metrics
                    for result in context.results.values():
                        self.metrics.total_steps += 1
                        if result.status == ExecutionStatus.SUCCESS:
                            self.metrics.successful_steps += 1
                        elif result.status == ExecutionStatus.FAILED:
                            self.metrics.failed_steps += 1
                        
                        # Update adaptation engine
                        self.adaptation_engine.update_performance(
                            result.step_id,
                            result.tool_used,
                            result.status == ExecutionStatus.SUCCESS,
                            result.duration_ms or 0
                        )
                    
                    self.state = LoopState.OBSERVING
                
                elif self.state == LoopState.OBSERVING:
                    # Analyze execution results
                    observation = self.observation_engine.analyze_execution(context)
                    
                    # Check goal satisfaction
                    satisfied, satisfaction = self.observation_engine.check_goal_satisfaction(goal, context)
                    observation["goal_satisfied"] = satisfied
                    observation["satisfaction"] = satisfaction
                    
                    # Log observation
                    final_result["trace"].append({
                        "timestamp": datetime.now().isoformat(),
                        "observation": observation
                    })
                    
                    self.state = LoopState.DECIDING
                
                elif self.state == LoopState.DECIDING:
                    # Make decision based on observations
                    observation = self.observation_engine.analyze_execution(context)
                    satisfied, satisfaction = self.observation_engine.check_goal_satisfaction(goal, context)
                    observation["goal_satisfied"] = satisfied
                    observation["satisfaction"] = satisfaction
                    
                    action, params = self.decision_engine.decide_next_action(
                        observation, self.metrics, context
                    )
                    
                    logger.info(f"Decision: {action} with params: {params}")
                    
                    # Execute decision
                    if action == "succeed":
                        self.state = LoopState.SUCCEEDED
                        final_result["reward"] = self.config.reward_success
                    elif action == "halt":
                        self.state = LoopState.HALTED
                        final_result["reward"] = self.config.penalty_timeout
                    elif action == "replan":
                        self.state = LoopState.PLANNING
                    elif action == "backtrack":
                        # Remove failed steps from plan
                        self.state = LoopState.PLANNING
                    else:  # continue
                        self.state = LoopState.EXECUTING
            
            # Finalize
            self.metrics.end_time = datetime.now()
            self.metrics.total_duration_ms = int(
                (self.metrics.end_time - self.metrics.start_time).total_seconds() * 1000
            )
            
            # Calculate final reward
            if self.state == LoopState.SUCCEEDED:
                final_result["status"] = "succeeded"
            elif self.state == LoopState.FAILED:
                final_result["status"] = "failed"
                final_result["reward"] = self.config.penalty_failure
            else:  # HALTED
                final_result["status"] = "halted"
            
            # Adjust reward based on efficiency
            if "reward" in final_result and self.metrics.total_steps > 20:
                final_result["reward"] += self.config.penalty_excessive_steps
            
            final_result["metrics"] = {
                "total_steps": self.metrics.total_steps,
                "successful_steps": self.metrics.successful_steps,
                "failed_steps": self.metrics.failed_steps,
                "success_rate": self.metrics.calculate_success_rate(),
                "replanning_count": self.metrics.replanning_count,
                "duration_ms": self.metrics.total_duration_ms
            }
            
            # Log final trace for RL
            await self._log_loop_trace(loop_id, goal, final_result)
            
        except Exception as e:
            logger.error(f"Loop error: {e}")
            self.state = LoopState.FAILED
            final_result["status"] = "error"
            final_result["error"] = str(e)
        
        return final_result
    
    async def _log_loop_trace(self, loop_id: str, goal: str, result: Dict[str, Any]):
        """Log complete loop execution for RL training"""
        trace = {
            "loop_id": loop_id,
            "goal": goal,
            "timestamp": datetime.now().isoformat(),
            "status": result["status"],
            "metrics": result.get("metrics", {}),
            "reward": result.get("reward", 0.0),
            "trace_count": len(result.get("trace", []))
        }
        
        # This would append to RL training data
        logger.info(f"Loop trace: {json.dumps(trace)}")


async def demo_sentient_loop():
    """Demonstrate the complete sentient loop"""
    
    # Create loop
    loop = SentientLoop()
    
    # Test goals
    goals = [
        "Check memory usage and clean if needed",
        "Monitor CPU and alert if over 90%",
        "Summarize errors from the last 24 hours"
    ]
    
    for goal in goals:
        print(f"\n{'='*60}")
        print(f"🎯 Goal: {goal}")
        print(f"{'='*60}")
        
        result = await loop.run_goal(goal)
        
        print(f"\n📊 Results:")
        print(f"  Status: {result['status']}")
        print(f"  Metrics: {json.dumps(result['metrics'], indent=2)}")
        print(f"  Reward: {result.get('reward', 0.0):.2f}")


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)
    asyncio.run(demo_sentient_loop())