# Activity Loop Service

The Activity Loop Service is a Rust implementation that migrates functionality from the Python `fast_goal_processor.py`. It provides continuous goal processing with command execution and reward calculation.

## Features

- **5-Second Processing Interval**: Checks for new goals every 5 seconds
- **Command Execution**: Maps goals to executable system commands
- **Reward Calculation**: Calculates rewards (0.0-1.0) based on command output
- **60-Second Heartbeat**: Automatically injects health check goals
- **Async/Tokio**: Fully async implementation using Tokio runtime
- **Structured Logging**: Integrates with the SentientOS logging system

## Goal-to-Command Mapping

The service maps natural language goals to system commands:

| Goal Pattern | Command |
|--------------|---------|
| Disk activity/IO | `df -h` and `iostat` |
| Memory usage | `free -h` |
| Network activity | `netstat` and `ss` |
| CPU usage | `uptime` and `top` |
| Process count | `ps aux | wc -l` |
| System health | Combined uptime, disk, and memory |
| Log analysis | Find and grep recent logs |
| Service status | Check SentientOS services |

## Reward Calculation

Rewards are calculated based on:
- Base reward (0.3) for successful execution
- +0.2 for output length > 50 characters
- +0.2 for structured output (contains `:` or `|`)
- +0.2 for numeric data
- -0.1 for error messages

## Usage

### Via Service Manager

```bash
# Start the service
sentient-shell service start activity-loop

# Check service status
sentient-shell service status activity-loop
```

### Direct Usage

```rust
use sentient_shell::services::activity_loop::ActivityLoopService;
use sentient_shell::services::SentientService;

#[tokio::main]
async fn main() {
    let mut service = ActivityLoopService::new();
    service.init().await.unwrap();
    service.run().await.unwrap();
}
```

### Goal Injection

Goals are read from `logs/goal_injections.jsonl`:

```json
{"goal":"Check memory usage","source":"user","timestamp":"2024-01-01T00:00:00Z","processed":false}
{"goal":"Monitor disk activity","source":"system","timestamp":"2024-01-01T00:00:00Z","processed":false}
```

## Configuration

Service configuration in `config/services/activity-loop.toml`:

```toml
[service]
name = "activity-loop"
enabled = true

[service.config]
check_interval_ms = 5000      # 5 seconds
heartbeat_interval_ms = 60000 # 60 seconds
logs_dir = "logs"
```

## Output

Execution results are logged to `logs/activity_loop_log_YYYYMMDD.jsonl`:

```json
{
  "timestamp": "2024-01-01T00:00:00Z",
  "goal": "Check memory usage",
  "source": "user",
  "command": "free -h | grep -E '^Mem:' ...",
  "output": "Memory: Total 16G, Used 8G, Free 8G, Available 8G",
  "success": true,
  "reward": 0.9,
  "execution_time": 0.05
}
```

## Testing

Run tests with:

```bash
cargo test activity_loop
```

Example test:
```rust
#[tokio::test]
async fn test_goal_processing() {
    let service = ActivityLoopService::new();
    let cmd = service.goal_to_command("Check memory usage");
    assert!(cmd.contains("free -h"));
}
```

## Integration with SentientOS

The Activity Loop Service integrates with:
- **Goal Processor**: Can work alongside for enhanced goal handling
- **LLM Observer**: Provides system state for LLM context
- **Reflective Analyzer**: Execution data feeds into system analysis
- **Logging System**: All activities are logged for traceability

## Migration from Python

This service replaces `fast_goal_processor.py` with improvements:
- Type safety with Rust
- Better error handling
- Integrated with service manager
- Native async/await support
- Resource limits and monitoring