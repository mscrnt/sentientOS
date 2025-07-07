#!/usr/bin/env python3
"""
SentientOS Trace Collector
Interactive CLI for generating high-quality execution traces with user feedback
"""

import json
import os
import sys
import uuid
import datetime
import random
from pathlib import Path
from typing import Optional, List, Dict, Any, Tuple
from dataclasses import dataclass, asdict
import readline  # For better input handling

# Import from test pipeline or implement inline
try:
    from test_llm_pipeline import RagToolPipeline, RouteResult, RagResponse, ToolExecution
except ImportError:
    # Inline implementation if needed
    pass

@dataclass
class TraceEntry:
    """Standard trace entry format for RL training"""
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
    # Additional metadata
    fallback_used: bool = False
    error_occurred: bool = False
    user_satisfaction: Optional[str] = None


class TraceCollector:
    """Interactive trace collection system"""
    
    def __init__(self, output_dir: str = "traces"):
        self.output_dir = Path(output_dir)
        self.output_dir.mkdir(exist_ok=True)
        self.trace_file = self.output_dir / "trace_log.jsonl"
        self.session_id = str(uuid.uuid4())[:8]
        self.traces_collected = 0
        
        # Initialize components
        self.pipeline = self._initialize_pipeline()
        
        # Track statistics
        self.stats = {
            "total_traces": 0,
            "positive_feedback": 0,
            "negative_feedback": 0,
            "skipped_feedback": 0,
            "intents_seen": set(),
            "models_used": set(),
            "tools_executed": set(),
        }
        
        # Example prompts for variety
        self.example_prompts = [
            # General Knowledge
            "What is the difference between RAM and storage?",
            "How does CPU scheduling work in modern operating systems?",
            "Explain the concept of virtual memory",
            
            # Tool Calls
            "Check current disk usage",
            "Show me system memory statistics",
            "List running processes sorted by CPU usage",
            "!@ call network_status",
            
            # Analysis & Debugging
            "Analyze why my system is running slow",
            "Debug this error: segmentation fault core dumped",
            "Why is my memory usage at 95%?",
            
            # Code Generation
            "Write a Python script to monitor disk space",
            "Generate a bash function to check system health",
            "Create a systemd service for log rotation",
            
            # Hybrid Queries
            "Check if I need to clean my cache and do it if necessary",
            "Monitor CPU temperature and alert if too high",
            "Show disk usage and suggest cleanup options",
            
            # Edge Cases
            "hjkl",  # Gibberish
            "Help",  # Simple help
            "What's 2+2?",  # Math
            "Tell me a joke about Linux",  # Humor
            "",  # Empty query
        ]
    
    def _initialize_pipeline(self):
        """Initialize the execution pipeline"""
        try:
            return RagToolPipeline()
        except:
            # Fallback implementation
            return MockPipeline()
    
    def collect_trace(self, prompt: str) -> TraceEntry:
        """Execute prompt and collect trace data"""
        start_time = datetime.datetime.now()
        trace_id = f"{self.session_id}-{self.traces_collected:04d}"
        
        try:
            # Execute through pipeline
            result = self.pipeline.execute(prompt, explain=False)
            
            # Extract trace data
            trace = TraceEntry(
                trace_id=trace_id,
                timestamp=start_time.isoformat(),
                prompt=prompt,
                intent=result.get('intent', 'Unknown'),
                model_used=result.get('model', 'phi2_local'),
                tool_executed=result.get('tool_execution', {}).get('tool_name') if result.get('tool_execution') else None,
                rag_used=result.get('rag_response') is not None,
                conditions_evaluated=result.get('conditions_matched', []),
                success=True,
                duration_ms=result.get('duration_ms', 0),
                reward=None,  # Will be set by feedback
                fallback_used=result.get('fallback_used', False),
                error_occurred=False
            )
            
            # Update statistics
            self.stats['intents_seen'].add(trace.intent)
            self.stats['models_used'].add(trace.model_used)
            if trace.tool_executed:
                self.stats['tools_executed'].add(trace.tool_executed)
            
        except Exception as e:
            # Handle errors gracefully
            trace = TraceEntry(
                trace_id=trace_id,
                timestamp=start_time.isoformat(),
                prompt=prompt,
                intent="Error",
                model_used="error_handler",
                tool_executed=None,
                rag_used=False,
                conditions_evaluated=[],
                success=False,
                duration_ms=0,
                reward=None,
                error_occurred=True
            )
            print(f"⚠️  Error during execution: {e}")
        
        return trace
    
    def collect_feedback(self, trace: TraceEntry) -> Tuple[Optional[float], str]:
        """Collect user feedback and map to reward"""
        print("\n" + "="*50)
        print("📋 Execution Summary:")
        print(f"  Intent: {trace.intent}")
        print(f"  Model: {trace.model_used}")
        if trace.tool_executed:
            print(f"  Tool: {trace.tool_executed}")
        if trace.rag_used:
            print(f"  RAG: Used")
        print(f"  Duration: {trace.duration_ms}ms")
        print("="*50)
        
        while True:
            feedback = input("\n💬 Was this helpful? [y]es / [n]o / [s]kip: ").strip().lower()
            
            if feedback in ['y', 'yes']:
                reward = 1.0
                satisfaction = "satisfied"
                self.stats['positive_feedback'] += 1
                print("✅ Thank you! Positive feedback recorded.")
                break
            elif feedback in ['n', 'no']:
                reward = -1.0
                satisfaction = "unsatisfied"
                self.stats['negative_feedback'] += 1
                print("❌ Thank you! Negative feedback recorded.")
                
                # Optional: collect more details
                reason = input("   (Optional) What went wrong? ").strip()
                if reason:
                    trace.user_satisfaction = f"unsatisfied: {reason}"
                break
            elif feedback in ['s', 'skip', '']:
                reward = None
                satisfaction = "skipped"
                self.stats['skipped_feedback'] += 1
                print("⏭️  Skipped - no reward assigned.")
                break
            else:
                print("❓ Please enter 'y', 'n', or 's'")
        
        return reward, satisfaction
    
    def save_trace(self, trace: TraceEntry):
        """Append trace to JSONL file"""
        with open(self.trace_file, 'a') as f:
            json.dump(asdict(trace), f)
            f.write('\n')
        self.traces_collected += 1
        self.stats['total_traces'] += 1
    
    def show_statistics(self):
        """Display collection statistics"""
        print("\n" + "="*60)
        print("📊 Collection Statistics")
        print("="*60)
        print(f"Total Traces: {self.stats['total_traces']}")
        print(f"Positive Feedback: {self.stats['positive_feedback']} ({self.stats['positive_feedback']/max(1, self.stats['total_traces'])*100:.1f}%)")
        print(f"Negative Feedback: {self.stats['negative_feedback']} ({self.stats['negative_feedback']/max(1, self.stats['total_traces'])*100:.1f}%)")
        print(f"Skipped: {self.stats['skipped_feedback']}")
        print(f"\nUnique Intents: {len(self.stats['intents_seen'])} - {self.stats['intents_seen']}")
        print(f"Models Used: {len(self.stats['models_used'])} - {self.stats['models_used']}")
        print(f"Tools Executed: {len(self.stats['tools_executed'])} - {self.stats['tools_executed']}")
    
    def suggest_prompts(self):
        """Suggest diverse prompts for testing"""
        print("\n💡 Suggested prompts for diversity:")
        # Get 5 random prompts
        suggestions = random.sample(self.example_prompts, min(5, len(self.example_prompts)))
        for i, prompt in enumerate(suggestions, 1):
            print(f"  {i}. {prompt}")
    
    def run_interactive_session(self):
        """Main interactive collection loop"""
        print("="*60)
        print("🤖 SentientOS Trace Collector")
        print("="*60)
        print(f"Session ID: {self.session_id}")
        print(f"Output: {self.trace_file}")
        print("\nCollect execution traces with user feedback for RL training.")
        print("Commands:")
        print("  'quit' or 'exit' - End session")
        print("  'stats' - Show statistics")
        print("  'suggest' - Get prompt suggestions")
        print("  'help' - Show this help")
        print("-"*60)
        
        # Enable readline history
        histfile = Path.home() / ".sentientos_trace_history"
        try:
            readline.read_history_file(histfile)
        except FileNotFoundError:
            pass
        
        while True:
            try:
                # Get user input
                prompt = input(f"\n[{self.traces_collected}] Enter prompt (or command): ").strip()
                
                # Handle commands
                if prompt.lower() in ['quit', 'exit']:
                    break
                elif prompt.lower() == 'stats':
                    self.show_statistics()
                    continue
                elif prompt.lower() == 'suggest':
                    self.suggest_prompts()
                    continue
                elif prompt.lower() == 'help':
                    print("Enter any query to test the system, then provide feedback.")
                    print("Try different types: questions, commands, tool calls, analysis requests.")
                    continue
                elif not prompt:
                    continue
                
                # Collect trace
                print(f"\n🔄 Processing: \"{prompt}\"")
                trace = self.collect_trace(prompt)
                
                # Show response (in real system, would show actual output)
                self._show_response(trace)
                
                # Collect feedback
                reward, satisfaction = self.collect_feedback(trace)
                trace.reward = reward
                trace.user_satisfaction = satisfaction
                
                # Save trace
                self.save_trace(trace)
                print(f"💾 Trace saved: {trace.trace_id}")
                
            except KeyboardInterrupt:
                print("\n\n⚠️  Interrupted. Type 'quit' to exit cleanly.")
                continue
            except Exception as e:
                print(f"\n❌ Error: {e}")
                continue
        
        # Save readline history
        try:
            readline.write_history_file(histfile)
        except:
            pass
        
        # Final summary
        print("\n" + "="*60)
        print("✅ Collection Session Complete!")
        self.show_statistics()
        print(f"\nTraces saved to: {self.trace_file}")
        print("="*60)
    
    def _show_response(self, trace: TraceEntry):
        """Show simulated response based on trace"""
        print("\n📤 Response:")
        
        if trace.intent == "GeneralKnowledge" and trace.rag_used:
            print("  [RAG] Retrieved relevant documentation about your query.")
        elif trace.intent == "ToolCall" and trace.tool_executed:
            print(f"  [Tool: {trace.tool_executed}] Execution completed successfully.")
        elif trace.intent == "CodeGeneration":
            print("  [Code] Generated the requested code snippet.")
        elif trace.intent == "Analysis":
            print("  [Analysis] Completed system analysis based on current state.")
        else:
            print("  [Response] Query processed.")
    
    def run_batch_collection(self, prompts: List[str], auto_feedback: Optional[str] = None):
        """Batch collection mode for automated testing"""
        print(f"📦 Batch mode: Processing {len(prompts)} prompts")
        
        for i, prompt in enumerate(prompts):
            print(f"\n[{i+1}/{len(prompts)}] {prompt}")
            
            # Collect trace
            trace = self.collect_trace(prompt)
            
            # Auto feedback or prompt
            if auto_feedback:
                if auto_feedback == "random":
                    reward = random.choice([1.0, -1.0, None])
                elif auto_feedback == "positive":
                    reward = 1.0
                elif auto_feedback == "negative":
                    reward = -1.0
                else:
                    reward = None
                trace.reward = reward
                print(f"   Auto-feedback: {reward}")
            else:
                reward, _ = self.collect_feedback(trace)
                trace.reward = reward
            
            # Save trace
            self.save_trace(trace)
        
        print(f"\n✅ Batch complete: {len(prompts)} traces collected")


