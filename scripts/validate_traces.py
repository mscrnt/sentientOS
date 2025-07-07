#!/usr/bin/env python3
"""
Validate and analyze trace logs for RL training readiness
"""

import json
import sys
from pathlib import Path
from typing import List, Dict, Any, Set
from collections import defaultdict
import statistics

def load_traces(file_path: str) -> List[Dict[str, Any]]:
    """Load traces from JSONL file"""
    traces = []
    with open(file_path, 'r') as f:
        for i, line in enumerate(f):
            try:
                trace = json.loads(line.strip())
                traces.append(trace)
            except json.JSONDecodeError as e:
                print(f"⚠️  Error parsing line {i+1}: {e}")
    return traces

def validate_trace_fields(trace: Dict[str, Any], required_fields: Set[str]) -> List[str]:
    """Validate a single trace has required fields"""
    errors = []
    
    for field in required_fields:
        if field not in trace:
            errors.append(f"Missing required field: {field}")
    
    # Type validation
    if "timestamp" in trace:
        try:
            # Check if timestamp is parseable
            from datetime import datetime
            datetime.fromisoformat(trace["timestamp"].replace('Z', '+00:00'))
        except:
            errors.append("Invalid timestamp format")
    
    if "reward" in trace and trace["reward"] is not None:
        if not isinstance(trace["reward"], (int, float)):
            errors.append("Reward must be numeric or null")
        elif trace["reward"] not in [-1.0, 0.0, 1.0]:
            errors.append(f"Unusual reward value: {trace['reward']}")
    
    return errors

def analyze_traces(traces: List[Dict[str, Any]]) -> Dict[str, Any]:
    """Comprehensive analysis of trace dataset"""
    
    analysis = {
        "total_traces": len(traces),
        "valid_traces": 0,
        "invalid_traces": 0,
        "validation_errors": [],
        
        # Coverage metrics
        "intents": defaultdict(int),
        "models": defaultdict(int),
        "tools": defaultdict(int),
        
        # Execution metrics
        "success_count": 0,
        "failure_count": 0,
        "rag_used_count": 0,
        "tool_used_count": 0,
        "fallback_count": 0,
        "error_count": 0,
        
        # Feedback metrics
        "positive_rewards": 0,
        "negative_rewards": 0,
        "skipped_rewards": 0,
        "reward_distribution": [],
        
        # Performance metrics
        "duration_stats": {
            "mean": 0,
            "median": 0,
            "min": float('inf'),
            "max": 0,
            "by_intent": defaultdict(list)
        },
        
        # Quality metrics
        "unique_prompts": set(),
        "duplicate_prompts": 0,
        "empty_prompts": 0,
        
        # Condition coverage
        "conditions_seen": defaultdict(int),
        "condition_combinations": defaultdict(int),
    }
    
    required_fields = {
        "trace_id", "timestamp", "prompt", "intent", "model_used",
        "rag_used", "success", "duration_ms"
    }
    
    durations = []
    
    for trace in traces:
        # Validate fields
        errors = validate_trace_fields(trace, required_fields)
        if errors:
            analysis["invalid_traces"] += 1
            analysis["validation_errors"].extend(errors)
            continue
        
        analysis["valid_traces"] += 1
        
        # Intent analysis
        intent = trace.get("intent", "Unknown")
        analysis["intents"][intent] += 1
        
        # Model analysis
        model = trace.get("model_used", "unknown")
        analysis["models"][model] += 1
        
        # Tool analysis
        tool = trace.get("tool_executed")
        if tool:
            analysis["tools"][tool] += 1
            analysis["tool_used_count"] += 1
        
        # Execution analysis
        if trace.get("success", False):
            analysis["success_count"] += 1
        else:
            analysis["failure_count"] += 1
        
        if trace.get("rag_used", False):
            analysis["rag_used_count"] += 1
        
        if trace.get("fallback_used", False):
            analysis["fallback_count"] += 1
        
        if trace.get("error_occurred", False):
            analysis["error_count"] += 1
        
        # Feedback analysis
        reward = trace.get("reward")
        if reward == 1.0:
            analysis["positive_rewards"] += 1
        elif reward == -1.0:
            analysis["negative_rewards"] += 1
        elif reward is None:
            analysis["skipped_rewards"] += 1
        
        if reward is not None:
            analysis["reward_distribution"].append(reward)
        
        # Performance analysis
        duration = trace.get("duration_ms", 0)
        if duration > 0:
            durations.append(duration)
            analysis["duration_stats"]["min"] = min(analysis["duration_stats"]["min"], duration)
            analysis["duration_stats"]["max"] = max(analysis["duration_stats"]["max"], duration)
            analysis["duration_stats"]["by_intent"][intent].append(duration)
        
        # Quality analysis
        prompt = trace.get("prompt", "")
        if prompt:
            if prompt in analysis["unique_prompts"]:
                analysis["duplicate_prompts"] += 1
            else:
                analysis["unique_prompts"].add(prompt)
        else:
            analysis["empty_prompts"] += 1
        
        # Condition analysis
        conditions = trace.get("conditions_evaluated", [])
        for condition in conditions:
            analysis["conditions_seen"][condition] += 1
        if conditions:
            combo = ",".join(sorted(conditions))
            analysis["condition_combinations"][combo] += 1
    
    # Calculate statistics
    if durations:
        analysis["duration_stats"]["mean"] = statistics.mean(durations)
        analysis["duration_stats"]["median"] = statistics.median(durations)
    
    # Convert sets to counts for JSON serialization
    analysis["unique_prompts"] = len(analysis["unique_prompts"])
    
    return analysis

