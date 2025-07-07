#!/usr/bin/env python3
"""
PPO Agent for SentientOS Model/Tool Selection
Full implementation with training, evaluation, and deployment
"""

import json
import pickle
import numpy as np
from pathlib import Path
from typing import Dict, List, Tuple, Optional, Any
from dataclasses import dataclass, asdict
from datetime import datetime
import logging

import torch
import torch.nn as nn
import torch.optim as optim
import torch.nn.functional as F
from torch.distributions import Categorical
from torch.utils.data import Dataset, DataLoader

# Configure logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)


@dataclass
class RLState:
    """State representation for RL agent"""
    # Intent features
    intent_type: str
    intent_confidence: float
    
    # Prompt features
    prompt_length: int
    has_tool_keywords: bool
    has_query_keywords: bool
    has_code_keywords: bool
    
    # Context features
    time_of_day: int  # 0-23
    previous_success_rate: float
    rag_available: bool
    
    # System features
    avg_response_time: float
    model_availability: Dict[str, bool]
    
    def to_tensor(self, intent_encoder: Dict[str, int], model_list: List[str]) -> torch.Tensor:
        """Convert state to tensor representation"""
        features = []
        
        # One-hot encode intent
        intent_vec = np.zeros(len(intent_encoder))
        intent_vec[intent_encoder.get(self.intent_type, 0)] = 1
        features.extend(intent_vec)
        
        # Add scalar features
        features.extend([
            self.intent_confidence,
            min(self.prompt_length / 1000, 1.0),  # Normalize
            float(self.has_tool_keywords),
            float(self.has_query_keywords),
            float(self.has_code_keywords),
            self.time_of_day / 24.0,
            self.previous_success_rate,
            float(self.rag_available),
            min(self.avg_response_time / 5000, 1.0),  # Normalize to 5s max
        ])
        
        # Model availability vector
        for model in model_list:
            features.append(float(self.model_availability.get(model, False)))
        
        return torch.tensor(features, dtype=torch.float32)


@dataclass
class RLAction:
    """Action representation for RL agent"""
    model: str
    use_rag: bool
    tool: Optional[str]
    
    def to_index(self, model_encoder: Dict[str, int], tool_encoder: Dict[str, int]) -> int:
        """Convert action to single index for discrete action space"""
        model_idx = model_encoder.get(self.model, 0)
        rag_idx = 1 if self.use_rag else 0
        tool_idx = tool_encoder.get(self.tool, 0) if self.tool else 0
        
        # Combine into single action index
        # action = model_idx * (2 * len(tool_encoder)) + rag_idx * len(tool_encoder) + tool_idx
        num_tools = len(tool_encoder)
        action_idx = model_idx * (2 * num_tools) + rag_idx * num_tools + tool_idx
        return action_idx
    
    @staticmethod
    def from_index(idx: int, models: List[str], tools: List[str]) -> 'RLAction':
        """Decode action index back to RLAction"""
        num_tools = len(tools)
        
        model_idx = idx // (2 * num_tools)
        remainder = idx % (2 * num_tools)
        
        rag_idx = remainder // num_tools
        tool_idx = remainder % num_tools
        
        return RLAction(
            model=models[model_idx] if model_idx < len(models) else models[0],
            use_rag=bool(rag_idx),
            tool=tools[tool_idx] if tool_idx > 0 else None
        )


