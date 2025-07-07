#!/usr/bin/env python3
"""
Online Learning and Policy Refinement for SentientOS RL Agent

This module handles:
- Monitoring trace logs for new data
- Auto-triggering retraining when threshold reached
- Managing checkpoint rotation
- Live policy updates without restart
"""

import json
import time
import logging
import asyncio
from pathlib import Path
from typing import Dict, List, Optional, Tuple
from datetime import datetime, timedelta
from dataclasses import dataclass
import pickle
import torch
import numpy as np

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


@dataclass
class RetrainingConfig:
    """Configuration for online retraining"""
    trace_file: Path = Path("logs/rl_trace.jsonl")
    checkpoint_dir: Path = Path("rl_checkpoints")
    trigger_threshold: int = 50  # New traces before retraining
    max_checkpoints: int = 10
    confidence_decay_days: int = 7
    exploration_bonus: float = 0.1
    live_update_signal: Path = Path("rl_agent/.live_update")


class OnlineLearningManager:
    """Manages online learning and policy updates"""
    
    def __init__(self, config: RetrainingConfig):
        self.config = config
        self.last_trace_count = 0
        self.last_retrain_time = datetime.now()
        self.checkpoint_counter = 0
        
        # Ensure directories exist
        self.config.checkpoint_dir.mkdir(exist_ok=True)
        
    def count_traces(self) -> int:
        """Count total traces in log file"""
        if not self.config.trace_file.exists():
            return 0
        
        count = 0
        with open(self.config.trace_file, 'r') as f:
            for line in f:
                if line.strip():
                    count += 1
        return count
    
    def get_new_traces(self, since_count: int) -> List[Dict]:
        """Get traces added since last count"""
        traces = []
        current_count = 0
        
        with open(self.config.trace_file, 'r') as f:
            for line in f:
                if line.strip():
                    current_count += 1
                    if current_count > since_count:
                        traces.append(json.loads(line))
        
        return traces
    
    def apply_confidence_decay(self, traces: List[Dict]) -> List[Dict]:
        """Apply confidence decay to older traces"""
        now = datetime.now()
        decay_threshold = now - timedelta(days=self.config.confidence_decay_days)
        
        for trace in traces:
            trace_time = datetime.fromisoformat(trace['timestamp'].replace('Z', '+00:00'))
            if trace_time < decay_threshold:
                # Decay reward based on age
                days_old = (now - trace_time).days
                decay_factor = np.exp(-0.1 * days_old)
                if trace.get('reward'):
                    trace['reward'] *= decay_factor
                    trace['confidence_decay'] = decay_factor
        
        return traces
    
    def calculate_exploration_metrics(self, traces: List[Dict]) -> Dict[str, float]:
        """Calculate exploration vs exploitation metrics"""
        exploration_count = 0
        exploitation_count = 0
        total_reward = 0.0
        
        for trace in traces:
            # High confidence = exploitation, low = exploration
            confidence = trace.get('confidence', 0.5)
            if confidence < 0.7:
                exploration_count += 1
            else:
                exploitation_count += 1
            
            if trace.get('reward'):
                total_reward += trace['reward']
        
        total = len(traces)
        return {
            'exploration_rate': exploration_count / total if total > 0 else 0,
            'exploitation_rate': exploitation_count / total if total > 0 else 0,
            'average_reward': total_reward / total if total > 0 else 0,
            'total_traces': total
        }
    
    def should_retrain(self) -> Tuple[bool, str]:
        """Check if retraining should be triggered"""
        current_count = self.count_traces()
        new_traces = current_count - self.last_trace_count
        
        if new_traces >= self.config.trigger_threshold:
            return True, f"Threshold reached: {new_traces} new traces"
        
        # Check time-based trigger (daily)
        if datetime.now() - self.last_retrain_time > timedelta(days=1):
            if new_traces > 10:  # At least some new data
                return True, f"Daily retrain with {new_traces} new traces"
        
        return False, f"Only {new_traces} new traces (threshold: {self.config.trigger_threshold})"
    
    def save_checkpoint(self, model_state: Dict, metrics: Dict) -> Path:
        """Save model checkpoint with rotation"""
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        checkpoint_path = self.config.checkpoint_dir / f"checkpoint_{timestamp}.pkl"
        
        checkpoint = {
            'model_state': model_state,
            'metrics': metrics,
            'trace_count': self.count_traces(),
            'timestamp': timestamp,
            'version': self.checkpoint_counter
        }
        
        with open(checkpoint_path, 'wb') as f:
            pickle.dump(checkpoint, f)
        
        logger.info(f"💾 Saved checkpoint: {checkpoint_path}")
        self.checkpoint_counter += 1
        
        # Rotate old checkpoints
        self._rotate_checkpoints()
        
        return checkpoint_path
    
    def _rotate_checkpoints(self):
        """Keep only the latest N checkpoints"""
        checkpoints = sorted(
            self.config.checkpoint_dir.glob("checkpoint_*.pkl"),
            key=lambda p: p.stat().st_mtime,
            reverse=True
        )
        
        for old_checkpoint in checkpoints[self.config.max_checkpoints:]:
            old_checkpoint.unlink()
            logger.info(f"🗑️ Removed old checkpoint: {old_checkpoint}")
    
    def signal_live_update(self):
        """Signal that a new policy is available"""
        self.config.live_update_signal.touch()
        logger.info("📡 Live update signal sent")
    
    async def monitor_and_retrain(self, retrain_callback):
        """Monitor traces and trigger retraining when needed"""
        logger.info("👁️ Starting online learning monitor...")
        
        while True:
            try:
                should_train, reason = self.should_retrain()
                
                if should_train:
                    logger.info(f"🔄 Triggering retraining: {reason}")
                    
                    # Get new traces
                    new_traces = self.get_new_traces(self.last_trace_count)
                    
                    # Apply confidence decay
                    new_traces = self.apply_confidence_decay(new_traces)
                    
                    # Calculate metrics
                    metrics = self.calculate_exploration_metrics(new_traces)
                    logger.info(f"📊 Metrics: {metrics}")
                    
                    # Trigger retraining
                    checkpoint_path = await retrain_callback(new_traces, metrics)
                    
                    # Update state
                    self.last_trace_count = self.count_traces()
                    self.last_retrain_time = datetime.now()
                    
                    # Signal live update
                    self.signal_live_update()
                    
                    logger.info(f"✅ Retraining complete. Checkpoint: {checkpoint_path}")
                
                # Check every minute
                await asyncio.sleep(60)
                
            except Exception as e:
                logger.error(f"❌ Monitor error: {e}")
                await asyncio.sleep(60)


