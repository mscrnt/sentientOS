#!/usr/bin/env python3
"""
Evaluate trained PPO agent and compare with baseline
Includes failure tracking and A/B testing
"""

import json
import pickle
import numpy as np
from pathlib import Path
from typing import Dict, List, Tuple, Any
from collections import defaultdict
import matplotlib.pyplot as plt
import seaborn as sns

import torch
from torch.utils.data import DataLoader

from ppo_agent import PPOAgent, TraceDataset, RLAction, create_encoders


class PolicyEvaluator:
    """Evaluate RL policy performance"""
    
    def __init__(self, agent: PPOAgent, encoders: Dict[str, Any], device: str = "cpu"):
        self.agent = agent
        self.encoders = encoders
        self.device = device
        
        # Metrics storage
        self.metrics = {
            'accuracy': {'overall': 0, 'by_intent': {}},
            'reward_improvement': 0,
            'model_selection': defaultdict(int),
            'tool_selection': defaultdict(int),
            'failures': []
        }
    
    def evaluate_dataset(self, dataset: TraceDataset, baseline_policy=None) -> Dict[str, Any]:
        """Evaluate agent on a dataset"""
        dataloader = DataLoader(dataset, batch_size=1, shuffle=False)
        
        correct_predictions = 0
        total_predictions = 0
        rewards_agent = []
        rewards_baseline = []
        
        intent_accuracy = defaultdict(lambda: {'correct': 0, 'total': 0})
        
        for i, batch in enumerate(dataloader):
            state = batch['state'].to(self.device)
            true_action = batch['action'].item()
            reward = batch['reward'].item()
            
            # Get agent's action
            with torch.no_grad():
                action_logits, value = self.agent.policy(state)
                action_probs = torch.softmax(action_logits, dim=-1)
                agent_action = torch.argmax(action_probs).item()
            
            # Decode actions
            true_action_obj = RLAction.from_index(
                true_action, 
                self.encoders['models'], 
                self.encoders['tools']
            )
            agent_action_obj = RLAction.from_index(
                agent_action,
                self.encoders['models'],
                self.encoders['tools']
            )
            
            # Check if prediction matches
            action_match = (
                agent_action_obj.model == true_action_obj.model and
                agent_action_obj.use_rag == true_action_obj.use_rag and
                agent_action_obj.tool == true_action_obj.tool
            )
            
            if action_match:
                correct_predictions += 1
            else:
                # Track failure
                trace = dataset.traces[i]
                failure = {
                    'trace_id': trace.get('trace_id', f'trace_{i}'),
                    'prompt': trace['prompt'],
                    'intent': trace['intent'],
                    'true_action': asdict(true_action_obj),
                    'agent_action': asdict(agent_action_obj),
                    'reward': reward,
                    'confidence': float(action_probs.max())
                }
                self.metrics['failures'].append(failure)
            
            total_predictions += 1
            
            # Track by intent
            intent = dataset.traces[i]['intent']
            intent_accuracy[intent]['total'] += 1
            if action_match:
                intent_accuracy[intent]['correct'] += 1
            
            # Track selections
            self.metrics['model_selection'][agent_action_obj.model] += 1
            if agent_action_obj.tool:
                self.metrics['tool_selection'][agent_action_obj.tool] += 1
            
            # Collect rewards
            rewards_agent.append(reward if action_match else -0.5)  # Penalty for wrong action
            
            # Baseline comparison (if provided)
            if baseline_policy:
                baseline_action = baseline_policy(dataset.traces[i])
                baseline_match = (
                    baseline_action.model == true_action_obj.model and
                    baseline_action.use_rag == true_action_obj.use_rag and
                    baseline_action.tool == true_action_obj.tool
                )
                rewards_baseline.append(reward if baseline_match else -0.5)
        
        # Calculate metrics
        self.metrics['accuracy']['overall'] = correct_predictions / total_predictions
        
        for intent, stats in intent_accuracy.items():
            if stats['total'] > 0:
                self.metrics['accuracy']['by_intent'][intent] = stats['correct'] / stats['total']
        
        # Reward improvement
        avg_reward_agent = np.mean(rewards_agent)
        if rewards_baseline:
            avg_reward_baseline = np.mean(rewards_baseline)
            self.metrics['reward_improvement'] = avg_reward_agent - avg_reward_baseline
        else:
            self.metrics['reward_improvement'] = avg_reward_agent
        
        return {
            'accuracy': self.metrics['accuracy']['overall'],
            'avg_reward': avg_reward_agent,
            'num_failures': len(self.metrics['failures']),
            'intent_accuracy': dict(self.metrics['accuracy']['by_intent'])
        }
    
    def save_failures(self, output_path: str = "rl_agent/failures.jsonl"):
        """Save failure cases for analysis"""
        with open(output_path, 'w') as f:
            for failure in self.metrics['failures']:
                json.dump(failure, f)
                f.write('\n')
        print(f"Saved {len(self.metrics['failures'])} failure cases to {output_path}")
    
    def generate_report(self) -> str:
        """Generate evaluation report"""
        report = []
        report.append("# RL Agent Evaluation Report\n")
        
        # Overall metrics
        report.append("## Overall Performance")
        report.append(f"- **Accuracy**: {self.metrics['accuracy']['overall']:.2%}")
        report.append(f"- **Reward Improvement**: {self.metrics['reward_improvement']:.3f}")
        report.append(f"- **Total Failures**: {len(self.metrics['failures'])}\n")
        
        # Intent-wise accuracy
        report.append("## Accuracy by Intent")
        for intent, acc in sorted(self.metrics['accuracy']['by_intent'].items()):
            report.append(f"- {intent}: {acc:.2%}")
        report.append("")
        
        # Model selection distribution
        report.append("## Model Selection Distribution")
        total_selections = sum(self.metrics['model_selection'].values())
        for model, count in sorted(self.metrics['model_selection'].items(), 
                                  key=lambda x: x[1], reverse=True):
            percentage = (count / total_selections) * 100 if total_selections > 0 else 0
            report.append(f"- {model}: {count} ({percentage:.1f}%)")
        report.append("")
        
        # Tool usage
        report.append("## Tool Usage")
        for tool, count in sorted(self.metrics['tool_selection'].items(),
                                key=lambda x: x[1], reverse=True):
            report.append(f"- {tool}: {count}")
        report.append("")
        
        # Top failures
        report.append("## Top Failure Patterns")
        if self.metrics['failures']:
            # Group failures by pattern
            failure_patterns = defaultdict(list)
            for failure in self.metrics['failures'][:20]:  # Top 20
                pattern = f"{failure['intent']} - {failure['true_action']['model']} → {failure['agent_action']['model']}"
                failure_patterns[pattern].append(failure)
            
            for pattern, failures in sorted(failure_patterns.items(), 
                                          key=lambda x: len(x[1]), reverse=True)[:5]:
                report.append(f"\n### {pattern} ({len(failures)} cases)")
                example = failures[0]
                report.append(f"- Example: \"{example['prompt'][:60]}...\"")
                report.append(f"- Confidence: {example['confidence']:.2f}")
        
        return "\n".join(report)


