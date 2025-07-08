use sentient_shell::services::{ServiceRunner, activity_loop::ActivityLoopService, SentientService};
use anyhow::Result;
use tokio;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use chrono::Utc;

/// Example showing how to run the Activity Loop Service
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();
    
    println!("Activity Loop Service Example");
    println!("============================\n");
    
    // Ensure logs directory exists
    create_dir_all("logs")?;
    
    // Create some example goals
    inject_example_goals()?;
    
    println!("Starting Activity Loop Service...");
    println!("The service will:");
    println!("- Check for new goals every 5 seconds");
    println!("- Execute system monitoring commands");
    println!("- Calculate rewards based on output");
    println!("- Generate heartbeat health checks every 60 seconds\n");
    
    // Run the service using ServiceRunner
    ServiceRunner::run_service("activity-loop").await?;
    
    Ok(())
}

/// Inject some example goals for the service to process
fn inject_example_goals() -> Result<()> {
    let injection_file = "logs/goal_injections.jsonl";
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(injection_file)?;
    
    // Example goals
    let goals = vec![
        ("Check memory usage and available RAM", "monitoring"),
        ("Monitor disk I/O activity", "monitoring"),
        ("Check network connections and traffic", "monitoring"),
        ("Analyze system logs for errors", "diagnostics"),
        ("Count running processes", "monitoring"),
        ("Check overall system health", "health-check"),
    ];
    
    println!("Injecting example goals:");
    for (goal, source) in goals {
        let entry = serde_json::json!({
            "goal": goal,
            "source": source,
            "timestamp": Utc::now().to_rfc3339(),
            "processed": false,
            "priority": "normal"
        });
        
        writeln!(file, "{}", entry)?;
        println!("  - {}", goal);
    }
    println!();
    
    Ok(())
}

/// Alternative: Direct service usage without ServiceRunner
#[allow(dead_code)]
async fn run_service_directly() -> Result<()> {
    let mut service = ActivityLoopService::new();
    
    // Initialize
    service.init().await?;
    
    // Run (this will loop forever)
    service.run().await?;
    
    Ok(())
}