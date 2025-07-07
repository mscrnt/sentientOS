#!/usr/bin/env python3
"""
Generate bootstrap traces for RL training
Creates a diverse set of execution traces with realistic feedback patterns
"""

import json
import random
import uuid
from datetime import datetime, timedelta
from pathlib import Path
from typing import List, Dict, Any

# Diverse prompt categories with expected intents and feedback patterns
PROMPT_CATEGORIES = {
    "general_knowledge": {
        "intent": "GeneralKnowledge",
        "prompts": [
            "What is virtual memory and how does it work?",
            "Explain the difference between TCP and UDP",
            "How does garbage collection work in modern programming languages?",
            "What are the benefits of using containers?",
            "Describe the CAP theorem in distributed systems",
            "What is a kernel panic?",
            "How do SSDs differ from HDDs?",
            "Explain DNS resolution process",
            "What is a race condition?",
            "How does HTTPS encryption work?",
        ],
        "models": ["llama3.2", "gpt-4o-mini"],
        "feedback_bias": 0.7,  # 70% positive
        "rag_probability": 0.9,
    },
    
    "tool_calls": {
        "intent": "ToolCall",
        "prompts": [
            "Check disk usage",
            "Show current memory statistics",
            "List all running processes",
            "!@ call network_status",
            "Display CPU information",
            "Check system uptime",
            "Show open network connections",
            "Get filesystem mount points",
            "Display system load average",
            "Check available system updates",
        ],
        "models": ["phi2_local"],  # Trusted for tools
        "feedback_bias": 0.8,  # 80% positive
        "rag_probability": 0.1,
        "tools": ["disk_info", "memory_usage", "process_list", "network_status", "system_info"],
    },
    
    "code_generation": {
        "intent": "CodeGeneration",
        "prompts": [
            "Write a Python script to monitor CPU temperature",
            "Generate a bash function to backup important files",
            "Create a systemd service file for a web app",
            "Write a script to alert when disk space is low",
            "Generate code to parse JSON logs",
            "Create a Python decorator for timing functions",
            "Write a shell script to rotate logs",
            "Generate a Docker compose file for a LAMP stack",
            "Create a function to validate email addresses",
            "Write a script to find large files recursively",
        ],
        "models": ["qwen2.5", "gpt-4o-mini"],
        "feedback_bias": 0.6,  # 60% positive
        "rag_probability": 0.3,
    },
    
    "analysis": {
        "intent": "Analysis",
        "prompts": [
            "Why is my system running slowly?",
            "Analyze this error: connection refused on port 443",
            "Debug segmentation fault in my C program",
            "Why is memory usage constantly increasing?",
            "Investigate high CPU usage by process X",
            "Analyze network latency issues",
            "Why does my application crash randomly?",
            "Debug permission denied errors",
            "Analyze slow database queries",
            "Why is my disk I/O so high?",
        ],
        "models": ["gpt-4o-mini", "llama3.2"],
        "feedback_bias": 0.5,  # 50% positive
        "rag_probability": 0.7,
    },
    
    "hybrid_queries": {
        "intent": "QueryThenAction",
        "prompts": [
            "Check if I need to clean cache and do it if necessary",
            "Monitor memory and suggest optimizations",
            "Show disk usage and recommend cleanup",
            "Check system health and fix any issues",
            "Analyze performance and apply optimizations",
            "Review security settings and harden system",
            "Check for updates and install if safe",
            "Monitor logs and alert on errors",
            "Verify backups and run if needed",
            "Check network security and close unnecessary ports",
        ],
        "models": ["llama3.2", "phi2_local", "gpt-4o-mini"],
        "feedback_bias": 0.65,  # 65% positive
        "rag_probability": 0.8,
        "conditions": ["high_memory", "disk_full", "security_issue"],
    },
    
    "edge_cases": {
        "intent": "Unknown",
        "prompts": [
            "",
            "help",
            "asdfghjkl",
            "😊",
            "SELECT * FROM users;",
            "<script>alert('test')</script>",
            "sudo rm -rf /",
            "What's the meaning of life?",
            "Tell me a joke",
            "12345",
        ],
        "models": ["phi2_local", "llama3.2"],
        "feedback_bias": 0.2,  # 20% positive
        "rag_probability": 0.3,
    },
}


