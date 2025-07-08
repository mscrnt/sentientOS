//! Example: Demonstrating the LLM environment

use sentient_rl_core::{Environment, EnvironmentConfig};
use sentient_rl_env::LLMEnv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Create LLM environment
    let config = EnvironmentConfig {
        seed: Some(42),
        max_steps: Some(10),
        render_mode: None,
        params: serde_json::json!({
            "max_turns": 8,
            "reward_type": "coherence",
            "model_name": "mock-gpt",
            "temperature": 0.7,
        }).as_object().unwrap().clone(),
    };
    
    let mut env = LLMEnv::new(config)?;
    env.set_task("Discuss the benefits of reinforcement learning".to_string());
    
    // Reset environment
    let (mut observation, _info) = env.reset().await?;
    println!("Initial observation: {:?}", observation);
    
    // Take a few steps with different actions
    let actions = vec![0, 1, 2, 7, 8, 9]; // Various conversation strategies
    
    for (i, action_id) in actions.iter().enumerate() {
        let action = sentient_rl_core::DiscreteAction(*action_id);
        println!("\nStep {}: Taking action {}", i + 1, action_id);
        
        let step = env.step(action).await?;
        println!("Reward: {:.2}", step.reward.0);
        println!("Done: {}", step.done);
        println!("Observation: {:?}", step.observation);
        
        if step.done {
            println!("Episode finished!");
            break;
        }
        
        observation = step.observation;
    }
    
    Ok(())
}