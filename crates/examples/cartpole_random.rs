//! Example: Random agent playing CartPole

use sentient_rl_agent::RandomAgent;
use sentient_rl_core::{Agent, Environment, TrackedEnvironment};
use sentient_rl_env::{CartPoleEnv, TimeLimit};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Create CartPole environment with time limit
    let env = CartPoleEnv::new(Default::default())?;
    let env = TimeLimit::new(env, 500);
    let mut env = TrackedEnvironment::new(env);
    
    // Create random agent
    let action_space = sentient_rl_core::DiscreteSpace::new(2);
    let mut agent = RandomAgent::new(action_space);
    
    // Run episodes
    let num_episodes = 10;
    let mut episode_rewards = Vec::new();
    
    for episode in 0..num_episodes {
        let (mut observation, _info) = env.reset().await?;
        let mut total_reward = 0.0;
        let mut steps = 0;
        
        loop {
            // Select action
            let action = agent.act(&observation).await?;
            
            // Take step
            let step = env.step(action).await?;
            total_reward += step.reward.0;
            steps += 1;
            
            // Observe (for learning agents)
            agent.observe(&step).await?;
            
            if step.done || step.truncated {
                break;
            }
            
            observation = step.observation;
        }
        
        episode_rewards.push(total_reward);
        println!(
            "Episode {}: Total Reward = {:.2}, Steps = {}",
            episode + 1,
            total_reward,
            steps
        );
    }
    
    // Print statistics
    let avg_reward: f64 = episode_rewards.iter().sum::<f64>() / episode_rewards.len() as f64;
    println!("\nAverage Reward over {} episodes: {:.2}", num_episodes, avg_reward);
    
    // Close environment
    env.close().await?;
    
    Ok(())
}