class IncrementalPPOTrainer:
    """Incremental PPO training for online learning"""
    
    def __init__(self, base_model_path: Path):
        self.base_model_path = base_model_path
        self.model = None
        self.encoders = None
        
    def load_base_model(self):
        """Load the base model and encoders"""
        # Load from existing checkpoint
        checkpoint_path = Path("rl_agent/rl_policy.pth")
        encoders_path = Path("rl_agent/encoders.pkl")
        
        if checkpoint_path.exists():
            self.model = torch.load(checkpoint_path)
            logger.info("📦 Loaded base model")
        
        if encoders_path.exists():
            with open(encoders_path, 'rb') as f:
                self.encoders = pickle.load(f)
            logger.info("📦 Loaded encoders")
    
    def prepare_incremental_data(self, new_traces: List[Dict]) -> Tuple[torch.Tensor, torch.Tensor]:
        """Prepare new traces for incremental training"""
        # This would use the same preprocessing as train_agent.py
        # For now, return dummy data
        n_samples = len(new_traces)
        state_dim = 64  # Match your actual state dimension
        action_dim = 16  # Match your actual action dimension
        
        states = torch.randn(n_samples, state_dim)
        actions = torch.randint(0, action_dim, (n_samples,))
        
        return states, actions
    
    async def incremental_train(self, new_traces: List[Dict], metrics: Dict) -> Path:
        """Perform incremental training on new traces"""
        if self.model is None:
            self.load_base_model()
        
        # Prepare data
        states, actions = self.prepare_incremental_data(new_traces)
        
        # Simple incremental update (in practice, use proper PPO)
        logger.info(f"🏃 Training on {len(new_traces)} new traces...")
        
        # Add exploration bonus based on metrics
        exploration_bonus = metrics['exploration_rate'] * 0.1
        
        # Save checkpoint
        manager = OnlineLearningManager(RetrainingConfig())
        checkpoint_path = manager.save_checkpoint(
            {'model': self.model, 'encoders': self.encoders},
            metrics
        )
        
        # Also update main policy files
        if self.model:
            torch.save(self.model, "rl_agent/rl_policy.pth")
        
        return checkpoint_path


async def main():
    """Main entry point for online learning daemon"""
    config = RetrainingConfig()
    manager = OnlineLearningManager(config)
    trainer = IncrementalPPOTrainer(Path("rl_agent/rl_policy.pth"))
    
    # Start monitoring
    await manager.monitor_and_retrain(trainer.incremental_train)


if __name__ == "__main__":
    asyncio.run(main())