def generate_trace(
    prompt: str,
    category: str,
    trace_num: int,
    session_id: str
) -> Dict[str, Any]:
    """Generate a single trace entry"""
    
    config = PROMPT_CATEGORIES[category]
    
    # Generate timestamp with some variance
    base_time = datetime.now() - timedelta(hours=random.randint(0, 48))
    timestamp = base_time + timedelta(seconds=trace_num * random.randint(30, 300))
    
    # Select model
    model = random.choice(config["models"])
    
    # Determine if RAG was used
    rag_used = random.random() < config["rag_probability"]
    
    # Determine tool execution
    tool_executed = None
    if "tools" in config and random.random() < 0.8:
        tool_executed = random.choice(config["tools"])
    elif config["intent"] == "ToolCall":
        tool_executed = "disk_info"  # Default tool
    
    # Conditions evaluated
    conditions_evaluated = []
    if "conditions" in config and rag_used and random.random() < 0.5:
        conditions_evaluated = [random.choice(config["conditions"])]
    
    # Success and duration
    success = random.random() < 0.95  # 95% success rate
    duration_ms = random.randint(50, 2000) if success else random.randint(10, 100)
    
    # Determine feedback
    feedback_roll = random.random()
    if feedback_roll < config["feedback_bias"]:
        reward = 1.0
    elif feedback_roll < config["feedback_bias"] + 0.2:  # 20% skip rate
        reward = None
    else:
        reward = -1.0
    
    # Add some realistic variance
    if not success:
        reward = -1.0  # Failed executions always get negative feedback
    
    # Fallback simulation
    fallback_used = model != config["models"][0] and random.random() < 0.1
    
    trace = {
        "trace_id": f"{session_id}-{trace_num:04d}",
        "timestamp": timestamp.isoformat(),
        "prompt": prompt,
        "intent": config["intent"],
        "model_used": model,
        "tool_executed": tool_executed,
        "rag_used": rag_used,
        "conditions_evaluated": conditions_evaluated,
        "success": success,
        "duration_ms": duration_ms,
        "reward": reward,
        "fallback_used": fallback_used,
        "error_occurred": not success,
        "user_satisfaction": "satisfied" if reward == 1.0 else "unsatisfied" if reward == -1.0 else None
    }
    
    return trace


def generate_bootstrap_traces(num_traces: int = 100) -> List[Dict[str, Any]]:
    """Generate a diverse set of bootstrap traces"""
    
    traces = []
    session_id = str(uuid.uuid4())[:8]
    
    # Calculate traces per category
    categories = list(PROMPT_CATEGORIES.keys())
    traces_per_category = num_traces // len(categories)
    remainder = num_traces % len(categories)
    
    trace_num = 0
    
    for i, category in enumerate(categories):
        # Add extra trace to first categories to handle remainder
        category_count = traces_per_category + (1 if i < remainder else 0)
        
        prompts = PROMPT_CATEGORIES[category]["prompts"]
        
        for j in range(category_count):
            # Cycle through prompts, with some repetition for variance
            prompt = prompts[j % len(prompts)]
            
            # Occasionally modify prompt slightly
            if random.random() < 0.2:
                modifiers = ["please ", "can you ", "I need to ", "help me "]
                prompt = random.choice(modifiers) + prompt.lower()
            
            trace = generate_trace(prompt, category, trace_num, session_id)
            traces.append(trace)
            trace_num += 1
    
    # Shuffle to mix categories
    random.shuffle(traces)
    
    # Re-number after shuffle
    for i, trace in enumerate(traces):
        trace["trace_id"] = f"{session_id}-{i:04d}"
    
    return traces


def validate_traces(traces: List[Dict[str, Any]]) -> Dict[str, Any]:
    """Validate trace quality and coverage"""
    
    stats = {
        "total": len(traces),
        "valid": 0,
        "invalid": 0,
        "intents": {},
        "models": {},
        "tools": {},
        "rewards": {"positive": 0, "negative": 0, "none": 0},
        "rag_used": 0,
        "tool_used": 0,
        "success_rate": 0,
    }
    
    required_fields = [
        "trace_id", "timestamp", "prompt", "intent", "model_used",
        "rag_used", "success", "duration_ms"
    ]
    
    for trace in traces:
        # Check required fields
        valid = all(field in trace for field in required_fields)
        
        if valid:
            stats["valid"] += 1
            
            # Count intents
            intent = trace["intent"]
            stats["intents"][intent] = stats["intents"].get(intent, 0) + 1
            
            # Count models
            model = trace["model_used"]
            stats["models"][model] = stats["models"].get(model, 0) + 1
            
            # Count tools
            if trace.get("tool_executed"):
                tool = trace["tool_executed"]
                stats["tools"][tool] = stats["tools"].get(tool, 0) + 1
                stats["tool_used"] += 1
            
            # Count rewards
            reward = trace.get("reward")
            if reward == 1.0:
                stats["rewards"]["positive"] += 1
            elif reward == -1.0:
                stats["rewards"]["negative"] += 1
            else:
                stats["rewards"]["none"] += 1
            
            # Other stats
            if trace.get("rag_used"):
                stats["rag_used"] += 1
            
            if trace.get("success"):
                stats["success_rate"] += 1
        else:
            stats["invalid"] += 1
    
    stats["success_rate"] = stats["success_rate"] / max(1, stats["valid"])
    
    return stats


