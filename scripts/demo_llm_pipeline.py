#!/usr/bin/env python3
"""
Interactive demo of the SentientOS LLM Pipeline
Shows the complete flow from query to learning
"""

import json
import time
from test_llm_pipeline import RagToolPipeline

def print_banner(text):
    print("\n" + "="*60)
    print(f"  {text}")
    print("="*60)

def demo_scenario(pipeline, scenario_name, prompt, pause=True):
    """Run a demo scenario with detailed output"""
    print_banner(f"Scenario: {scenario_name}")
    print(f"\n🎯 User Query: \"{prompt}\"")
    
    if pause:
        input("\nPress Enter to execute...")
    
    print("\n" + "-"*40)
    result = pipeline.execute(prompt, explain=True)
    print("-"*40)
    
    # Show detailed results
    print("\n📊 Execution Results:")
    print(f"  • Intent: {result['intent']}")
    print(f"  • Model: {result['model']}")
    print(f"  • Duration: {result['duration_ms']}ms")
    
    if result['rag_response']:
        print(f"\n📚 Knowledge Retrieved:")
        print(f"  • Answer: {result['rag_response'].answer}")
        print(f"  • Confidence: {result['rag_response'].confidence}")
        print(f"  • Sources: {result['rag_response'].sources}")
    
    if result['tool_execution']:
        print(f"\n🔧 Tool Executed:")
        print(f"  • Tool: {result['tool_execution'].tool_name}")
        print(f"  • Exit Code: {result['tool_execution'].exit_code}")
        print(f"  • Output:\n    {result['tool_execution'].output}")
    
    if result['conditions_matched']:
        print(f"\n✅ Conditions Matched: {result['conditions_matched']}")
    
    # Collect feedback
    print("\n💬 Was this helpful? [Y/n/s(kip)]: ", end="")
    feedback = input().strip() or "y"
    
    if feedback.lower() != "s":
        reward = pipeline.collect_feedback(result['trace_id'], feedback)
        print(f"📝 Feedback recorded: {reward}")
    
    return result

def main():
    print_banner("🧠 SentientOS LLM Pipeline Demo")
    print("\nThis demo shows the complete intelligent pipeline:")
    print("• Intent Detection → Model Selection")
    print("• RAG Retrieval → Condition Matching")
    print("• Tool Execution → Trace Logging")
    print("• User Feedback → Reinforcement Learning")
    
    pipeline = RagToolPipeline()
    
    # Demo scenarios
    scenarios = [
        ("Information Query", 
         "What is the purpose of memory management in SentientOS?"),
        
        ("Direct Tool Execution", 
         "Run disk_info to check available space"),
        
        ("Intelligent Action", 
         "My system is running slow, what should I check?"),
        
        ("Conditional Tool Trigger", 
         "How can I monitor disk usage? My disk might be getting full"),
        
        ("Code Generation Request", 
         "Write a script to monitor system resources"),
        
        ("Complex Analysis", 
         "Analyze why my memory usage is at 95% and suggest fixes")
    ]
    
    for name, prompt in scenarios:
        demo_scenario(pipeline, name, prompt)
    
    # Show learning summary
    print_banner("📊 Learning Summary")
    summary = pipeline.get_trace_summary()
    
    print("\n🔍 Execution Statistics:")
    print(f"  • Total Queries: {summary['total_executions']}")
    print(f"  • Success Rate: {summary['success_rate']*100:.1f}%")
    print(f"  • RAG Used: {summary['rag_used']} times")
    print(f"  • Tools Used: {summary['tool_used']} times")
    print(f"  • Avg Duration: {summary['avg_duration_ms']:.1f}ms")
    
    print("\n🎯 Feedback Analysis:")
    print(f"  • Responses with Feedback: {summary['rewarded_count']}")
    print(f"  • Average Reward: {summary['avg_reward']:.2f}")
    
    # Show model performance
    print("\n📈 Model Performance:")
    model_stats = {}
    for trace in pipeline.traces:
        model = trace.model_used
        if model not in model_stats:
            model_stats[model] = {"count": 0, "total_reward": 0, "rewarded": 0}
        model_stats[model]["count"] += 1
        if trace.reward is not None:
            model_stats[model]["total_reward"] += trace.reward
            model_stats[model]["rewarded"] += 1
    
    for model, stats in model_stats.items():
        avg_reward = stats["total_reward"] / stats["rewarded"] if stats["rewarded"] > 0 else 0
        print(f"  • {model}: {stats['count']} uses, avg reward: {avg_reward:.2f}")
    
    # Show intent distribution
    print("\n🎯 Intent Distribution:")
    intent_counts = {}
    for trace in pipeline.traces:
        intent_counts[trace.intent] = intent_counts.get(trace.intent, 0) + 1
    
    for intent, count in intent_counts.items():
        percentage = (count / summary['total_executions']) * 100
        print(f"  • {intent}: {count} ({percentage:.1f}%)")
    
    # Export for RL training
    print_banner("💾 Exporting Data for RL Training")
    
    trace_file = "demo_traces.jsonl"
    with open(trace_file, "w") as f:
        for trace in pipeline.traces:
            f.write(json.dumps(trace.__dict__, default=str) + "\n")
    
    print(f"\n✅ Exported {len(pipeline.traces)} traces to {trace_file}")
    print("📚 This data is ready for reinforcement learning training")
    
    # Show RL readiness
    print("\n🤖 RL Agent Readiness Check:")
    ready_checks = [
        ("Trace data collected", len(pipeline.traces) > 0),
        ("User feedback present", summary['rewarded_count'] > 0),
        ("Multiple intents covered", len(intent_counts) > 1),
        ("Multiple models tested", len(model_stats) > 1),
        ("Positive & negative rewards", 
         any(t.reward > 0 for t in pipeline.traces if t.reward) and 
         any(t.reward < 0 for t in pipeline.traces if t.reward))
    ]
    
    all_ready = True
    for check, passed in ready_checks:
        status = "✅" if passed else "❌"
        print(f"  {status} {check}")
        all_ready = all_ready and passed
    
    if all_ready:
        print("\n🎉 System is ready for reinforcement learning!")
        print("   Run: python rl_agent/agent_skeleton.py")
    else:
        print("\n⚠️  Collect more diverse data before training")
    
    print("\n✨ Demo complete!")

if __name__ == "__main__":
    main()