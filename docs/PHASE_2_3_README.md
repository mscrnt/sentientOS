# SentientOS Phase 2-3: Intelligent Action + Reinforcement Learning

## Overview

Phase 2-3 extends SentientOS with intelligent RAG-Tool fusion and reinforcement learning capabilities, enabling the system to:

1. **Understand and Act** - Seamlessly combine knowledge retrieval with tool execution
2. **Learn from Experience** - Use reinforcement learning to optimize decisions
3. **Adapt Over Time** - Improve model/tool selection based on user feedback

## Architecture

### Phase 2: RAG-Tool Fusion

```
User Query → Intent Detection → Hybrid Pipeline → Result
                ↓                      ↓
            [Pure Query]          [RAG + Tool]
            [Pure Action]         [Conditions]
            [Conditional]         [Execution]
```

#### Key Components

1. **RagToolRouter** (`rag_tool_fusion/rag_tool_router.rs`)
   - Detects hybrid intent from user prompts
   - Orchestrates RAG queries and tool execution
   - Manages execution pipeline flow

2. **ConditionMatcher** (`rag_tool_fusion/condition_matcher.rs`)
   - Evaluates conditions based on RAG responses
   - Triggers appropriate tools based on patterns
   - Supports regex, keyword, numeric, and combined conditions

3. **TraceLogger** (`rag_tool_fusion/trace_logger.rs`)
   - Logs all execution details for RL training
   - Tracks success metrics and user feedback
   - Provides analytics and summaries

### Phase 3: Reinforcement Learning

```
Traces → State Extraction → RL Agent → Policy Update
           ↓                    ↓            ↓
      [Features]           [Action]    [Improved Selection]
```

#### Components

1. **Trace Collection** - Every execution logged with outcomes
2. **Reward System** - User feedback + automatic evaluation
3. **RL Agent** - PPO-based model/tool selection optimizer
4. **Analytics** - Performance insights and trends

## Usage

### Basic Commands

```bash
# Hybrid RAG + Tool execution
rag_tool "Check disk space and explain the output" --explain

# View RL traces
rl trace summary
rl trace list -n 10
rl trace best
rl trace worst

# Export traces for analysis
rl export -f json -o traces_export.json
```

### Configuration

#### Conditions (`config/conditions.yaml`)
Define when tools should be triggered based on RAG results:

```yaml
conditions:
  - name: high_memory_usage
    pattern:
      type: Numeric
      field: memory_percent
      operator: ">"
      value: 90.0
    tool: clean_cache
    confirm: true
    priority: 10
```

#### Rewards (`config/rewards.yaml`)
Define reward policies for RL training:

```yaml
reward_policies:
  base_rewards:
    tool_success: 0.5
    rag_match: 0.3
    user_approval: 1.0
```

## Intent Types

1. **PureQuery** - Information retrieval only (RAG)
2. **PureAction** - Direct tool execution
3. **QueryThenAction** - RAG first, then tool based on result
4. **ActionThenQuery** - Tool first, then RAG explanation
5. **ConditionalAction** - Tool execution depends on conditions

## Feedback System

After each execution, users can provide feedback:
- **Y/Yes** - Positive feedback (+1.0 reward)
- **N/No** - Negative feedback (-1.0 reward)
- **S/Skip** - No feedback (no reward update)

This feedback trains the RL agent to make better decisions.

## RL Agent Training

The Python RL agent (`rl_agent/agent_skeleton.py`) uses:
- **PPO** (Proximal Policy Optimization) for stable learning
- **State Features**: Intent, prompt characteristics, time, history
- **Actions**: Model selection, RAG usage, tool selection

### Training Process

1. Collect traces with `rag_tool` commands
2. Provide feedback for quality training data
3. Run RL agent training:
   ```bash
   python rl_agent/agent_skeleton.py
   ```
4. Deploy improved policy back to router

## Example Workflows

### Scenario 1: System Monitoring
```
User: "My system feels slow, what should I check?"
System: 
1. Detects QueryThenAction intent
2. RAG retrieves performance troubleshooting guide
3. Identifies high CPU condition
4. Executes process_list tool
5. Combines knowledge + live data in response
```

### Scenario 2: Conditional Maintenance
```
User: "Check if I need to clean my cache"
System:
1. Detects ConditionalAction intent
2. RAG explains cache purpose
3. Executes memory_usage tool
4. If >90% usage, recommends cache cleanup
5. Asks for confirmation before executing
```

## Performance Metrics

Track system performance with:
```bash
rl trace summary
```

Metrics include:
- Success rate by intent type
- Average execution time
- Model/tool usage distribution
- User satisfaction (average reward)

## Future Enhancements

1. **Multi-step Planning** - Chain multiple tools intelligently
2. **Context Awareness** - Remember conversation history
3. **Proactive Suggestions** - Recommend actions before issues
4. **Transfer Learning** - Share learning across deployments
5. **A/B Testing** - Compare strategies systematically

## Development

### Adding New Conditions

1. Edit `config/conditions.yaml`
2. Define pattern matching rules
3. Specify tool and arguments
4. Set priority and confirmation

### Extending the RL Agent

1. Add new state features in `agent_skeleton.py`
2. Expand action space for new models/tools
3. Customize reward function
4. Retrain on collected traces

## Troubleshooting

### No Tool Execution
- Check conditions in `config/conditions.yaml`
- Verify tool registration in registry
- Use `--explain` flag to see pipeline

### Low RL Performance
- Collect more diverse traces
- Ensure balanced positive/negative feedback
- Check reward policy configuration
- Increase training epochs

### Trace Analysis
```bash
# Find specific trace
grep "keyword" logs/rl_trace.jsonl

# View failed executions
rl trace worst

# Export for external analysis
rl export -f csv -o analysis.csv
```