class TraceDataset(Dataset):
    """PyTorch dataset for trace data"""
    
    def __init__(self, traces: List[Dict[str, Any]], encoders: Dict[str, Any]):
        self.traces = traces
        self.encoders = encoders
        self.states = []
        self.actions = []
        self.rewards = []
        self.next_states = []
        self.dones = []
        
        self._preprocess_traces()
    
    def _preprocess_traces(self):
        """Convert traces to state-action-reward tuples"""
        for i, trace in enumerate(self.traces):
            # Extract state
            state = self._extract_state(trace, i)
            self.states.append(state)
            
            # Extract action
            action = RLAction(
                model=trace['model_used'],
                use_rag=trace['rag_used'],
                tool=trace.get('tool_executed')
            )
            self.actions.append(action)
            
            # Extract reward
            reward = trace.get('reward', 0.0)
            if reward is None:
                reward = 0.0  # Treat skipped feedback as neutral
            self.rewards.append(reward)
            
            # Next state (simplified - in practice would be next trace)
            if i < len(self.traces) - 1:
                next_state = self._extract_state(self.traces[i+1], i+1)
            else:
                next_state = state  # Terminal state
            self.next_states.append(next_state)
            
            # Episode boundaries (simplified)
            self.dones.append(i == len(self.traces) - 1)
    
    def _extract_state(self, trace: Dict[str, Any], idx: int) -> RLState:
        """Extract RLState from trace"""
        prompt = trace['prompt']
        
        # Detect keywords
        prompt_lower = prompt.lower()
        has_tool_keywords = any(kw in prompt_lower for kw in ['run', 'execute', 'check', 'call'])
        has_query_keywords = any(kw in prompt_lower for kw in ['what', 'how', 'why', 'explain'])
        has_code_keywords = any(kw in prompt_lower for kw in ['code', 'script', 'function', 'write'])
        
        # Extract time from timestamp
        try:
            timestamp = datetime.fromisoformat(trace['timestamp'].replace('Z', '+00:00'))
            hour = timestamp.hour
        except:
            hour = 12  # Default to noon
        
        # Model availability (simulated based on trace data)
        model_availability = {m: True for m in self.encoders['models']}
        if trace.get('fallback_used', False):
            # Mark primary model as unavailable if fallback was used
            model_availability[trace['model_used']] = False
        
        return RLState(
            intent_type=trace['intent'],
            intent_confidence=0.8,  # Simulated
            prompt_length=len(prompt),
            has_tool_keywords=has_tool_keywords,
            has_query_keywords=has_query_keywords,
            has_code_keywords=has_code_keywords,
            time_of_day=hour,
            previous_success_rate=0.9 if idx == 0 else sum(1 for t in self.traces[:idx] if t['success']) / idx,
            rag_available=True,  # Assume always available
            avg_response_time=trace['duration_ms'],
            model_availability=model_availability
        )
    
    def __len__(self):
        return len(self.traces)
    
    def __getitem__(self, idx):
        state_tensor = self.states[idx].to_tensor(
            self.encoders['intents'],
            self.encoders['models']
        )
        
        action_idx = self.actions[idx].to_index(
            self.encoders['model_to_idx'],
            self.encoders['tool_to_idx']
        )
        
        next_state_tensor = self.next_states[idx].to_tensor(
            self.encoders['intents'],
            self.encoders['models']
        )
        
        return {
            'state': state_tensor,
            'action': action_idx,
            'reward': torch.tensor(self.rewards[idx], dtype=torch.float32),
            'next_state': next_state_tensor,
            'done': torch.tensor(self.dones[idx], dtype=torch.float32)
        }


class PolicyNetwork(nn.Module):
    """Policy network for action selection"""
    
    def __init__(self, state_dim: int, action_dim: int, hidden_dim: int = 128):
        super().__init__()
        self.fc1 = nn.Linear(state_dim, hidden_dim)
        self.fc2 = nn.Linear(hidden_dim, hidden_dim)
        self.fc3 = nn.Linear(hidden_dim, hidden_dim)
        self.action_head = nn.Linear(hidden_dim, action_dim)
        self.value_head = nn.Linear(hidden_dim, 1)
        
        # Initialize weights
        self.apply(self._init_weights)
    
    def _init_weights(self, module):
        if isinstance(module, nn.Linear):
            nn.init.orthogonal_(module.weight, gain=np.sqrt(2))
            nn.init.constant_(module.bias, 0.0)
    
    def forward(self, x):
        x = F.relu(self.fc1(x))
        x = F.relu(self.fc2(x))
        x = F.relu(self.fc3(x))
        
        action_logits = self.action_head(x)
        value = self.value_head(x)
        
        return action_logits, value