def print_analysis_report(analysis: Dict[str, Any]):
    """Print formatted analysis report"""
    
    print("\n" + "="*60)
    print("📊 TRACE DATASET ANALYSIS REPORT")
    print("="*60)
    
    # Overall statistics
    print(f"\n📈 Overall Statistics:")
    print(f"  Total Traces: {analysis['total_traces']}")
    print(f"  Valid Traces: {analysis['valid_traces']}")
    print(f"  Invalid Traces: {analysis['invalid_traces']}")
    
    if analysis['invalid_traces'] > 0:
        print(f"\n⚠️  Validation Errors:")
        error_counts = defaultdict(int)
        for error in analysis['validation_errors']:
            error_counts[error] += 1
        for error, count in error_counts.items():
            print(f"    {error}: {count}")
    
    # Coverage analysis
    print(f"\n🎯 Intent Coverage:")
    for intent, count in sorted(analysis['intents'].items()):
        percentage = (count / analysis['valid_traces']) * 100
        print(f"  {intent}: {count} ({percentage:.1f}%)")
    
    print(f"\n🤖 Model Usage:")
    for model, count in sorted(analysis['models'].items()):
        percentage = (count / analysis['valid_traces']) * 100
        print(f"  {model}: {count} ({percentage:.1f}%)")
    
    if analysis['tools']:
        print(f"\n🔧 Tool Execution:")
        for tool, count in sorted(analysis['tools'].items()):
            print(f"  {tool}: {count}")
    
    # Execution metrics
    print(f"\n⚡ Execution Metrics:")
    success_rate = (analysis['success_count'] / analysis['valid_traces']) * 100
    print(f"  Success Rate: {success_rate:.1f}% ({analysis['success_count']}/{analysis['valid_traces']})")
    print(f"  RAG Used: {analysis['rag_used_count']} ({(analysis['rag_used_count']/analysis['valid_traces'])*100:.1f}%)")
    print(f"  Tools Used: {analysis['tool_used_count']} ({(analysis['tool_used_count']/analysis['valid_traces'])*100:.1f}%)")
    print(f"  Fallbacks: {analysis['fallback_count']}")
    print(f"  Errors: {analysis['error_count']}")
    
    # Feedback analysis
    print(f"\n💬 Feedback Distribution:")
    total_feedback = analysis['positive_rewards'] + analysis['negative_rewards'] + analysis['skipped_rewards']
    if total_feedback > 0:
        print(f"  Positive: {analysis['positive_rewards']} ({(analysis['positive_rewards']/total_feedback)*100:.1f}%)")
        print(f"  Negative: {analysis['negative_rewards']} ({(analysis['negative_rewards']/total_feedback)*100:.1f}%)")
        print(f"  Skipped: {analysis['skipped_rewards']} ({(analysis['skipped_rewards']/total_feedback)*100:.1f}%)")
        
        if analysis['reward_distribution']:
            avg_reward = statistics.mean(analysis['reward_distribution'])
            print(f"  Average Reward: {avg_reward:.2f}")
    
    # Performance analysis
    print(f"\n⏱️  Performance Statistics:")
    if analysis['duration_stats']['mean'] > 0:
        print(f"  Mean Duration: {analysis['duration_stats']['mean']:.0f}ms")
        print(f"  Median Duration: {analysis['duration_stats']['median']:.0f}ms")
        print(f"  Min Duration: {analysis['duration_stats']['min']}ms")
        print(f"  Max Duration: {analysis['duration_stats']['max']}ms")
        
        print(f"\n  Duration by Intent:")
        for intent, durations in analysis['duration_stats']['by_intent'].items():
            if durations:
                avg_duration = statistics.mean(durations)
                print(f"    {intent}: {avg_duration:.0f}ms avg")
    
    # Quality metrics
    print(f"\n📝 Data Quality:")
    print(f"  Unique Prompts: {analysis['unique_prompts']}")
    print(f"  Duplicate Prompts: {analysis['duplicate_prompts']}")
    print(f"  Empty Prompts: {analysis['empty_prompts']}")
    
    # Condition coverage
    if analysis['conditions_seen']:
        print(f"\n🔍 Condition Coverage:")
        for condition, count in sorted(analysis['conditions_seen'].items()):
            print(f"  {condition}: {count}")

