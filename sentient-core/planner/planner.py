#!/usr/bin/env python3
"""
SentientPlan: Hierarchical Goal Planner for SentientOS

This module decomposes high-level goals into executable action sequences.
It uses LLM-based planning with RAG context to create optimal execution plans.
"""

import json
import logging
from typing import List, Dict, Any, Optional, Tuple
from dataclasses import dataclass, field
from enum import Enum
from datetime import datetime
import re

logger = logging.getLogger(__name__)


class ActionType(Enum):
    """Types of actions in a plan"""
    QUERY = "query"          # Information retrieval
    EXECUTE = "execute"      # Tool execution
    CONDITION = "condition"  # Conditional branch
    LOOP = "loop"           # Iterative action
    PARALLEL = "parallel"   # Parallel execution
    STOP = "stop"           # Termination


@dataclass
class PlanStep:
    """Single step in an execution plan"""
    step_id: str
    action_type: ActionType
    description: str
    tool_hint: Optional[str] = None
    dependencies: List[str] = field(default_factory=list)
    inputs: Dict[str, Any] = field(default_factory=dict)
    expected_output: Optional[str] = None
    success_criteria: Optional[str] = None
    retry_policy: Dict[str, Any] = field(default_factory=lambda: {"max_retries": 2})
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "step_id": self.step_id,
            "action_type": self.action_type.value,
            "description": self.description,
            "tool_hint": self.tool_hint,
            "dependencies": self.dependencies,
            "inputs": self.inputs,
            "expected_output": self.expected_output,
            "success_criteria": self.success_criteria,
            "retry_policy": self.retry_policy
        }


@dataclass
class ExecutionPlan:
    """Complete execution plan for a goal"""
    plan_id: str
    goal: str
    steps: List[PlanStep]
    created_at: datetime
    metadata: Dict[str, Any] = field(default_factory=dict)
    
    def get_executable_steps(self, completed_steps: List[str]) -> List[PlanStep]:
        """Get steps that can be executed based on completed dependencies"""
        executable = []
        for step in self.steps:
            if step.step_id not in completed_steps:
                if all(dep in completed_steps for dep in step.dependencies):
                    executable.append(step)
        return executable
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "plan_id": self.plan_id,
            "goal": self.goal,
            "steps": [step.to_dict() for step in self.steps],
            "created_at": self.created_at.isoformat(),
            "metadata": self.metadata
        }