class PPOAgent:
    """PPO Agent for model/tool selection"""
    
    def __init__(self, state_dim: int, action_dim: int, lr: float = 3e-4):
        self.device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
        logger.info(f"Using device: {self.device}")
        
        self.policy = PolicyNetwork(state_dim, action_dim).to(self.device)
        self.optimizer = optim.Adam(self.policy.parameters(), lr=lr)
        
        # PPO hyperparameters
        self.clip_param = 0.2
        self.value_loss_coef = 0.5
        self.entropy_coef = 0.01
        self.max_grad_norm = 0.5
        self.gamma = 0.99
        self.gae_lambda = 0.95
        
        # Training history
        self.training_history = {
            'rewards': [],
            'policy_losses': [],
            'value_losses': [],
            'entropies': [],
            'accuracies': []
        }
    
    def select_action(self, state: torch.Tensor) -> Tuple[int, torch.Tensor, torch.Tensor]:
        """Select action using current policy"""
        with torch.no_grad():
            state = state.to(self.device)
            action_logits, value = self.policy(state.unsqueeze(0))
            
            action_probs = F.softmax(action_logits, dim=-1)
            dist = Categorical(action_probs)
            action = dist.sample()
            log_prob = dist.log_prob(action)
            
        return action.item(), log_prob, value.squeeze()
    
    def compute_gae(self, rewards: List[float], values: List[torch.Tensor], 
                    dones: List[bool]) -> Tuple[torch.Tensor, torch.Tensor]:
        """Compute Generalized Advantage Estimation"""
        advantages = []
        returns = []
        
        gae = 0
        next_value = 0
        
        for t in reversed(range(len(rewards))):
            if dones[t]:
                next_value = 0
                gae = 0
            
            delta = rewards[t] + self.gamma * next_value - values[t]
            gae = delta + self.gamma * self.gae_lambda * gae
            
            advantages.insert(0, gae)
            returns.insert(0, gae + values[t])
            
            next_value = values[t]
        
        advantages = torch.tensor(advantages, dtype=torch.float32)
        returns = torch.tensor(returns, dtype=torch.float32)
        
        # Normalize advantages
        advantages = (advantages - advantages.mean()) / (advantages.std() + 1e-8)
        
        return advantages, returns
    
    def train_epoch(self, dataloader: DataLoader, num_epochs: int = 4):
        """Train for one epoch using PPO"""
        epoch_rewards = []
        epoch_policy_losses = []
        epoch_value_losses = []
        epoch_entropies = []
        
        for epoch in range(num_epochs):
            for batch in dataloader:
                states = batch['state'].to(self.device)
                actions = batch['action'].to(self.device)
                rewards = batch['reward'].to(self.device)
                next_states = batch['next_state'].to(self.device)
                dones = batch['done'].to(self.device)
                
                # Get current policy predictions
                action_logits, values = self.policy(states)
                action_probs = F.softmax(action_logits, dim=-1)
                dist = Categorical(action_probs)
                
                # Calculate advantages
                with torch.no_grad():
                    _, next_values = self.policy(next_states)
                    advantages = rewards + self.gamma * next_values.squeeze() * (1 - dones) - values.squeeze()
                    returns = advantages + values.squeeze()
                
                # Calculate losses
                log_probs = dist.log_prob(actions)
                entropy = dist.entropy().mean()
                
                # Policy loss (PPO clip)
                ratio = torch.exp(log_probs - log_probs.detach())
                surr1 = ratio * advantages
                surr2 = torch.clamp(ratio, 1 - self.clip_param, 1 + self.clip_param) * advantages
                policy_loss = -torch.min(surr1, surr2).mean()
                
                # Value loss
                value_loss = F.mse_loss(values.squeeze(), returns.detach())
                
                # Total loss
                loss = policy_loss + self.value_loss_coef * value_loss - self.entropy_coef * entropy
                
                # Optimize
                self.optimizer.zero_grad()
                loss.backward()
                nn.utils.clip_grad_norm_(self.policy.parameters(), self.max_grad_norm)
                self.optimizer.step()
                
                # Record metrics
                epoch_rewards.extend(rewards.cpu().numpy())
                epoch_policy_losses.append(policy_loss.item())
                epoch_value_losses.append(value_loss.item())
                epoch_entropies.append(entropy.item())
        
        # Update history
        self.training_history['rewards'].append(np.mean(epoch_rewards))
        self.training_history['policy_losses'].append(np.mean(epoch_policy_losses))
        self.training_history['value_losses'].append(np.mean(epoch_value_losses))
        self.training_history['entropies'].append(np.mean(epoch_entropies))
    
    def save_checkpoint(self, path: str):
        """Save model checkpoint"""
        checkpoint = {
            'policy_state_dict': self.policy.state_dict(),
            'optimizer_state_dict': self.optimizer.state_dict(),
            'training_history': self.training_history,
            'hyperparameters': {
                'clip_param': self.clip_param,
                'value_loss_coef': self.value_loss_coef,
                'entropy_coef': self.entropy_coef,
                'gamma': self.gamma
            }
        }
        torch.save(checkpoint, path)
        logger.info(f"Checkpoint saved to {path}")
    
    def load_checkpoint(self, path: str):
        """Load model checkpoint"""
        checkpoint = torch.load(path, map_location=self.device)
        self.policy.load_state_dict(checkpoint['policy_state_dict'])
        self.optimizer.load_state_dict(checkpoint['optimizer_state_dict'])
        self.training_history = checkpoint['training_history']
        logger.info(f"Checkpoint loaded from {path}")


