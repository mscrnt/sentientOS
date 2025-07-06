# Models Configuration Schema

The `models.toml` file defines all available LLM models, their capabilities, safety settings, and routing preferences for SentientOS.

## Model Entry Schema

Each model is defined under `[models.model_id]` with the following fields:

### Required Fields

```toml
[models.example_model]
name = "Human Readable Name"           # Display name
provider = "local|ollama|openai"       # Provider type
trusted = true|false                   # Whether model is trusted for system operations
allow_tool_calls = true|false          # Whether model can execute tool calls
capabilities = ["capability1", ...]    # List of supported capabilities
performance_tier = "tier"              # fast|balanced|powerful|specialized
context_length = 8192                  # Maximum context in tokens
priority = 80                          # Higher = preferred (0-100)
use_cases = ["use_case1", ...]        # Intended use cases
```

### Optional Fields

```toml
endpoint = "http://host:port"          # API endpoint (for remote models)
location = "boot|local|remote"         # Where model runs
model_id = "model:tag"                 # Provider-specific model ID
offline_only = true|false              # Only use when offline
safety_notes = "Security notes"        # Human-readable safety information
```

## Safety Flags

### `trusted`
- **true**: Model is from a trusted source and can access system state
- **false**: Model is untrusted and should have limited access

### `allow_tool_calls`
- **true**: Model can execute tool calls (!@, !$, etc.)
- **false**: Model cannot execute tools (analysis/generation only)

⚠️ **Security Note**: Only set both `trusted=true` and `allow_tool_calls=true` for models you fully control or trust. Remote models should generally have `allow_tool_calls=false`.

## Capabilities

Standard capabilities include:
- `tool_calling` - Can interpret and execute tool commands
- `basic_reasoning` - Simple logical reasoning
- `advanced_reasoning` - Complex analysis and planning
- `code_generation` - Generate programming code
- `command_interpretation` - Parse shell commands
- `system_diagnostics` - Analyze system state
- `conversation` - Natural dialogue
- `vision_understanding` - Process images/screenshots
- `long_context` - Handle large inputs

## Performance Tiers

- **fast** (<100ms typical) - For real-time operations
- **balanced** (<500ms typical) - General purpose
- **powerful** (no limit) - Complex tasks
- **specialized** - Specific use cases (e.g., vision)

## Routing Configuration

### Default Routing

```toml
[routing]
default_model = "phi2_local"           # Fallback model
offline_chain = ["phi2_local", "llama3_local"]  # Offline fallback order
```

### Intent-Based Routing

```toml
[routing.intents]
tool_call = ["phi2_local", "mistral_instruct"]
code_generation = ["deepseek_v2", "llama3_local"]
system_analysis = ["llama3_local", "mistral_instruct"]
```

### Performance-Based Routing

```toml
[routing.performance]
realtime = ["phi2_local", "mistral_instruct"]
fast = ["llama3_local", "mistral_instruct"]
balanced = ["llama3_local", "deepseek_v2"]
powerful = ["deepseek_v2"]
```

### Context-Based Routing

```toml
[routing.context]
short = ["phi2_local", "mistral_instruct"]    # <2K tokens
medium = ["llama3_local", "mistral_instruct"]  # <8K tokens
long = ["deepseek_v2"]                         # <32K tokens
```

## Load Balancing

```toml
[load_balancing]
strategy = "capability_first"          # round_robin|capability_first|latency_based
max_concurrent_requests = 3
timeout_ms = 30000
retry_attempts = 2
```

## Example Model Definitions

### Trusted Local Model (Boot)

```toml
[models.phi2_local]
name = "Microsoft Phi-2"
provider = "local"
location = "boot"
trusted = true
allow_tool_calls = true
capabilities = ["tool_calling", "basic_reasoning", "command_interpretation"]
performance_tier = "fast"
context_length = 2048
priority = 100
use_cases = ["tool_interpretation", "quick_responses", "offline_operation"]
safety_notes = "Boot-level model, fully trusted for system operations"
```

### Untrusted Remote Model

```toml
[models.deepseek_v2]
name = "DeepSeek V2 Coder"
provider = "ollama"
endpoint = "http://192.168.69.197:11434"
model_id = "deepseek-coder-v2"
trusted = false
allow_tool_calls = false
capabilities = ["advanced_reasoning", "code_generation", "long_context"]
performance_tier = "powerful"
context_length = 32768
priority = 60
use_cases = ["complex_reasoning", "code_writing", "system_design"]
safety_notes = "Remote model, not trusted for direct tool execution"
```

## CLI Usage Examples

### View routing for a prompt
```bash
llm route test "!@ call disk_info"
llm route test --verbose "Write a sorting function"
```

### Explain routing decision
```bash
llm route explain "Analyze system performance"
```

### Manage model trust
```bash
llm model show-trusted        # List trusted models
llm model info phi2_local     # Show model details
llm model trust new_model     # Instructions to trust a model
```

## Security Best Practices

1. **Minimal Trust**: Only mark models as `trusted` if you control them
2. **Tool Call Restrictions**: Set `allow_tool_calls=false` for all remote/cloud models
3. **Regular Audits**: Periodically review which models have elevated permissions
4. **Offline Fallback**: Ensure offline chain contains only local, trusted models
5. **Logging**: Enable verbose routing to audit model selection decisions

## Adding New Models

1. Choose a unique `model_id`
2. Set appropriate safety flags based on trust level
3. List all capabilities the model supports
4. Assign priority (higher = preferred)
5. Add to relevant routing rules
6. Test with `llm route test` before deployment

## Troubleshooting

- **Model not selected**: Check if model is in routing rules for the detected intent
- **Tool calls failing**: Verify `allow_tool_calls=true` and model is trusted
- **Performance issues**: Review performance tier and context length limits
- **Offline failures**: Ensure offline_chain contains available local models