def check_rl_readiness(analysis: Dict[str, Any]) -> bool:
    """Check if dataset meets criteria for RL training"""
    
    print("\n" + "="*60)
    print("✅ RL TRAINING READINESS CHECK")
    print("="*60)
    
    criteria = {
        "Minimum 100 valid traces": analysis['valid_traces'] >= 100,
        "4+ different intents": len(analysis['intents']) >= 4,
        "3+ different models": len(analysis['models']) >= 3,
        "Both positive and negative rewards": 
            analysis['positive_rewards'] > 0 and analysis['negative_rewards'] > 0,
        "RAG cases present": analysis['rag_used_count'] > 0,
        "Tool execution cases": analysis['tool_used_count'] > 0,
        "Success rate > 80%": 
            (analysis['success_count'] / max(1, analysis['valid_traces'])) > 0.8,
        "Balanced rewards (20-80% positive)": 
            0.2 <= (analysis['positive_rewards'] / max(1, analysis['positive_rewards'] + analysis['negative_rewards'])) <= 0.8,
        "Low duplicate rate (<20%)": 
            (analysis['duplicate_prompts'] / max(1, analysis['unique_prompts'])) < 0.2,
    }
    
    all_passed = True
    for criterion, passed in criteria.items():
        status = "✅" if passed else "❌"
        print(f"  {status} {criterion}")
        all_passed = all_passed and passed
    
    if all_passed:
        print("\n🎉 Dataset is READY for RL training!")
        print("\n📚 Recommended next steps:")
        print("  1. Run train/test split: Already done in rl_data/")
        print("  2. Start agent training: python rl_agent/train_agent.py")
        print("  3. Monitor training metrics")
    else:
        print("\n⚠️  Dataset needs improvement before training")
        print("\n💡 Suggestions:")
        if analysis['valid_traces'] < 100:
            print("  - Collect more traces using trace_collector.py")
        if len(analysis['intents']) < 4:
            print("  - Include more diverse query types")
        if analysis['positive_rewards'] == 0 or analysis['negative_rewards'] == 0:
            print("  - Ensure balanced feedback (both positive and negative)")
    
    return all_passed

def main():
    """Main validation script"""
    
    # Parse arguments
    if len(sys.argv) > 1:
        trace_file = sys.argv[1]
    else:
        trace_file = "traces/trace_log.jsonl"
    
    # Check if file exists
    if not Path(trace_file).exists():
        print(f"❌ Error: Trace file not found: {trace_file}")
        print("\nUsage: python validate_traces.py [trace_file.jsonl]")
        sys.exit(1)
    
    print(f"🔍 Validating traces from: {trace_file}")
    
    # Load traces
    traces = load_traces(trace_file)
    
    if not traces:
        print("❌ No traces found in file!")
        sys.exit(1)
    
    # Analyze traces
    analysis = analyze_traces(traces)
    
    # Print report
    print_analysis_report(analysis)
    
    # Check RL readiness
    is_ready = check_rl_readiness(analysis)
    
    # Save analysis results
    analysis_file = Path(trace_file).parent / "trace_analysis.json"
    with open(analysis_file, 'w') as f:
        json.dump(analysis, f, indent=2, default=str)
    print(f"\n💾 Analysis saved to: {analysis_file}")
    
    # Exit with appropriate code
    sys.exit(0 if is_ready else 1)

if __name__ == "__main__":
    main()