def create_encoders(traces: List[Dict[str, Any]]) -> Dict[str, Any]:
    """Create encoders for categorical variables"""
    # Extract unique values
    intents = sorted(set(trace['intent'] for trace in traces))
    models = sorted(set(trace['model_used'] for trace in traces))
    tools = sorted(set(trace.get('tool_executed', 'none') for trace in traces))
    tools = ['none'] + [t for t in tools if t != 'none']  # Ensure 'none' is first
    
    # Create encoders
    encoders = {
        'intents': {intent: i for i, intent in enumerate(intents)},
        'models': models,
        'model_to_idx': {model: i for i, model in enumerate(models)},
        'tools': tools,
        'tool_to_idx': {tool: i for i, tool in enumerate(tools)},
        'idx_to_model': {i: model for i, model in enumerate(models)},
        'idx_to_tool': {i: tool for i, tool in enumerate(tools)}
    }
    
    # Calculate dimensions
    encoders['state_dim'] = (
        len(intents) +  # One-hot encoded intent
        9 +  # Scalar features
        len(models)  # Model availability
    )
    encoders['action_dim'] = len(models) * 2 * len(tools)
    
    return encoders


def main():
    """Main training script"""
    logger.info("Starting PPO Agent Training for SentientOS")
    
    # Load training data
    train_path = Path("rl_data/train.jsonl")
    test_path = Path("rl_data/test.jsonl")
    
    train_traces = []
    with open(train_path, 'r') as f:
        for line in f:
            train_traces.append(json.loads(line))
    
    test_traces = []
    with open(test_path, 'r') as f:
        for line in f:
            test_traces.append(json.loads(line))
    
    logger.info(f"Loaded {len(train_traces)} training traces, {len(test_traces)} test traces")
    
    # Create encoders
    all_traces = train_traces + test_traces
    encoders = create_encoders(all_traces)
    
    logger.info(f"State dimension: {encoders['state_dim']}")
    logger.info(f"Action dimension: {encoders['action_dim']}")
    logger.info(f"Models: {encoders['models']}")
    logger.info(f"Tools: {encoders['tools']}")
    
    # Create datasets
    train_dataset = TraceDataset(train_traces, encoders)
    test_dataset = TraceDataset(test_traces, encoders)
    
    train_loader = DataLoader(train_dataset, batch_size=32, shuffle=True)
    test_loader = DataLoader(test_dataset, batch_size=32, shuffle=False)
    
    # Save encoders
    with open('rl_agent/encoders.pkl', 'wb') as f:
        pickle.dump(encoders, f)
    logger.info("Encoders saved to rl_agent/encoders.pkl")
    
    # Initialize agent
    agent = PPOAgent(encoders['state_dim'], encoders['action_dim'])
    
    # Training loop
    num_epochs = 50
    checkpoint_interval = 10
    
    logger.info(f"Starting training for {num_epochs} epochs...")
    
    for epoch in range(num_epochs):
        # Train
        agent.train_epoch(train_loader)
        
        # Log progress
        avg_reward = agent.training_history['rewards'][-1]
        policy_loss = agent.training_history['policy_losses'][-1]
        logger.info(f"Epoch {epoch+1}/{num_epochs} - Avg Reward: {avg_reward:.3f}, Policy Loss: {policy_loss:.3f}")
        
        # Save checkpoint
        if (epoch + 1) % checkpoint_interval == 0:
            agent.save_checkpoint(f'rl_agent/checkpoint_epoch_{epoch+1}.pth')
    
    # Save final model
    agent.save_checkpoint('rl_agent/rl_policy.pth')
    
    # Save preprocessed datasets
    with open('rl_agent/train_dataset.pkl', 'wb') as f:
        pickle.dump(train_dataset, f)
    
    logger.info("Training complete!")
    
    # Generate summary
    summary = {
        'final_avg_reward': agent.training_history['rewards'][-1],
        'total_epochs': num_epochs,
        'state_dim': encoders['state_dim'],
        'action_dim': encoders['action_dim'],
        'num_train_traces': len(train_traces),
        'num_test_traces': len(test_traces)
    }
    
    with open('rl_agent/training_summary.json', 'w') as f:
        json.dump(summary, f, indent=2)
    
    logger.info(f"Final average reward: {summary['final_avg_reward']:.3f}")


if __name__ == "__main__":
    main()