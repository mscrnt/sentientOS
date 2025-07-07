#!/usr/bin/env python3
"""
Deploy trained RL policy for real-time decision making
Shows how to integrate with the routing system
"""

import json
import pickle
from pathlib import Path
from typing import Dict, Any, Optional, Tuple
from datetime import datetime

import torch
import torch.nn.functional as F

from ppo_agent import PPOAgent, RLState, RLAction, PolicyNetwork


class RLPolicyRouter:
    """Production-ready RL policy router"""
    
    def __init__(self, policy_path: str = "rl_agent/rl_policy.pth",
                 encoders_path: str = "rl_agent/encoders.pkl",
                 confidence_threshold: float = 0.7):
        """
        Initialize RL policy router
        
        Args:
            policy_path: Path to trained policy checkpoint
            encoders_path: Path to feature encoders
            confidence_threshold: Minimum confidence to use RL policy
        """
        self.device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
        self.confidence_threshold = confidence_threshold
        
        # Load encoders
        with open(encoders_path, 'rb') as f:
            self.encoders = pickle.load(f)
        
        # Initialize and load policy
        self.policy = PolicyNetwork(
            state_dim=self.encoders['state_dim'],
            action_dim=self.encoders['action_dim']
        ).to(self.device)
        
        # Load checkpoint
        checkpoint = torch.load(policy_path, map_location=self.device)
        self.policy.load_state_dict(checkpoint['policy_state_dict'])
        self.policy.eval()  # Set to evaluation mode
        
        print(f"✅ RL Policy loaded from {policy_path}")
        print(f"   Device: {self.device}")
        print(f"   Models: {self.encoders['models']}")
        print(f"   Tools: {self.encoders['tools']}")
    
    def _load_policy_and_encoders(self):
        """Load or reload policy and encoders"""
        try:
            # Load encoders
            with open(self.encoders_path, 'rb') as f:
                self.encoders = pickle.load(f)
            
            # Initialize and load policy
            self.policy = PolicyNetwork(
                state_dim=self.encoders['state_dim'],
                action_dim=self.encoders['action_dim']
            ).to(self.device)
            
            # Load checkpoint
            if self.policy_path.exists():
                checkpoint = torch.load(self.policy_path, map_location=self.device)
                self.policy.load_state_dict(checkpoint['policy_state_dict'])
                self.policy.eval()  # Set to evaluation mode
                self.last_modified_time = self.policy_path.stat().st_mtime
                logger.info(f"✅ Loaded policy from {self.policy_path}")
            else:
                logger.warning(f"⚠️ Policy file not found: {self.policy_path}")
                
        except Exception as e:
            logger.error(f"❌ Error loading policy: {e}")
            raise
    
    def _monitor_reload(self):
        """Monitor for reload signals and policy updates"""
        logger.info("👁️ Monitoring for policy updates...")
        
        while True:
            try:
                # Check for explicit reload signal
                if self.reload_signal_path.exists():
                    logger.info("📡 Reload signal detected")
                    with self.reload_lock:
                        self._load_policy_and_encoders()
                    self.reload_signal_path.unlink()  # Remove signal file
                    
                # Check for file modification
                elif self.policy_path.exists():
                    current_mtime = self.policy_path.stat().st_mtime
                    if self.last_modified_time and current_mtime > self.last_modified_time:
                        logger.info("📝 Policy file updated, reloading...")
                        with self.reload_lock:
                            self._load_policy_and_encoders()
                
                time.sleep(5)  # Check every 5 seconds
                
            except Exception as e:
                logger.error(f"❌ Reload monitor error: {e}")
                time.sleep(10)
    
    def route(self, prompt: str, context: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
        """
        Route a query using the RL policy
        
        Args:
            prompt: User query
            context: Optional context (previous traces, system state, etc.)
            
        Returns:
            Routing decision with model, RAG usage, and tool selection
        """
        # Extract state features
        state = self._extract_state(prompt, context)
        
        # Convert to tensor
        state_tensor = state.to_tensor(
            self.encoders['intents'],
            self.encoders['models']
        ).to(self.device)
        
        # Get policy prediction (thread-safe)
        with self.reload_lock:
            with torch.no_grad():
                action_logits, value = self.policy(state_tensor.unsqueeze(0))
                action_probs = F.softmax(action_logits, dim=-1)
                
                # Get best action and confidence
                best_action_idx = torch.argmax(action_probs).item()
                confidence = action_probs[0, best_action_idx].item()
        
        # Decode action
        action = RLAction.from_index(
            best_action_idx,
            self.encoders['models'],
            self.encoders['tools']
        )
        
        # Prepare response
        decision = {
            'model': action.model,
            'use_rag': action.use_rag,
            'tool': action.tool,
            'confidence': confidence,
            'value_estimate': value.item(),
            'fallback_used': False
        }
        
        # Check confidence threshold
        if confidence < self.confidence_threshold:
            # Use fallback heuristic
            decision['fallback_used'] = True
            decision.update(self._fallback_policy(prompt, state))
        
        # Add metadata
        decision['metadata'] = {
            'timestamp': datetime.now().isoformat(),
            'intent': state.intent_type,
            'prompt_length': len(prompt),
            'policy_version': 'v1.0'
        }
        
        return decision
    
    def _extract_state(self, prompt: str, context: Optional[Dict[str, Any]]) -> RLState:
        """Extract state features from prompt and context"""
        prompt_lower = prompt.lower()
        
        # Detect intent (simplified - in production use full intent detector)
        if any(kw in prompt_lower for kw in ['call', 'execute', 'run', '!@']):
            intent = 'ToolCall'
            confidence = 0.9
        elif any(kw in prompt_lower for kw in ['write', 'generate', 'create', 'code']):
            intent = 'CodeGeneration'
            confidence = 0.85
        elif any(kw in prompt_lower for kw in ['analyze', 'debug', 'why']):
            intent = 'Analysis'
            confidence = 0.8
        else:
            intent = 'GeneralKnowledge'
            confidence = 0.75
        
        # Extract features
        has_tool_keywords = any(kw in prompt_lower for kw in ['run', 'execute', 'check', 'call'])
        has_query_keywords = any(kw in prompt_lower for kw in ['what', 'how', 'why', 'explain'])
        has_code_keywords = any(kw in prompt_lower for kw in ['code', 'script', 'function', 'write'])
        
        # Context features
        if context:
            prev_success_rate = context.get('success_rate', 0.9)
            avg_response_time = context.get('avg_response_time', 1000)
            model_availability = context.get('model_availability', {})
        else:
            prev_success_rate = 0.9
            avg_response_time = 1000
            model_availability = {m: True for m in self.encoders['models']}
        
        return RLState(
            intent_type=intent,
            intent_confidence=confidence,
            prompt_length=len(prompt),
            has_tool_keywords=has_tool_keywords,
            has_query_keywords=has_query_keywords,
            has_code_keywords=has_code_keywords,
            time_of_day=datetime.now().hour,
            previous_success_rate=prev_success_rate,
            rag_available=True,
            avg_response_time=avg_response_time,
            model_availability=model_availability
        )
    
    def _fallback_policy(self, prompt: str, state: RLState) -> Dict[str, Any]:
        """Fallback heuristic policy when confidence is low"""
        # Simple rules based on intent
        if state.intent_type == 'ToolCall':
            return {
                'model': 'deepseek-v2:16b',
                'use_rag': False,
                'tool': 'disk_info'  # Default tool
            }
        elif state.intent_type == 'CodeGeneration':
            return {
                'model': 'deepseek-v2:16b',
                'use_rag': False,
                'tool': None
            }
        else:
            return {
                'model': 'deepseek-v2:16b',
                'use_rag': True,
                'tool': None
            }
    
    def update_context(self, trace: Dict[str, Any], context: Dict[str, Any]) -> Dict[str, Any]:
        """Update context based on execution trace"""
        # Update success rate
        if 'traces' not in context:
            context['traces'] = []
        context['traces'].append(trace)
        
        # Calculate rolling success rate
        recent_traces = context['traces'][-100:]  # Last 100 traces
        success_count = sum(1 for t in recent_traces if t.get('success', False))
        context['success_rate'] = success_count / len(recent_traces) if recent_traces else 0.9
        
        # Calculate average response time
        response_times = [t.get('duration_ms', 1000) for t in recent_traces if 'duration_ms' in t]
        context['avg_response_time'] = sum(response_times) / len(response_times) if response_times else 1000
        
        # Update model availability based on failures
        model_failures = {}
        for t in recent_traces:
            if not t.get('success', True) and 'model_used' in t:
                model = t['model_used']
                model_failures[model] = model_failures.get(model, 0) + 1
        
        context['model_availability'] = {}
        for model in self.encoders['models']:
            # Mark as unavailable if >20% failure rate in recent traces
            failure_rate = model_failures.get(model, 0) / max(1, len(recent_traces))
            context['model_availability'][model] = failure_rate < 0.2
        
        return context


def demo_deployment():
    """Demonstrate policy deployment"""
    print("🚀 RL Policy Deployment Demo")
    print("="*50)
    
    # Initialize router
    router = RLPolicyRouter()
    
    # Test queries
    test_queries = [
        "What is virtual memory?",
        "Check disk space usage",
        "Write a Python script to monitor CPU",
        "My system is running slowly, analyze the issue",
        "!@ call memory_usage",
        "How does garbage collection work?",
    ]
    
    # Context tracking
    context = {
        'success_rate': 0.9,
        'avg_response_time': 1000,
        'model_availability': {m: True for m in router.encoders['models']}
    }
    
    print("\n📋 Testing RL Policy Router:\n")
    
    for i, query in enumerate(test_queries, 1):
        print(f"Query {i}: \"{query}\"")
        
        # Get routing decision
        decision = router.route(query, context)
        
        print(f"  → Model: {decision['model']}")
        print(f"  → Use RAG: {decision['use_rag']}")
        print(f"  → Tool: {decision['tool'] or 'None'}")
        print(f"  → Confidence: {decision['confidence']:.2f}")
        print(f"  → Value Est: {decision['value_estimate']:.2f}")
        
        if decision['fallback_used']:
            print(f"  ⚠️  Fallback used (low confidence)")
        
        # Simulate execution and update context
        trace = {
            'prompt': query,
            'model_used': decision['model'],
            'tool_executed': decision['tool'],
            'rag_used': decision['use_rag'],
            'success': True,
            'duration_ms': 500 + i * 100  # Simulated
        }
        
        context = router.update_context(trace, context)
        print()
    
    print("\n📊 Context after executions:")
    print(f"  Success Rate: {context['success_rate']:.2%}")
    print(f"  Avg Response Time: {context['avg_response_time']:.0f}ms")
    print(f"  Model Availability: {dict(context['model_availability'])}")
    
    print("\n✅ RL Policy Router is ready for production deployment!")


def export_for_rust():
    """Export policy for Rust integration"""
    print("\n📦 Exporting for Rust Integration...")
    
    # Load policy and encoders
    with open('rl_agent/encoders.pkl', 'rb') as f:
        encoders = pickle.load(f)
    
    checkpoint = torch.load('rl_agent/rl_policy.pth', map_location='cpu')
    
    # Export configuration
    config = {
        'models': encoders['models'],
        'tools': encoders['tools'],
        'intents': list(encoders['intents'].keys()),
        'state_dim': encoders['state_dim'],
        'action_dim': encoders['action_dim'],
        'model_to_idx': encoders['model_to_idx'],
        'tool_to_idx': encoders['tool_to_idx']
    }
    
    with open('rl_agent/rl_config.json', 'w') as f:
        json.dump(config, f, indent=2)
    
    print("✅ Configuration exported to rl_agent/rl_config.json")
    print("\nTo integrate with Rust:")
    print("1. Use tch crate for PyTorch model loading")
    print("2. Load rl_policy.pth weights")
    print("3. Implement state extraction in Rust")
    print("4. Use policy for real-time routing")


if __name__ == "__main__":
    # Check if policy exists
    if not Path("rl_agent/rl_policy.pth").exists():
        print("❌ No trained policy found!")
        print("Run: python rl_agent/train_agent.py")
    else:
        demo_deployment()
        export_for_rust()