class MockPipeline:
    """Mock pipeline for testing without full implementation"""
    
    def execute(self, prompt: str, explain: bool = False) -> Dict[str, Any]:
        """Simulate pipeline execution"""
        import time
        import random
        
        # Simulate processing time
        time.sleep(random.uniform(0.1, 0.3))
        
        # Simple intent detection
        prompt_lower = prompt.lower()
        if any(kw in prompt_lower for kw in ['call', 'execute', 'run', '!@']):
            intent = "ToolCall"
            model = "phi2_local"
            tool = random.choice(["disk_info", "memory_usage", "process_list"])
        elif any(kw in prompt_lower for kw in ['write', 'generate', 'create', 'code']):
            intent = "CodeGeneration"
            model = "qwen2.5"
            tool = None
        elif any(kw in prompt_lower for kw in ['analyze', 'debug', 'why']):
            intent = "Analysis"
            model = "gpt-4o-mini"
            tool = None
        else:
            intent = "GeneralKnowledge"
            model = "llama3.2"
            tool = None
        
        # Simulate RAG usage
        rag_used = intent in ["GeneralKnowledge", "Analysis"] or random.random() > 0.5
        
        # Build response
        result = {
            "intent": intent,
            "model": model,
            "rag_response": {"answer": "Mock response"} if rag_used else None,
            "tool_execution": {"tool_name": tool} if tool else None,
            "conditions_matched": ["condition1"] if tool and rag_used else [],
            "duration_ms": random.randint(50, 500),
            "fallback_used": random.random() < 0.1
        }
        
        return result


def main():
    """Main entry point"""
    import argparse
    
    parser = argparse.ArgumentParser(description="SentientOS Trace Collector")
    parser.add_argument("--batch", nargs="+", help="Run in batch mode with prompts")
    parser.add_argument("--auto-feedback", choices=["random", "positive", "negative"],
                       help="Automatic feedback for batch mode")
    parser.add_argument("--output-dir", default="traces", help="Output directory for traces")
    parser.add_argument("--examples", action="store_true", help="Run with example prompts")
    
    args = parser.parse_args()
    
    # Initialize collector
    collector = TraceCollector(output_dir=args.output_dir)
    
    if args.batch:
        # Batch mode
        collector.run_batch_collection(args.batch, args.auto_feedback)
    elif args.examples:
        # Run with examples
        print("🎯 Running with example prompts...")
        example_subset = random.sample(collector.example_prompts, 20)
        collector.run_batch_collection(example_subset, "random")
    else:
        # Interactive mode
        collector.run_interactive_session()


if __name__ == "__main__":
    main()