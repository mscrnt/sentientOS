# SentientOS Reinforcement Learning Crates

This directory contains the modular Rust implementation of reinforcement learning components for SentientOS.

## Crates

### sentient-rl-core
Core traits and types for reinforcement learning:
- **Traits**: `Environment`, `Agent`, `Policy`, `ValueFunction`, etc.
- **Types**: `State`, `Action`, `Observation`, `Reward`, `Trajectory`
- **Spaces**: `DiscreteSpace`, `ContinuousSpace`, `BoxSpace`
- **Error handling**: Unified error types for RL operations

### sentient-rl-agent
Implementation of various RL agents:
- **Random Agent**: Baseline agent for comparisons
- **DQN**: Deep Q-Network (placeholder for full implementation)
- **PPO**: Proximal Policy Optimization (placeholder for full implementation)
- **Utilities**: Experience replay buffers, schedules, normalization

### sentient-rl-env
Collection of RL environments:
- **Classic Control**: CartPole, MountainCar
- **LLM Environments**: Environments for LLM interaction and optimization
- **Wrappers**: TimeLimit, FrameStack, Normalize, etc.
- **Registry**: Dynamic environment registration and creation

## Architecture

The crates follow a modular, trait-based design that allows for:

1. **Type Safety**: Strong typing ensures compile-time correctness
2. **Modularity**: Each component can be used independently
3. **Extensibility**: Easy to add new agents, environments, and algorithms
4. **Async Support**: All operations are async-first for scalability
5. **Integration**: Designed to work seamlessly with SentientOS

## Usage

Add the crates to your `Cargo.toml`:

```toml
[dependencies]
sentient-rl-core = { path = "crates/sentient-rl-core" }
sentient-rl-agent = { path = "crates/sentient-rl-agent" }
sentient-rl-env = { path = "crates/sentient-rl-env" }
```

Basic example:

```rust
use sentient_rl_core::prelude::*;
use sentient_rl_agent::RandomAgent;
use sentient_rl_env::CartPoleEnv;

#[tokio::main]
async fn main() -> Result<()> {
    // Create environment
    let mut env = CartPoleEnv::new(Default::default())?;
    
    // Create agent
    let action_space = env.action_space();
    let agent = RandomAgent::new(action_space);
    
    // Run episode
    let (mut obs, _) = env.reset().await?;
    loop {
        let action = agent.act(&obs).await?;
        let step = env.step(action).await?;
        if step.done {
            break;
        }
        obs = step.observation;
    }
    
    Ok(())
}
```

## Building

From the crates directory:

```bash
cargo build --release
cargo test
cargo run --example cartpole_random
```

## Future Work

- [ ] Implement neural network backends (PyTorch, Candle)
- [ ] Complete DQN and PPO implementations
- [ ] Add more environments (Atari, MuJoCo, etc.)
- [ ] Implement additional algorithms (SAC, A3C, IMPALA)
- [ ] Add distributed training support
- [ ] Integrate with SentientOS cognitive architecture
- [ ] Add visualization and monitoring tools

## Contributing

See the main SentientOS contributing guidelines. Key points:
- Follow Rust best practices and idioms
- Add tests for new functionality
- Document public APIs
- Use clippy and rustfmt