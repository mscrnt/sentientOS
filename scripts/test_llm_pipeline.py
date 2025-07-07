#!/usr/bin/env python3
"""
Test the LLM pipeline with simulated components
Demonstrates the full flow without requiring compilation
"""

import json
import datetime
from dataclasses import dataclass
from typing import Optional, List, Dict, Any

# Simulated components

@dataclass
class RouteResult:
    intent: str
    model: str
    confidence: float
    fallback_chain: List[str]

@dataclass
class RagResponse:
    answer: str
    sources: List[str]
    confidence: float

@dataclass
class ToolExecution:
    tool_name: str
    output: str
    exit_code: int
    duration_ms: int

@dataclass
class TraceEntry:
    trace_id: str
    timestamp: str
    prompt: str
    intent: str
    model_used: str
    tool_executed: Optional[str]
    rag_used: bool
    conditions_evaluated: List[str]
    success: bool
    duration_ms: int
    reward: Optional[float]


class IntelligentRouter:
    """Simulates the LLM routing logic"""
    
    def route(self, prompt: str) -> RouteResult:
        prompt_lower = prompt.lower()
        
        # Intent detection
        if any(kw in prompt_lower for kw in ['call', 'execute', 'run', '!@']):
            intent = "ToolCall"
            model = "phi2_local"  # Fast, trusted for tools
        elif any(kw in prompt_lower for kw in ['code', 'script', 'function']):
            intent = "CodeGeneration"
            model = "qwen2.5"
        elif any(kw in prompt_lower for kw in ['analyze', 'debug', 'error']):
            intent = "Analysis"
            model = "gpt-4o-mini"
        else:
            intent = "GeneralKnowledge"
            model = "llama3.2"
        
        return RouteResult(
            intent=intent,
            model=model,
            confidence=0.85,
            fallback_chain=[model, "phi2_local"]
        )


class RagSystem:
    """Simulates RAG retrieval"""
    
    def query(self, prompt: str) -> RagResponse:
        # Simulate knowledge retrieval
        knowledge_base = {
            "memory": "Memory management in SentientOS uses automatic garbage collection...",
            "disk": "Disk space can be checked using the disk_info tool...",
            "cpu": "CPU usage is monitored through the process_list tool...",
        }
        
        answer = "No specific information found."
        sources = []
        confidence = 0.3
        
        for key, value in knowledge_base.items():
            if key in prompt.lower():
                answer = value
                sources = [f"docs/{key}_management.md"]
                confidence = 0.9
                break
        
        return RagResponse(answer=answer, sources=sources, confidence=confidence)


class ConditionMatcher:
    """Simulates condition matching for tool triggers"""
    
    def evaluate(self, rag_response: str) -> List[Dict[str, Any]]:
        conditions = []
        
        if "memory" in rag_response.lower() and "90%" in rag_response:
            conditions.append({
                "name": "high_memory",
                "tool": "clean_cache",
                "args": {"aggressive": True},
                "priority": 10
            })
        
        if "disk" in rag_response.lower():
            conditions.append({
                "name": "disk_check",
                "tool": "disk_info",
                "args": {"verbose": True},
                "priority": 8
            })
        
        return conditions


class ToolRegistry:
    """Simulates tool execution"""
    
    def execute(self, tool_name: str, args: Dict) -> ToolExecution:
        tools = {
            "disk_info": "Filesystem     Size  Used  Avail Use% Mounted on\n/dev/sda1       20G   15G   4.0G  79% /",
            "memory_usage": "Memory Usage: 8.2GB / 16GB (51.25%)",
            "clean_cache": "Cleaned 1.2GB of cache",
            "process_list": "PID   CPU%  MEM%  COMMAND\n1234  12.5  3.2   python",
        }
        
        output = tools.get(tool_name, f"Executed {tool_name}")
        return ToolExecution(
            tool_name=tool_name,
            output=output,
            exit_code=0,
            duration_ms=150
        )