class SentientPlanner:
    """Hierarchical planner for autonomous goal execution"""
    
    def __init__(self, llm_client=None, rag_client=None):
        self.llm_client = llm_client
        self.rag_client = rag_client
        self.planning_templates = self._load_planning_templates()
        
    def _load_planning_templates(self) -> Dict[str, str]:
        """Load planning prompt templates"""
        return {
            "decompose": """You are a goal decomposition expert for an autonomous AI system.
Given a high-level goal, break it down into concrete, executable steps.

Goal: {goal}

Consider available tools and capabilities:
- File operations (read, write, search)
- System monitoring (CPU, memory, disk)
- Log analysis and summarization
- Alert generation
- Data processing and transformation

Generate a step-by-step plan with:
1. Clear action descriptions
2. Tool hints (which tool might be used)
3. Dependencies between steps
4. Success criteria
5. Expected outputs

Format as JSON list of steps.""",

            "refine": """Given this execution plan and the current context, refine it:

Original Plan: {plan}
Context: {context}
Completed Steps: {completed}
Failures: {failures}

Refine the plan by:
1. Adjusting for completed work
2. Handling failures with alternatives
3. Optimizing remaining steps
4. Adding recovery actions if needed

Return refined JSON plan."""
        }
    
    def plan_goal(self, goal: str, context: Optional[Dict[str, Any]] = None) -> ExecutionPlan:
        """Create an execution plan for a high-level goal"""
        logger.info(f"Planning for goal: {goal}")
        
        # Extract goal components
        goal_analysis = self._analyze_goal(goal)
        
        # Retrieve relevant context from RAG if available
        rag_context = self._get_rag_context(goal) if self.rag_client else {}
        
        # Generate plan steps
        if self.llm_client:
            steps = self._llm_plan_generation(goal, goal_analysis, rag_context)
        else:
            steps = self._heuristic_plan_generation(goal, goal_analysis)
        
        # Create execution plan
        plan = ExecutionPlan(
            plan_id=f"plan_{datetime.now().strftime('%Y%m%d_%H%M%S')}",
            goal=goal,
            steps=steps,
            created_at=datetime.now(),
            metadata={
                "goal_analysis": goal_analysis,
                "rag_context": rag_context
            }
        )
        
        logger.info(f"Generated plan with {len(steps)} steps")
        return plan
    
    def _analyze_goal(self, goal: str) -> Dict[str, Any]:
        """Analyze goal to extract key components"""
        analysis = {
            "keywords": [],
            "actions": [],
            "conditions": [],
            "targets": []
        }
        
        # Extract action words
        action_patterns = [
            r'\b(check|monitor|analyze|summarize|clean|send|alert|email|fetch|filter)\b',
            r'\b(read|write|execute|run|stop|start|restart)\b'
        ]
        for pattern in action_patterns:
            matches = re.findall(pattern, goal.lower())
            analysis["actions"].extend(matches)
        
        # Extract conditions
        if any(word in goal.lower() for word in ['if', 'when', 'unless', 'until']):
            analysis["conditions"].append("conditional_execution")
        
        # Extract targets (logs, memory, cpu, etc.)
        target_patterns = [
            r'\b(log|logs|error|errors|memory|cpu|disk|file|files)\b',
            r'\b(system|process|service|alert|notification)\b'
        ]
        for pattern in target_patterns:
            matches = re.findall(pattern, goal.lower())
            analysis["targets"].extend(matches)
        
        return analysis
    
    def _get_rag_context(self, goal: str) -> Dict[str, Any]:
        """Retrieve relevant context from RAG system"""
        # This would query the RAG system for relevant examples,
        # previous successful plans, tool documentation, etc.
        return {
            "similar_goals": [],
            "tool_suggestions": [],
            "best_practices": []
        }
    
    def _llm_plan_generation(self, goal: str, analysis: Dict, context: Dict) -> List[PlanStep]:
        """Generate plan using LLM"""
        prompt = self.planning_templates["decompose"].format(goal=goal)
        
        # Call LLM (placeholder for actual implementation)
        # response = self.llm_client.generate(prompt)
        
        # For now, fall back to heuristic
        return self._heuristic_plan_generation(goal, analysis)
    
    def _heuristic_plan_generation(self, goal: str, analysis: Dict) -> List[PlanStep]:
        """Generate plan using heuristics"""
        steps = []
        step_counter = 1
        
        # Common patterns
        if "check" in analysis["actions"] and "memory" in analysis["targets"]:
            steps.append(PlanStep(
                step_id=f"step_{step_counter}",
                action_type=ActionType.EXECUTE,
                description="Check system memory usage",
                tool_hint="memory_check",
                expected_output="memory_stats",
                success_criteria="memory_data_retrieved"
            ))
            step_counter += 1
            
            if "clean" in analysis["actions"]:
                steps.append(PlanStep(
                    step_id=f"step_{step_counter}",
                    action_type=ActionType.CONDITION,
                    description="Evaluate if memory cleanup needed",
                    dependencies=[f"step_{step_counter-1}"],
                    inputs={"memory_threshold": 80},
                    success_criteria="decision_made"
                ))
                step_counter += 1
                
                steps.append(PlanStep(
                    step_id=f"step_{step_counter}",
                    action_type=ActionType.EXECUTE,
                    description="Execute memory cleanup",
                    tool_hint="memory_clean",
                    dependencies=[f"step_{step_counter-1}"],
                    success_criteria="memory_freed"
                ))
                step_counter += 1
        
        elif "summarize" in analysis["actions"] and "error" in analysis["targets"]:
            steps.extend([
                PlanStep(
                    step_id=f"step_{step_counter}",
                    action_type=ActionType.EXECUTE,
                    description="Fetch recent logs",
                    tool_hint="log_fetch",
                    inputs={"time_range": "24h"},
                    expected_output="log_entries"
                ),
                PlanStep(
                    step_id=f"step_{step_counter+1}",
                    action_type=ActionType.EXECUTE,
                    description="Filter error entries",
                    tool_hint="log_filter",
                    dependencies=[f"step_{step_counter}"],
                    inputs={"filter": "error|critical"},
                    expected_output="error_logs"
                ),
                PlanStep(
                    step_id=f"step_{step_counter+2}",
                    action_type=ActionType.QUERY,
                    description="Summarize error patterns",
                    tool_hint="llm_summarize",
                    dependencies=[f"step_{step_counter+1}"],
                    expected_output="error_summary"
                )
            ])
            step_counter += 3
        
        # Add termination step
        steps.append(PlanStep(
            step_id=f"step_{step_counter}",
            action_type=ActionType.STOP,
            description="Goal completed",
            dependencies=[s.step_id for s in steps[-1:]]  # Depend on last step
        ))
        
        return steps
    
    def refine_plan(self, plan: ExecutionPlan, execution_state: Dict[str, Any]) -> ExecutionPlan:
        """Refine plan based on execution state"""
        completed = execution_state.get("completed_steps", [])
        failures = execution_state.get("failures", {})
        
        if self.llm_client:
            prompt = self.planning_templates["refine"].format(
                plan=json.dumps([s.to_dict() for s in plan.steps]),
                context=json.dumps(execution_state.get("context", {})),
                completed=json.dumps(completed),
                failures=json.dumps(failures)
            )
            # refined_steps = self.llm_client.generate(prompt)
        
        # For now, simple refinement
        remaining_steps = []
        for step in plan.steps:
            if step.step_id not in completed:
                # Add retry logic for failed steps
                if step.step_id in failures:
                    if failures[step.step_id]["retry_count"] < step.retry_policy["max_retries"]:
                        step.description = f"[RETRY] {step.description}"
                        remaining_steps.append(step)
                else:
                    remaining_steps.append(step)
        
        # Create refined plan
        refined = ExecutionPlan(
            plan_id=f"{plan.plan_id}_refined",
            goal=plan.goal,
            steps=remaining_steps,
            created_at=datetime.now(),
            metadata={
                **plan.metadata,
                "refinement_reason": "execution_state_update",
                "original_plan_id": plan.plan_id
            }
        )
        
        return refined
    
    def validate_plan(self, plan: ExecutionPlan) -> Tuple[bool, List[str]]:
        """Validate plan for common issues"""
        issues = []
        
        # Check for circular dependencies
        for step in plan.steps:
            if step.step_id in step.dependencies:
                issues.append(f"Circular dependency in {step.step_id}")
        
        # Check for unreachable steps
        all_deps = set()
        all_steps = {s.step_id for s in plan.steps}
        for step in plan.steps:
            all_deps.update(step.dependencies)
        
        missing_deps = all_deps - all_steps
        if missing_deps:
            issues.append(f"Missing dependencies: {missing_deps}")
        
        # Check for termination
        has_stop = any(s.action_type == ActionType.STOP for s in plan.steps)
        if not has_stop:
            issues.append("No termination step found")
        
        return len(issues) == 0, issues


def create_example_plans():
    """Create example plans for testing"""
    planner = SentientPlanner()
    
    examples = [
        "Check memory usage and clean if needed",
        "Summarize yesterday's errors",
        "Monitor CPU and alert if over 90%",
        "Analyze logs for security threats and generate report"
    ]
    
    for goal in examples:
        plan = planner.plan_goal(goal)
        print(f"\nGoal: {goal}")
        print(f"Plan: {plan.plan_id}")
        for step in plan.steps:
            deps = f" (deps: {step.dependencies})" if step.dependencies else ""
            print(f"  - {step.step_id}: {step.description}{deps}")


if __name__ == "__main__":
    create_example_plans()