def baseline_policy(trace: Dict[str, Any]) -> RLAction:
    """Simple baseline policy for comparison"""
    intent = trace['intent']
    
    # Simple heuristic rules
    if intent == 'ToolCall':
        return RLAction(model='llama3.2:latest', use_rag=False, tool='disk_info')
    elif intent == 'CodeGeneration':
        return RLAction(model='deepseek-v2:16b', use_rag=False, tool=None)
    elif intent == 'Analysis':
        return RLAction(model='deepseek-v2:16b', use_rag=True, tool=None)
    else:
        return RLAction(model='deepseek-v2:16b', use_rag=True, tool=None)


def plot_training_curves(history: Dict[str, List[float]], save_path: str = "rl_agent/training_curves.png"):
    """Plot training curves"""
    fig, axes = plt.subplots(2, 2, figsize=(12, 10))
    fig.suptitle('PPO Training Progress', fontsize=16)
    
    # Rewards
    axes[0, 0].plot(history['rewards'], 'b-', linewidth=2)
    axes[0, 0].set_title('Average Reward')
    axes[0, 0].set_xlabel('Epoch')
    axes[0, 0].set_ylabel('Reward')
    axes[0, 0].grid(True, alpha=0.3)
    
    # Policy Loss
    axes[0, 1].plot(history['policy_losses'], 'r-', linewidth=2)
    axes[0, 1].set_title('Policy Loss')
    axes[0, 1].set_xlabel('Epoch')
    axes[0, 1].set_ylabel('Loss')
    axes[0, 1].grid(True, alpha=0.3)
    
    # Value Loss
    axes[1, 0].plot(history['value_losses'], 'g-', linewidth=2)
    axes[1, 0].set_title('Value Loss')
    axes[1, 0].set_xlabel('Epoch')
    axes[1, 0].set_ylabel('Loss')
    axes[1, 0].grid(True, alpha=0.3)
    
    # Entropy
    axes[1, 1].plot(history['entropies'], 'm-', linewidth=2)
    axes[1, 1].set_title('Policy Entropy')
    axes[1, 1].set_xlabel('Epoch')
    axes[1, 1].set_ylabel('Entropy')
    axes[1, 1].grid(True, alpha=0.3)
    
    plt.tight_layout()
    plt.savefig(save_path, dpi=150)
    plt.close()
    print(f"Training curves saved to {save_path}")