class RagToolPipeline:
    """Main pipeline orchestrator"""
    
    def __init__(self):
        self.router = IntelligentRouter()
        self.rag = RagSystem()
        self.conditions = ConditionMatcher()
        self.tools = ToolRegistry()
        self.traces = []
    
    def execute(self, prompt: str, explain: bool = False) -> Dict[str, Any]:
        start_time = datetime.datetime.now()
        trace_id = f"trace-{len(self.traces)+1}"
        
        # Step 1: Route to appropriate model
        route_result = self.router.route(prompt)
        if explain:
            print(f"🧠 Intent detected: {route_result.intent}")
            print(f"📊 Selected model: {route_result.model}")
        
        # Step 2: Determine execution flow
        intent = route_result.intent
        rag_response = None
        tool_execution = None
        conditions_matched = []
        
        if intent == "ToolCall":
            # Direct tool execution
            tool_name = self._extract_tool_name(prompt)
            tool_execution = self.tools.execute(tool_name, {})
            if explain:
                print(f"🔧 Executing tool: {tool_name}")
        
        elif intent in ["GeneralKnowledge", "Analysis"]:
            # RAG first, then check conditions
            rag_response = self.rag.query(prompt)
            if explain:
                print(f"📚 RAG Response: {rag_response.answer}")
            
            # Check if conditions trigger tools
            conditions = self.conditions.evaluate(rag_response.answer)
            if conditions:
                conditions_matched = [c["name"] for c in conditions]
                if explain:
                    print(f"✅ Conditions matched: {conditions_matched}")
                
                # Execute highest priority tool
                tool_condition = max(conditions, key=lambda x: x["priority"])
                tool_execution = self.tools.execute(
                    tool_condition["tool"], 
                    tool_condition["args"]
                )
        
        # Calculate duration
        duration_ms = int((datetime.datetime.now() - start_time).total_seconds() * 1000)
        
        # Create trace entry
        trace = TraceEntry(
            trace_id=trace_id,
            timestamp=datetime.datetime.now().isoformat(),
            prompt=prompt,
            intent=intent,
            model_used=route_result.model,
            tool_executed=tool_execution.tool_name if tool_execution else None,
            rag_used=rag_response is not None,
            conditions_evaluated=conditions_matched,
            success=True,
            duration_ms=duration_ms,
            reward=None
        )
        
        self.traces.append(trace)
        
        # Build response
        response = {
            "trace_id": trace_id,
            "intent": intent,
            "model": route_result.model,
            "rag_response": rag_response,
            "tool_execution": tool_execution,
            "conditions_matched": conditions_matched,
            "duration_ms": duration_ms
        }
        
        if explain:
            print(f"⏱️  Duration: {duration_ms}ms")
            print(f"📝 Trace logged: {trace_id}")
        
        return response
    
    def _extract_tool_name(self, prompt: str) -> str:
        """Extract tool name from prompt"""
        tools = ["disk_info", "memory_usage", "process_list", "clean_cache"]
        for tool in tools:
            if tool in prompt.lower():
                return tool
        return "disk_info"  # default
    
    def collect_feedback(self, trace_id: str, feedback: str) -> float:
        """Collect user feedback and update reward"""
        reward_map = {"y": 1.0, "yes": 1.0, "n": -1.0, "no": -1.0}
        reward = reward_map.get(feedback.lower(), 0.0)
        
        # Update trace
        for trace in self.traces:
            if trace.trace_id == trace_id:
                trace.reward = reward
                break
        
        return reward
    
    def get_trace_summary(self) -> Dict[str, Any]:
        """Get summary statistics"""
        total = len(self.traces)
        successful = sum(1 for t in self.traces if t.success)
        rag_used = sum(1 for t in self.traces if t.rag_used)
        tool_used = sum(1 for t in self.traces if t.tool_executed)
        avg_duration = sum(t.duration_ms for t in self.traces) / total if total > 0 else 0
        
        rewarded = [t for t in self.traces if t.reward is not None]
        avg_reward = sum(t.reward for t in rewarded) / len(rewarded) if rewarded else 0
        
        return {
            "total_executions": total,
            "successful": successful,
            "success_rate": successful / total if total > 0 else 0,
            "rag_used": rag_used,
            "tool_used": tool_used,
            "avg_duration_ms": avg_duration,
            "rewarded_count": len(rewarded),
            "avg_reward": avg_reward
        }


def run_tests():
    """Run comprehensive pipeline tests"""
    print("🧪 SentientOS LLM Pipeline Test")
    print("=" * 60)
    
    pipeline = RagToolPipeline()
    
    # Test cases
    test_cases = [
        {
            "name": "Pure Query (RAG only)",
            "prompt": "What is system memory pressure?",
            "expected_intent": "GeneralKnowledge"
        },
        {
            "name": "Tool Call",
            "prompt": "call disk_info",
            "expected_intent": "ToolCall"
        },
        {
            "name": "Query with Tool Trigger",
            "prompt": "How do I check disk space?",
            "expected_intent": "GeneralKnowledge"
        },
        {
            "name": "Code Generation",
            "prompt": "Write a Python script to monitor CPU",
            "expected_intent": "CodeGeneration"
        },
        {
            "name": "System Analysis",
            "prompt": "Analyze this error log",
            "expected_intent": "Analysis"
        }
    ]
    
    print("\n📋 Running test cases:\n")
    
    for i, test in enumerate(test_cases, 1):
        print(f"Test {i}: {test['name']}")
        print(f"Prompt: \"{test['prompt']}\"")
        
        result = pipeline.execute(test['prompt'], explain=True)
        
        # Verify intent
        assert result['intent'] == test['expected_intent'], \
            f"Expected {test['expected_intent']}, got {result['intent']}"
        
        # Simulate feedback
        feedback = "y" if i % 2 == 0 else "n"
        reward = pipeline.collect_feedback(result['trace_id'], feedback)
        print(f"💬 Feedback: {feedback} → Reward: {reward}")
        
        print("-" * 40)
    
    # Show summary
    print("\n📊 Pipeline Summary:")
    summary = pipeline.get_trace_summary()
    for key, value in summary.items():
        print(f"  {key}: {value}")
    
    # Export traces
    print("\n📤 Exporting traces...")
    with open("test_traces.jsonl", "w") as f:
        for trace in pipeline.traces:
            f.write(json.dumps(trace.__dict__, default=str) + "\n")
    print("✅ Traces exported to test_traces.jsonl")
    
    print("\n✨ All tests passed!")


if __name__ == "__main__":
    run_tests()