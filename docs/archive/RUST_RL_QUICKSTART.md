# Rust RL System Quick Start Guide

## Overview
SentientOS now uses a pure Rust reinforcement learning implementation. All Python RL code has been removed.

## Starting the System

### Basic Usage
```bash
# Start all services
docker-compose up -d

# Check status
docker ps | grep sentientos
docker logs sentientos-runtime
```

### RL Training Mode
```bash
# Start with RL training enabled
SENTIENT_MODE=training docker-compose up

# Custom RL configuration
RL_AGENT=ppo RL_ENV=goal-task RL_EPISODES=5000 docker-compose up
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `SENTIENT_MODE` | `service` | Set to `training` for RL mode |
| `RL_AGENT` | `ppo` | RL algorithm: `ppo`, `random` |
| `RL_ENV` | `goal-task` | Training environment |
| `RL_EPISODES` | `1000` | Number of training episodes |
| `RUST_LOG` | `info` | Logging level |

## Volume Mounts

- `/var/rl_checkpoints` - RL model checkpoints
- `/sentientos/logs` - Application logs
- `/sentientos/traces` - Trace data

## RL Commands (with sentientctl binary)

```bash
# Train an agent
sentientctl rl train --agent ppo --env goal-task

# List policies
sentientctl rl policy list

# View reward graph
sentientctl rl reward-graph

# Inject policy into goal system
sentientctl rl inject-policy
```

## Monitoring

- Port 8080: Monitoring dashboard
- Port 8081: RL dashboard
- Port 8082: Additional services

## Troubleshooting

### Container stops immediately
Check logs: `docker logs sentientos-runtime`

### RL training not starting
Ensure `SENTIENT_MODE=training` is set

### Permission issues
Verify `/var/rl_checkpoints` has correct permissions:
```bash
sudo chown -R $(id -u):$(id -g) /var/rl_checkpoints
```

## Architecture

```
┌─────────────────────────────────────┐
│         Docker Container            │
├─────────────────────────────────────┤
│  Rust RL Components:                │
│  - sentient-rl-core                 │
│  - sentient-rl-agent (PPO)          │
│  - sentient-rl-env                  │
│  - sentient-memory (replay buffer)  │
├─────────────────────────────────────┤
│  Core Services:                     │
│  - Goal Processor                   │
│  - LLM Observer                     │
│  - Shell Interface                  │
└─────────────────────────────────────┘
```

The system is now fully Rust-based with no Python dependencies for RL functionality.