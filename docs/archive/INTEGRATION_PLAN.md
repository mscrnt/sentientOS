# SentientOS Integration Plan: From Python Scripts to Rust Services

## Current State
- We have standalone Python scripts for goal processing, LLM observation, and reflection
- The Rust OS has a proper service manager, AI router, and command infrastructure
- Services can be defined with manifests and started automatically

## Integration Goals
1. Convert core functionality from Python to Rust services
2. Use the existing service manager for automatic startup
3. Leverage the AI router for LLM interactions
4. Implement goal processing as a native OS capability

## Phase 1: Create Rust Services

### 1.1 Goal Processor Service
Convert `fast_goal_processor.py` to a Rust service:
- Location: `sentient-shell/src/services/goal_processor.rs`
- Features:
  - 5-second goal execution cycle
  - Command mapping (disk, memory, network monitoring)
  - Reward calculation
  - Activity logging
- Service manifest: `goal-processor.toml`

### 1.2 LLM Observer Service  
Convert `controlled_llm_observer.py` to Rust:
- Location: `sentient-shell/src/services/llm_observer.rs`
- Features:
  - 30-second goal injection cycle
  - Uses existing AI router for Ollama/DeepSeek
  - Fallback goals when LLM unavailable
- Service manifest: `llm-observer.toml`

### 1.3 Reflective Analyzer Service
Convert `reflective_analyzer.py` to Rust:
- Location: `sentient-shell/src/services/reflective_analyzer.rs`
- Features:
  - Analyze activity patterns
  - Generate daily journals
  - Self-improvement goal injection
- Service manifest: `reflective-analyzer.toml`

## Phase 2: Native OS Integration

### 2.1 Update Service Manager
- Add default service manifests for our AI services
- Configure dependencies (e.g., reflective analyzer depends on goal processor)
- Set autostart flags for core services

### 2.2 Create Activity Feed Module
- Location: `sentient-shell/src/activity_feed/mod.rs`
- Unified activity logging for all services
- Structured format for analysis

### 2.3 Enhance Goal Command
- Extend `sentient_goal` command to use native goal processor
- Add subcommands: inject, status, history
- Integration with service manager

## Phase 3: Service Manifests

### Goal Processor Service
```toml
[service]
name = "goal-processor"
command = "sentient-shell"
args = ["service", "run", "goal-processor"]
autostart = true
restart = "always"
restart_delay_ms = 5000

[service.health_check]
command = "sentient-shell service health goal-processor"
interval_ms = 30000
timeout_ms = 5000
retries = 3

[environment]
GOAL_INTERVAL_MS = "5000"
HEARTBEAT_INTERVAL_MS = "60000"

[dependencies]
# No dependencies - base service
```

### LLM Observer Service
```toml
[service]
name = "llm-observer"
command = "sentient-shell"
args = ["service", "run", "llm-observer"]
autostart = true
restart = "on-failure"
restart_delay_ms = 10000

[environment]
OLLAMA_URL = "http://192.168.69.197:11434"
INJECTION_INTERVAL_MS = "30000"

[dependencies]
depends_on = ["goal-processor", "ai-router"]
```

### Reflective Analyzer Service
```toml
[service]
name = "reflective-analyzer"
command = "sentient-shell"
args = ["service", "run", "reflective-analyzer"]
autostart = true
restart = "on-failure"
restart_delay_ms = 5000

[environment]
ANALYSIS_INTERVAL_MS = "300000"  # 5 minutes

[dependencies]
depends_on = ["goal-processor"]
```

## Phase 4: Admin Interface

### 4.1 Native Admin Command
- `sentient-shell admin` - CLI-based admin interface
- Real-time service status
- Goal injection
- Activity monitoring

### 4.2 Web Interface (Optional)
- Use existing Rust web framework (actix-web or warp)
- Single binary, no external dependencies
- Served by admin-web service

## Benefits of This Approach

1. **True OS Integration**: Services start with the OS, managed by service manager
2. **Resource Efficiency**: Rust services use less memory than Python
3. **Type Safety**: Rust's type system prevents many runtime errors
4. **Performance**: Native code execution, no interpreter overhead
5. **Unified Architecture**: Everything runs within the SentientOS framework

## Implementation Order

1. Start with goal processor service (core functionality)
2. Add activity feed module (needed by other services)
3. Implement LLM observer service
4. Add reflective analyzer service
5. Create admin interface
6. Remove Python dependencies

## Docker Integration

Update Dockerfile to:
- Build all Rust components
- Copy service manifests to config directory
- Start sentient-shell as init process
- Services auto-start based on manifests

No more manual script starting - the OS handles everything!