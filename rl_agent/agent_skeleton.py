#!/usr/bin/env python3
"""
SentientOS RL Agent Skeleton
Phase 3: Reinforcement Learning for Model/Tool Selection

This is a lightweight PPO (Proximal Policy Optimization) agent that learns
from execution traces to optimize model and tool selection strategies.
"""

import json
import numpy as np
from pathlib import Path
from typing import Dict, List, Tuple, Optional
from dataclasses import dataclass
from datetime import datetime
import torch
import torch.nn as nn
import torch.optim as optim
from torch.distributions import Categorical


@dataclass
class State:
    """Environment state for the RL agent"""
    intent: str
    prompt_length: int
    has_rag_keywords: bool
    has_tool_keywords: bool
    time_of_day: int  # Hour (0-23)
    previous_success_rate: float
    

@dataclass
class Action:
    """Agent action (model/tool selection)"""
    model: str
    use_rag: bool
    tool: Optional[str]
    

class PolicyNetwork(nn.Module):
    """Neural network for policy (action selection)"""
    def __init__(self, state_dim: int, action_dim: int, hidden_dim: int = 64):
        super().__init__()
        self.fc1 = nn.Linear(state_dim, hidden_dim)
        self.fc2 = nn.Linear(hidden_dim, hidden_dim)
        self.fc3 = nn.Linear(hidden_dim, action_dim)
        
    def forward(self, x):
        x = torch.relu(self.fc1(x))
        x = torch.relu(self.fc2(x))
        return torch.softmax(self.fc3(x), dim=-1)


class ValueNetwork(nn.Module):
    """Neural network for value function (state evaluation)"""
    def __init__(self, state_dim: int, hidden_dim: int = 64):
        super().__init__()
        self.fc1 = nn.Linear(state_dim, hidden_dim)
        self.fc2 = nn.Linear(hidden_dim, hidden_dim)
        self.fc3 = nn.Linear(hidden_dim, 1)
        
    def forward(self, x):
        x = torch.relu(self.fc1(x))
        x = torch.relu(self.fc2(x))
        return self.fc3(x)


