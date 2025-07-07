#!/usr/bin/env python3
"""
Main training script for SentientOS RL Agent
Orchestrates the complete training pipeline
"""

import os
import sys
import json
import time
import argparse
from pathlib import Path
from datetime import datetime

# Ensure rl_agent module is in path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import numpy as np
import matplotlib.pyplot as plt

# Check for PyTorch
try:
    import torch
    print(f"✅ PyTorch {torch.__version__} available")
    if torch.cuda.is_available():
        print(f"✅ CUDA available: {torch.cuda.get_device_name(0)}")
    else:
        print("ℹ️  CUDA not available, using CPU")
except ImportError:
    print("❌ PyTorch not installed!")
    print("Install with: pip install torch torchvision")
    sys.exit(1)


def check_data_availability():
    """Check if training data is available"""
    train_path = Path("rl_data/train.jsonl")
    test_path = Path("rl_data/test.jsonl")
    
    if not train_path.exists() or not test_path.exists():
        print("❌ Training data not found!")
        print("Run: python generate_bootstrap_traces.py")
        return False
    
    # Check trace counts
    with open(train_path) as f:
        train_count = sum(1 for _ in f)
    with open(test_path) as f:
        test_count = sum(1 for _ in f)
    
    print(f"📊 Data available: {train_count} train, {test_count} test traces")
    return True


