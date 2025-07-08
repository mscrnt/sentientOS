// RL command implementations for sentientctl

use anyhow::{Result, Context};
use serde_json::json;
use std::path::Path;

use crate::{RLCommands, PolicyAction, inject_goal};

pub fn handle_rl_command(cmd: RLCommands) -> Result<()> {
    match cmd {
        RLCommands::Train {
            agent,
            env,
            episodes,
            trace_file,
            checkpoint_interval,
        } => {
            start_training(agent, env, episodes, trace_file, checkpoint_interval)?;
        }
        
        RLCommands::Policy { action } => {
            handle_policy_command(action)?;
        }
        
        RLCommands::RewardGraph { episodes } => {
            show_reward_graph(episodes)?;
        }
        
        RLCommands::InjectPolicy { checkpoint_id } => {
            inject_from_policy(checkpoint_id)?;
        }
    }
    
    Ok(())
}

fn start_training(
    agent: String,
    env: String,
    episodes: usize,
    trace_file: Option<String>,
    checkpoint_interval: usize,
) -> Result<()> {
    println!("🤖 Starting RL Training");
    println!("   Agent: {}", agent);
    println!("   Environment: {}", env);
    println!("   Episodes: {}", episodes);
    println!("   Checkpoint interval: {}", checkpoint_interval);
    
    if let Some(ref trace) = trace_file {
        println!("   Trace file: {}", trace);
    }
    
    // Create config file for training
    let config = json!({
        "agent_type": agent,
        "environment": env,
        "episodes": episodes,
        "checkpoint_interval": checkpoint_interval,
        "trace_file": trace_file,
        "log_interval": 10,
        "reward_goal_threshold": 0.8,
    });
    
    // Write config to temporary file
    let config_path = "/tmp/rl_training_config.json";
    std::fs::write(config_path, serde_json::to_string_pretty(&config)?)?;
    
    // Start training process
    println!("\n🚀 Launching training process...");
    
    let mut child = std::process::Command::new("sentient-shell")
        .args(&["rl", "train", "--config", config_path])
        .spawn()
        .context("Failed to start training process")?;
    
    // Wait for training to complete
    let status = child.wait()?;
    
    if status.success() {
        println!("\n✅ Training completed successfully!");
    } else {
        eprintln!("\n❌ Training failed with status: {}", status);
    }
    
    Ok(())
}

fn handle_policy_command(action: PolicyAction) -> Result<()> {
    match action {
        PolicyAction::List => {
            println!("📋 Policy Checkpoints:\n");
            
            let checkpoints_dir = Path::new("/var/rl_checkpoints/policies");
            if !checkpoints_dir.exists() {
                println!("No checkpoints found.");
                return Ok(());
            }
            
            let mut entries = std::fs::read_dir(checkpoints_dir)?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .collect::<Vec<_>>();
            
            entries.sort_by_key(|e| e.path());
            
            for entry in entries {
                if let Ok(metadata_path) = entry.path().join("metadata.json").canonicalize() {
                    if let Ok(content) = std::fs::read_to_string(metadata_path) {
                        if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&content) {
                            let id = entry.file_name().to_string_lossy();
                            let episode = metadata["metadata"]["episode"].as_u64().unwrap_or(0);
                            let reward = metadata["metadata"]["best_reward"].as_f64().unwrap_or(0.0);
                            let created = metadata["created_at"].as_str().unwrap_or("?");
                            
                            println!("ID: {}", id);
                            println!("   Episode: {}", episode);
                            println!("   Best Reward: {:.3}", reward);
                            println!("   Created: {}", created);
                            println!();
                        }
                    }
                }
            }
        }
        
        PolicyAction::Show { id } => {
            let metadata_path = format!("/var/rl_checkpoints/policies/{}/metadata.json", id);
            
            match std::fs::read_to_string(&metadata_path) {
                Ok(content) => {
                    let metadata: serde_json::Value = serde_json::from_str(&content)?;
                    println!("🔍 Policy Details: {}\n", id);
                    println!("{}", serde_json::to_string_pretty(&metadata)?);
                }
                Err(_) => {
                    eprintln!("❌ Policy checkpoint not found: {}", id);
                }
            }
        }
        
        PolicyAction::Load { id } => {
            println!("📥 Loading policy: {}", id);
            
            let result = std::process::Command::new("sentient-shell")
                .args(&["rl", "policy", "load", &id])
                .output()?;
            
            if result.status.success() {
                println!("✅ Policy loaded successfully");
            } else {
                eprintln!("❌ Failed to load policy");
                eprintln!("{}", String::from_utf8_lossy(&result.stderr));
            }
        }
        
        PolicyAction::Compare { id1, id2 } => {
            println!("🔄 Comparing policies: {} vs {}", id1, id2);
            
            // Load both policies
            let metadata1_path = format!("/var/rl_checkpoints/policies/{}/metadata.json", id1);
            let metadata2_path = format!("/var/rl_checkpoints/policies/{}/metadata.json", id2);
            
            let meta1: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(metadata1_path)?
            )?;
            let meta2: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(metadata2_path)?
            )?;
            
            println!("\nPolicy 1 ({})", id1);
            println!("   Episode: {}", meta1["metadata"]["episode"]);
            println!("   Best Reward: {:.3}", meta1["metadata"]["best_reward"].as_f64().unwrap_or(0.0));
            println!("   Avg Reward: {:.3}", meta1["metadata"]["average_reward"].as_f64().unwrap_or(0.0));
            
            println!("\nPolicy 2 ({})", id2);
            println!("   Episode: {}", meta2["metadata"]["episode"]);
            println!("   Best Reward: {:.3}", meta2["metadata"]["best_reward"].as_f64().unwrap_or(0.0));
            println!("   Avg Reward: {:.3}", meta2["metadata"]["average_reward"].as_f64().unwrap_or(0.0));
            
            let reward_diff = meta2["metadata"]["best_reward"].as_f64().unwrap_or(0.0) 
                - meta1["metadata"]["best_reward"].as_f64().unwrap_or(0.0);
            
            println!("\nDifference:");
            println!("   Reward improvement: {:+.3}", reward_diff);
        }
    }
    
    Ok(())
}