class SentientRLAgent:
    """PPO-based RL agent for SentientOS"""
    
    def __init__(self, config_path: str = "config/rl_agent.yaml"):
        self.config = self._load_config(config_path)
        self.state_dim = 6  # Number of state features
        self.action_dim = 10  # Number of possible actions
        
        # Initialize networks
        self.policy_net = PolicyNetwork(self.state_dim, self.action_dim)
        self.value_net = ValueNetwork(self.state_dim)
        
        # Optimizers
        self.policy_optimizer = optim.Adam(self.policy_net.parameters(), lr=3e-4)
        self.value_optimizer = optim.Adam(self.value_net.parameters(), lr=3e-4)
        
        # Action mappings
        self.models = ["phi2_local", "qwen2.5", "llama3.2", "gpt-4o-mini"]
        self.tools = ["disk_info", "memory_usage", "process_list", "network_status", None]
        
    def _load_config(self, path: str) -> Dict:
        """Load agent configuration"""
        # In production, load from YAML
        return {
            "epsilon": 0.2,  # PPO clipping parameter
            "gamma": 0.99,   # Discount factor
            "batch_size": 32,
            "epochs": 10,
        }
    
    def state_to_tensor(self, state: State) -> torch.Tensor:
        """Convert state to tensor for neural network"""
        features = [
            self._encode_intent(state.intent),
            min(state.prompt_length / 1000, 1.0),  # Normalize
            float(state.has_rag_keywords),
            float(state.has_tool_keywords),
            state.time_of_day / 24.0,  # Normalize
            state.previous_success_rate,
        ]
        return torch.tensor(features, dtype=torch.float32)
    
    def _encode_intent(self, intent: str) -> float:
        """Encode intent as numeric value"""
        intent_map = {
            "PureQuery": 0.0,
            "PureAction": 0.25,
            "QueryThenAction": 0.5,
            "ActionThenQuery": 0.75,
            "ConditionalAction": 1.0,
        }
        return intent_map.get(intent, 0.5)
    
    def select_action(self, state: State) -> Tuple[Action, float]:
        """Select action using current policy"""
        state_tensor = self.state_to_tensor(state)
        
        # Get action probabilities
        with torch.no_grad():
            action_probs = self.policy_net(state_tensor)
            value = self.value_net(state_tensor)
        
        # Sample action
        dist = Categorical(action_probs)
        action_idx = dist.sample()
        log_prob = dist.log_prob(action_idx)
        
        # Decode action
        action = self._decode_action(action_idx.item())
        
        return action, log_prob.item()
    
    def _decode_action(self, action_idx: int) -> Action:
        """Decode action index to Action object"""
        # Simple mapping - in production, use more sophisticated encoding
        model_idx = action_idx % len(self.models)
        use_rag = (action_idx // len(self.models)) % 2 == 1
        tool_idx = action_idx // (len(self.models) * 2)
        
        return Action(
            model=self.models[model_idx],
            use_rag=use_rag,
            tool=self.tools[tool_idx] if tool_idx < len(self.tools) else None
        )
    
    def load_traces(self, trace_path: str = "logs/rl_trace.jsonl") -> List[Dict]:
        """Load execution traces for training"""
        traces = []
        with open(trace_path, 'r') as f:
            for line in f:
                if line.strip():
                    traces.append(json.loads(line))
        return traces
    
    def train_on_traces(self, traces: List[Dict]):
        """Train the agent on collected traces"""
        # Convert traces to training data
        states = []
        actions = []
        rewards = []
        
        for trace in traces:
            if trace.get('reward') is not None:
                state = self._extract_state(trace)
                action = self._extract_action(trace)
                reward = trace['reward']
                
                states.append(state)
                actions.append(action)
                rewards.append(reward)
        
        # Implement PPO training loop
        print(f"Training on {len(states)} experiences...")
        # ... PPO implementation ...
        
    def _extract_state(self, trace: Dict) -> State:
        """Extract state from trace entry"""
        return State(
            intent=trace['intent'],
            prompt_length=len(trace['prompt']),
            has_rag_keywords=any(kw in trace['prompt'].lower() 
                                for kw in ['what', 'how', 'why', 'explain']),
            has_tool_keywords=any(kw in trace['prompt'].lower() 
                                 for kw in ['check', 'run', 'execute', 'show']),
            time_of_day=datetime.fromisoformat(trace['timestamp']).hour,
            previous_success_rate=0.8,  # Would track this over time
        )
    
    def _extract_action(self, trace: Dict) -> int:
        """Extract action index from trace"""
        # Reverse of _decode_action
        model_idx = self.models.index(trace['model_used'])
        use_rag = int(trace['rag_used'])
        tool_idx = self.tools.index(trace.get('tool_executed'))
        
        return model_idx + use_rag * len(self.models) + tool_idx * len(self.models) * 2
    
    def save_checkpoint(self, path: str = "models/rl_agent.pth"):
        """Save model checkpoint"""
        torch.save({
            'policy_state_dict': self.policy_net.state_dict(),
            'value_state_dict': self.value_net.state_dict(),
            'policy_optimizer_state_dict': self.policy_optimizer.state_dict(),
            'value_optimizer_state_dict': self.value_optimizer.state_dict(),
        }, path)
        
    def load_checkpoint(self, path: str = "models/rl_agent.pth"):
        """Load model checkpoint"""
        checkpoint = torch.load(path)
        self.policy_net.load_state_dict(checkpoint['policy_state_dict'])
        self.value_net.load_state_dict(checkpoint['value_state_dict'])
        self.policy_optimizer.load_state_dict(checkpoint['policy_optimizer_state_dict'])
        self.value_optimizer.load_state_dict(checkpoint['value_optimizer_state_dict'])


def main():
    """Example usage"""
    agent = SentientRLAgent()
    
    # Load traces
    traces = agent.load_traces()
    print(f"Loaded {len(traces)} execution traces")
    
    # Filter traces with rewards
    rewarded_traces = [t for t in traces if t.get('reward') is not None]
    print(f"Found {len(rewarded_traces)} traces with feedback")
    
    if rewarded_traces:
        # Train agent
        agent.train_on_traces(rewarded_traces)
        
        # Save checkpoint
        agent.save_checkpoint()
        print("Agent checkpoint saved")
    
    # Example: Select action for new query
    example_state = State(
        intent="QueryThenAction",
        prompt_length=50,
        has_rag_keywords=True,
        has_tool_keywords=True,
        time_of_day=14,
        previous_success_rate=0.85
    )
    
    action, log_prob = agent.select_action(example_state)
    print(f"\nRecommended action for example state:")
    print(f"  Model: {action.model}")
    print(f"  Use RAG: {action.use_rag}")
    print(f"  Tool: {action.tool}")


if __name__ == "__main__":
    main()