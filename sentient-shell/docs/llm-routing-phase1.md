# Phase 1: LLM Router + Embedded Inference Engine

## Overview

Phase 1 implements intelligent LLM routing that automatically selects the best model based on:
- Intent detection from user prompts
- Model capabilities and performance tiers
- Context length requirements
- Online/offline availability

## Architecture

### Components

1. **Intent Detector** (`ai_router/intent.rs`)
   - Analyzes prompts to detect user intent
   - Maps intents to capabilities and performance requirements
   - Estimates context length needs

2. **Model Configuration** (`config/models.toml`)
   - Defines available models and their characteristics
   - Specifies routing rules and preferences
   - Configures load balancing strategies

3. **Enhanced Router** (`ai_router/enhanced_router.rs`)
   - Implements intent-based routing logic
   - Manages fallback chains for offline operation
   - Provides model selection transparency

4. **CLI Interface** (`ai_router/llm_cli.rs`)
   - Commands for routing inspection and testing
   - Model information and capability queries
   - Routing decision visualization

## Model Configuration

The system supports multiple model types:

### Local Models
- **phi2_local** - Boot-level model for offline operation
  - Fast tool calling and command interpretation
  - Always available, high priority
  - Limited context (2K tokens)

### Remote Models
- **llama3:8b** - Balanced general-purpose model
  - Good for general reasoning and conversation
  - Medium context (8K tokens)
  
- **deepseek-v2** - Powerful model for complex tasks
  - Advanced reasoning and code generation
  - Long context (32K tokens)
  - Lower priority, used for demanding tasks

- **mistral:7b-instruct** - Fast instruction following
  - Tool orchestration and quick responses
  - Medium context (8K tokens)

- **llama3.2-vision** - Specialized for visual tasks
  - Screenshot analysis and UI debugging
  - Limited context (4K tokens)

## Intent Detection

The system recognizes multiple intent types:

1. **Tool Calling** - Commands with !@, !$, etc. or "call tool"
2. **Code Generation** - "Write function", "implement", "create code"
3. **System Analysis** - "Analyze", "diagnose", "check performance"
4. **Quick Response** - Short queries needing fast answers
5. **Visual Analysis** - Screenshot or image-related requests
6. **Complex Reasoning** - Long prompts or "explain why/how"
7. **General Query** - Default for unmatched patterns

## Routing Logic

```
User Prompt → Intent Detection → Capability Mapping → Model Selection → Execution
                                                          ↓
                                                    Fallback Chain
```

### Selection Process

1. Detect intent from prompt
2. Map intent to recommended models
3. Filter by availability and capabilities
4. Sort by priority
5. Try models in order until success
6. Fall back to offline chain if all fail

## CLI Commands

### Routing Commands
```bash
# Show routing rules
llm route list

# Test routing for a prompt
llm route test "Write a sorting function"

# Show routing configuration
llm route info
```

### Model Commands
```bash
# List all models
llm model list

# Show model details
llm model info deepseek-v2

# Show model capabilities
llm model capabilities
```

## Usage Examples

### Tool Execution (Fast, Local)
```bash
sentient> llm route test "!@ call disk_info"
Testing routing for: "!@ call disk_info"
=====================================

Detected Intent: ToolCall
Recommended Models: ["phi2_local", "mistral:7b-instruct", "llama3:8b"]
Estimated Tokens: 500
Temperature: 0.1

📊 Intent Analysis:
  Performance Required: Realtime
  Context Requirement: 508 tokens
```

### Code Generation (Powerful, Remote)
```bash
sentient> llm route test "Write a binary search tree implementation"
Testing routing for: "Write a binary search tree implementation"
=====================================

Detected Intent: CodeGeneration
Recommended Models: ["deepseek-coder-v2", "llama3:8b", "codellama:13b"]
Estimated Tokens: 2000
Temperature: 0.3

📊 Intent Analysis:
  Performance Required: Balanced
  Context Requirement: 2044 tokens
```

### Offline Fallback
When remote models are unavailable:
```
Primary models failed → Try offline chain → Use phi2_local → Degrade gracefully
```

## Integration with Existing System

The enhanced router integrates seamlessly:

1. **Tool Framework** - Tool calls are routed to fast local models
2. **RAG System** - Complex queries use powerful models with context
3. **Service Manager** - System commands use deterministic models
4. **Boot LLM** - Phi-2 serves as ultimate fallback

## Performance Characteristics

| Intent | Preferred Tier | Latency Target | Context Needs |
|--------|---------------|----------------|---------------|
| Tool Call | Fast | <100ms | Small |
| Quick Response | Fast | <100ms | Small |
| System Analysis | Balanced | <500ms | Medium |
| Code Generation | Balanced | <2s | Large |
| Complex Reasoning | Powerful | No limit | Large |

## Future Enhancements

- Model performance tracking and adaptation
- Dynamic priority adjustment based on success rates
- Cost-aware routing for cloud models
- Multi-model ensemble for critical tasks
- Streaming response aggregation

## Testing

Run the test script to verify routing:
```bash
./scripts/test-llm-routing.sh
```

This tests:
- Routing rule listing
- Model information display
- Intent detection for various prompts
- Model selection logic
- Fallback chain operation