def run_ab_test(agent: PPOAgent, test_dataset: TraceDataset, encoders: Dict[str, Any]) -> Dict[str, Any]:
    """Run A/B test comparing agent with baseline"""
    print("\n🧪 Running A/B Test...")
    
    # Evaluate agent (B)
    evaluator_agent = PolicyEvaluator(agent, encoders)
    results_agent = evaluator_agent.evaluate_dataset(test_dataset, baseline_policy)
    
    # Evaluate baseline (A) 
    correct_baseline = 0
    rewards_baseline = []
    
    for i, trace in enumerate(test_dataset.traces):
        true_action = RLAction(
            model=trace['model_used'],
            use_rag=trace['rag_used'],
            tool=trace.get('tool_executed')
        )
        
        baseline_action = baseline_policy(trace)
        
        if (baseline_action.model == true_action.model and
            baseline_action.use_rag == true_action.use_rag and
            baseline_action.tool == true_action.tool):
            correct_baseline += 1
            rewards_baseline.append(trace.get('reward', 0))
        else:
            rewards_baseline.append(-0.5)
    
    accuracy_baseline = correct_baseline / len(test_dataset)
    avg_reward_baseline = np.mean(rewards_baseline)
    
    # Compare results
    ab_results = {
        'baseline': {
            'accuracy': accuracy_baseline,
            'avg_reward': avg_reward_baseline
        },
        'agent': {
            'accuracy': results_agent['accuracy'],
            'avg_reward': results_agent['avg_reward']
        },
        'improvement': {
            'accuracy': results_agent['accuracy'] - accuracy_baseline,
            'reward': results_agent['avg_reward'] - avg_reward_baseline
        }
    }
    
    return ab_results, evaluator_agent


def main():
    """Main evaluation script"""
    print("🔍 Evaluating PPO Agent...")
    
    # Load test data
    test_path = Path("rl_data/test.jsonl")
    test_traces = []
    with open(test_path, 'r') as f:
        for line in f:
            test_traces.append(json.loads(line))
    
    # Load encoders
    with open('rl_agent/encoders.pkl', 'rb') as f:
        encoders = pickle.load(f)
    
    # Create test dataset
    test_dataset = TraceDataset(test_traces, encoders)
    
    # Load trained agent
    agent = PPOAgent(encoders['state_dim'], encoders['action_dim'])
    agent.load_checkpoint('rl_agent/rl_policy.pth')
    
    # Plot training curves
    plot_training_curves(agent.training_history)
    
    # Run evaluation
    evaluator = PolicyEvaluator(agent, encoders)
    results = evaluator.evaluate_dataset(test_dataset, baseline_policy)
    
    print(f"\n📊 Evaluation Results:")
    print(f"  Overall Accuracy: {results['accuracy']:.2%}")
    print(f"  Average Reward: {results['avg_reward']:.3f}")
    print(f"  Number of Failures: {results['num_failures']}")
    
    # Save failures
    evaluator.save_failures()
    
    # Run A/B test
    ab_results, _ = run_ab_test(agent, test_dataset, encoders)
    
    print(f"\n📊 A/B Test Results:")
    print(f"  Baseline - Accuracy: {ab_results['baseline']['accuracy']:.2%}, Reward: {ab_results['baseline']['avg_reward']:.3f}")
    print(f"  Agent    - Accuracy: {ab_results['agent']['accuracy']:.2%}, Reward: {ab_results['agent']['avg_reward']:.3f}")
    print(f"  Improvement - Accuracy: {ab_results['improvement']['accuracy']:+.2%}, Reward: {ab_results['improvement']['reward']:+.3f}")
    
    # Generate and save reports
    report = evaluator.generate_report()
    with open('rl_agent/evaluation_report.md', 'w') as f:
        f.write(report)
    
    # Save A/B test results
    with open('rl_agent/ab_test_results.json', 'w') as f:
        json.dump(ab_results, f, indent=2)
    
    print("\n✅ Evaluation complete! Check rl_agent/ for detailed reports.")


if __name__ == "__main__":
    main()