fn show_reward_graph(episodes: usize) -> Result<()> {
    println!("📊 Reward Graph (last {} episodes)\n", episodes);
    
    // Read training stats
    let stats_file = Path::new("/var/rl_checkpoints/training_stats.jsonl");
    if !stats_file.exists() {
        println!("No training statistics found.");
        return Ok(());
    }
    
    let content = std::fs::read_to_string(stats_file)?;
    let mut rewards = Vec::new();
    
    for line in content.lines() {
        if let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(reward) = entry["episode_reward"].as_f64() {
                rewards.push(reward);
            }
        }
    }
    
    // Take last N episodes
    let start = rewards.len().saturating_sub(episodes);
    let recent_rewards = &rewards[start..];
    
    if recent_rewards.is_empty() {
        println!("No reward data available.");
        return Ok(());
    }
    
    // Calculate statistics
    let max_reward = recent_rewards.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let min_reward = recent_rewards.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let avg_reward = recent_rewards.iter().sum::<f64>() / recent_rewards.len() as f64;
    
    // Simple ASCII graph
    let graph_height = 10;
    let graph_width = 50;
    
    println!("Max: {:.2}  Avg: {:.2}  Min: {:.2}", max_reward, avg_reward, min_reward);
    println!("┌{}┐", "─".repeat(graph_width));
    
    for h in (0..graph_height).rev() {
        print!("│");
        
        for i in 0..graph_width {
            let episode_idx = (i * recent_rewards.len()) / graph_width;
            if episode_idx < recent_rewards.len() {
                let reward = recent_rewards[episode_idx];
                let normalized = (reward - min_reward) / (max_reward - min_reward + 1e-6);
                let bar_height = (normalized * graph_height as f64) as usize;
                
                if bar_height >= h {
                    print!("█");
                } else {
                    print!(" ");
                }
            } else {
                print!(" ");
            }
        }
        
        println!("│");
    }
    
    println!("└{}┘", "─".repeat(graph_width));
    println!(" Episode {} {} ", start, start + recent_rewards.len());
    
    // Show recent trend
    if recent_rewards.len() >= 10 {
        let recent_10_avg = recent_rewards[recent_rewards.len()-10..].iter().sum::<f64>() / 10.0;
        let older_avg = recent_rewards[..recent_rewards.len()-10].iter().sum::<f64>() 
            / (recent_rewards.len() - 10) as f64;
        
        let trend = if recent_10_avg > older_avg { "📈" } else { "📉" };
        println!("\nTrend: {} ({:+.2}% vs older episodes)", 
                 trend, 
                 ((recent_10_avg / older_avg) - 1.0) * 100.0);
    }
    
    Ok(())
}

fn inject_from_policy(checkpoint_id: Option<String>) -> Result<()> {
    let id = checkpoint_id.unwrap_or_else(|| "latest".to_string());
    
    println!("🎯 Injecting goal from policy: {}", id);
    
    // Call RL runtime to get goal suggestion
    let result = std::process::Command::new("sentient-shell")
        .args(&["rl", "suggest-goal", "--policy", &id])
        .output()?;
    
    if result.status.success() {
        let goal = String::from_utf8_lossy(&result.stdout).trim().to_string();
        
        if !goal.is_empty() {
            println!("   Suggested goal: {}", goal);
            
            // Inject the goal
            inject_goal(&goal, "high", "rl_policy")?;
        } else {
            eprintln!("❌ No goal suggested by policy");
        }
    } else {
        eprintln!("❌ Failed to get goal from policy");
        eprintln!("{}", String::from_utf8_lossy(&result.stderr));
    }
    
    Ok(())
}