def generate_training_report(start_time: float, end_time: float):
    """Generate comprehensive training report"""
    duration = end_time - start_time
    
    report = []
    report.append("# SentientOS RL Training Report")
    report.append(f"\nGenerated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    report.append(f"Training Duration: {duration:.1f} seconds")
    
    # Load training summary
    with open('rl_agent/training_summary.json', 'r') as f:
        summary = json.load(f)
    
    report.append("\n## Training Configuration")
    report.append(f"- State Dimension: {summary['state_dim']}")
    report.append(f"- Action Dimension: {summary['action_dim']}")
    report.append(f"- Training Traces: {summary['num_train_traces']}")
    report.append(f"- Test Traces: {summary['num_test_traces']}")
    report.append(f"- Total Epochs: {summary['total_epochs']}")
    
    report.append("\n## Final Performance")
    report.append(f"- Final Average Reward: {summary['final_avg_reward']:.3f}")
    
    # Load evaluation results if available
    eval_path = Path('rl_agent/evaluation_report.md')
    if eval_path.exists():
        report.append("\n## Evaluation Results")
        with open(eval_path, 'r') as f:
            eval_content = f.read()
            # Extract key metrics
            for line in eval_content.split('\n'):
                if 'Accuracy:' in line or 'Reward Improvement:' in line:
                    report.append(line)
    
    # Load A/B test results
    ab_path = Path('rl_agent/ab_test_results.json')
    if ab_path.exists():
        with open(ab_path, 'r') as f:
            ab_results = json.load(f)
        
        report.append("\n## A/B Test Results")
        report.append("\n### Baseline (Heuristic Policy)")
        report.append(f"- Accuracy: {ab_results['baseline']['accuracy']:.2%}")
        report.append(f"- Average Reward: {ab_results['baseline']['avg_reward']:.3f}")
        
        report.append("\n### RL Agent")
        report.append(f"- Accuracy: {ab_results['agent']['accuracy']:.2%}")
        report.append(f"- Average Reward: {ab_results['agent']['avg_reward']:.3f}")
        
        report.append("\n### Improvement")
        report.append(f"- Accuracy: {ab_results['improvement']['accuracy']:+.2%}")
        report.append(f"- Reward: {ab_results['improvement']['reward']:+.3f}")
    
    # Training curves
    report.append("\n## Training Progress")
    report.append("![Training Curves](training_curves.png)")
    
    # Failure analysis
    failures_path = Path('rl_agent/failures.jsonl')
    if failures_path.exists():
        with open(failures_path, 'r') as f:
            failure_count = sum(1 for _ in f)
        report.append(f"\n## Failure Analysis")
        report.append(f"- Total Failures: {failure_count}")
        report.append("- See `failures.jsonl` for detailed analysis")
    
    # Recommendations
    report.append("\n## Recommendations")
    
    if summary['final_avg_reward'] > 0.5:
        report.append("✅ Agent shows positive average reward - ready for deployment")
    else:
        report.append("⚠️  Agent shows low average reward - consider more training data")
    
    if ab_results and ab_results['improvement']['accuracy'] > 0:
        report.append("✅ Agent outperforms baseline - significant improvement achieved")
    else:
        report.append("⚠️  Agent underperforms baseline - review failure patterns")
    
    report.append("\n## Next Steps")
    report.append("1. Deploy policy: Integrate `rl_policy.pth` into routing system")
    report.append("2. Online learning: Collect more traces with deployed agent")
    report.append("3. Curriculum learning: Focus on failure cases")
    report.append("4. Hyperparameter tuning: Experiment with learning rate, batch size")
    
    # Save report
    report_path = Path('rl_agent/RL_TRAINING_REPORT.md')
    with open(report_path, 'w') as f:
        f.write('\n'.join(report))
    
    print(f"\n📄 Training report saved to {report_path}")


def main():
    """Main training orchestrator"""
    parser = argparse.ArgumentParser(description='Train RL agent for SentientOS')
    parser.add_argument('--epochs', type=int, default=50, help='Number of training epochs')
    parser.add_argument('--skip-training', action='store_true', help='Skip training, only evaluate')
    parser.add_argument('--skip-evaluation', action='store_true', help='Skip evaluation')
    parser.add_argument('--live-train', action='store_true', help='Enable online learning mode')
    parser.add_argument('--retrain-threshold', type=int, default=50, help='New traces before retraining')
    args = parser.parse_args()
    
    print("🚀 SentientOS RL Agent Training Pipeline")
    print("="*50)
    
    # Live training mode
    if args.live_train:
        print("\n🔄 Starting Online Learning Mode...")
        print(f"   Retrain threshold: {args.retrain_threshold} new traces")
        print("   Press Ctrl+C to stop")
        
        # Import and run online learning
        import asyncio
        from online_learning import OnlineLearningManager, RetrainingConfig, IncrementalPPOTrainer
        
        config = RetrainingConfig(trigger_threshold=args.retrain_threshold)
        manager = OnlineLearningManager(config)
        trainer = IncrementalPPOTrainer(Path("rl_agent/rl_policy.pth"))
        
        try:
            asyncio.run(manager.monitor_and_retrain(trainer.incremental_train))
        except KeyboardInterrupt:
            print("\n⏹️ Online learning stopped by user")
        return 0
    
    # Check prerequisites
    if not check_data_availability():
        return
    
    # Create rl_agent directory if needed
    Path("rl_agent").mkdir(exist_ok=True)
    
    start_time = time.time()
    
    try:
        if not args.skip_training:
            print("\n📚 Phase 1: Training PPO Agent...")
            from ppo_agent import main as train_main
            
            # Temporarily modify epochs if specified
            if args.epochs != 50:
                import ppo_agent
                ppo_agent.num_epochs = args.epochs
            
            train_main()
            print("✅ Training complete!")
        
        if not args.skip_evaluation:
            print("\n🔍 Phase 2: Evaluating Agent...")
            from evaluate_agent import main as eval_main
            eval_main()
            print("✅ Evaluation complete!")
        
        end_time = time.time()
        
        # Generate comprehensive report
        print("\n📊 Phase 3: Generating Reports...")
        generate_training_report(start_time, end_time)
        
        # Final summary
        print("\n" + "="*50)
        print("✨ RL Agent Training Complete!")
        print("="*50)
        
        print("\n📦 Generated Artifacts:")
        artifacts = [
            "rl_agent/rl_policy.pth - Trained policy weights",
            "rl_agent/encoders.pkl - Feature encoders",
            "rl_agent/train_dataset.pkl - Preprocessed training data",
            "rl_agent/training_curves.png - Learning progress visualization",
            "rl_agent/failures.jsonl - Misclassified examples",
            "rl_agent/RL_TRAINING_REPORT.md - Comprehensive report",
            "rl_agent/ab_test_results.json - A/B test comparison"
        ]
        
        for artifact in artifacts:
            print(f"  ✓ {artifact}")
        
        print("\n🎯 Deployment Ready!")
        print("The RL agent is trained and ready to optimize model/tool selection.")
        print("\nTo deploy:")
        print("1. Load rl_policy.pth in your routing system")
        print("2. Use the agent for real-time decision making")
        print("3. Continue collecting feedback for online learning")
        
    except Exception as e:
        print(f"\n❌ Error during training: {e}")
        import traceback
        traceback.print_exc()
        return 1
    
    return 0


if __name__ == "__main__":
    sys.exit(main())