def split_dataset(traces: List[Dict[str, Any]], train_ratio: float = 0.8) -> tuple:
    """Split traces into train and test sets"""
    
    # Shuffle before splitting
    shuffled = traces.copy()
    random.shuffle(shuffled)
    
    # Calculate split point
    split_idx = int(len(shuffled) * train_ratio)
    
    train_set = shuffled[:split_idx]
    test_set = shuffled[split_idx:]
    
    return train_set, test_set


def main():
    """Generate bootstrap traces for RL training"""
    
    print("🚀 SentientOS Bootstrap Trace Generator")
    print("="*60)
    
    # Configuration
    num_traces = 100  # Target number of traces
    output_dir = Path("traces")
    rl_data_dir = Path("rl_data")
    
    # Create directories
    output_dir.mkdir(exist_ok=True)
    rl_data_dir.mkdir(exist_ok=True)
    
    # Generate traces
    print(f"\n📝 Generating {num_traces} bootstrap traces...")
    traces = generate_bootstrap_traces(num_traces)
    
    # Validate traces
    print("\n🔍 Validating traces...")
    stats = validate_traces(traces)
    
    print(f"\n📊 Validation Results:")
    print(f"  Total: {stats['total']}")
    print(f"  Valid: {stats['valid']}")
    print(f"  Invalid: {stats['invalid']}")
    print(f"\n  Intents: {dict(stats['intents'])}")
    print(f"  Models: {dict(stats['models'])}")
    print(f"  Tools: {dict(stats['tools'])}")
    print(f"\n  Rewards: +{stats['rewards']['positive']} / -{stats['rewards']['negative']} / skip {stats['rewards']['none']}")
    print(f"  RAG Used: {stats['rag_used']} ({stats['rag_used']/stats['valid']*100:.1f}%)")
    print(f"  Tool Used: {stats['tool_used']} ({stats['tool_used']/stats['valid']*100:.1f}%)")
    print(f"  Success Rate: {stats['success_rate']*100:.1f}%")
    
    # Save raw traces
    trace_file = output_dir / "trace_log.jsonl"
    print(f"\n💾 Saving traces to {trace_file}...")
    with open(trace_file, 'w') as f:
        for trace in traces:
            json.dump(trace, f)
            f.write('\n')
    
    # Split dataset
    print("\n🔀 Splitting dataset (80/20)...")
    train_set, test_set = split_dataset(traces, train_ratio=0.8)
    
    # Save train/test sets
    train_file = rl_data_dir / "train.jsonl"
    test_file = rl_data_dir / "test.jsonl"
    
    with open(train_file, 'w') as f:
        for trace in train_set:
            json.dump(trace, f)
            f.write('\n')
    
    with open(test_file, 'w') as f:
        for trace in test_set:
            json.dump(trace, f)
            f.write('\n')
    
    print(f"  Train set: {len(train_set)} traces → {train_file}")
    print(f"  Test set: {len(test_set)} traces → {test_file}")
    
    # Check coverage criteria
    print("\n✅ Coverage Criteria Check:")
    criteria = [
        ("4+ intents covered", len(stats['intents']) >= 4),
        ("3+ models used", len(stats['models']) >= 3),
        ("Positive rewards present", stats['rewards']['positive'] > 0),
        ("Negative rewards present", stats['rewards']['negative'] > 0),
        ("RAG cases included", stats['rag_used'] > 0),
        ("Tool cases included", stats['tool_used'] > 0),
        ("100+ valid traces", stats['valid'] >= 100),
    ]
    
    all_passed = True
    for criterion, passed in criteria:
        status = "✅" if passed else "❌"
        print(f"  {status} {criterion}")
        all_passed = all_passed and passed
    
    if all_passed:
        print("\n🎉 All criteria met! Dataset ready for RL training.")
        print("\n📚 Next steps:")
        print("  1. Review traces: less traces/trace_log.jsonl")
        print("  2. Start training: python rl_agent/train_agent.py")
    else:
        print("\n⚠️  Some criteria not met. Consider generating more traces.")
    
    print("\n✨ Bootstrap generation complete!")


if __name__ == "__main__":
    main()