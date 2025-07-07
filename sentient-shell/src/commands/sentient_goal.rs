use anyhow::{Result, Context};
use clap::Args;
use std::process::Command;
use std::path::Path;
use tokio::runtime::Runtime;

#[derive(Debug, Args)]
pub struct SentientGoalArgs {
    /// The goal to achieve
    #[arg(help = "Natural language goal description")]
    pub goal: String,
    
    /// Show execution details
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,
    
    /// Dry run - show plan without executing
    #[arg(short = 'd', long = "dry-run")]
    pub dry_run: bool,
    
    /// Maximum execution steps
    #[arg(short = 'm', long = "max-steps", default_value = "50")]
    pub max_steps: u32,
    
    /// Save execution trace
    #[arg(short = 't', long = "trace")]
    pub save_trace: bool,
}

pub async fn execute(args: SentientGoalArgs) -> Result<()> {
    println!("🧠 SentientOS Autonomous Goal Execution");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎯 Goal: {}", args.goal);
    println!();
    
    // Check if Python environment is available
    let python_check = Command::new("python3")
        .arg("--version")
        .output()
        .context("Python3 not found. Please ensure Python 3.7+ is installed")?;
    
    if !python_check.status.success() {
        return Err(anyhow::anyhow!("Python3 is required for autonomous execution"));
    }
    
    // Build Python command
    let mut cmd = Command::new("python3");
    cmd.arg("-m")
        .arg("sentient_core.main")
        .arg("--goal")
        .arg(&args.goal)
        .arg("--max-steps")
        .arg(args.max_steps.to_string());
    
    if args.verbose {
        cmd.arg("--verbose");
    }
    
    if args.dry_run {
        cmd.arg("--dry-run");
    }
    
    if args.save_trace {
        cmd.arg("--save-trace");
    }
    
    // Set working directory to SentientOS root
    let sentient_root = Path::new("/mnt/d/Projects/SentientOS");
    if sentient_root.exists() {
        cmd.current_dir(sentient_root);
    }
    
    // Execute
    println!("🚀 Starting autonomous execution...\n");
    
    let output = cmd.output()
        .context("Failed to execute sentient-core")?;
    
    // Display output
    if !output.stdout.is_empty() {
        println!("{}", String::from_utf8_lossy(&output.stdout));
    }
    
    if !output.stderr.is_empty() {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    }
    
    if !output.status.success() {
        return Err(anyhow::anyhow!("Goal execution failed"));
    }
    
    // Show trace location if saved
    if args.save_trace {
        println!("\n📝 Execution trace saved to: logs/sentient_trace.jsonl");
    }
    
    Ok(())
}

/// Execute a goal using the integrated Rust API (future enhancement)
pub async fn execute_integrated(args: SentientGoalArgs) -> Result<()> {
    use crate::bindings::rl_policy::{SimplePythonRL, extract_state_from_prompt};
    use crate::rag_tool_fusion::{TraceLogger, TraceEntry};
    
    println!("🧠 SentientOS Autonomous Goal Execution (Integrated)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎯 Goal: {}", args.goal);
    
    // Initialize RL policy
    let mut rl_policy = SimplePythonRL::new();
    rl_policy.initialize()
        .context("Failed to initialize RL policy")?;
    
    // Extract initial state
    let state = extract_state_from_prompt(&args.goal);
    
    if args.verbose {
        println!("\n📊 Goal Analysis:");
        println!("  Intent: {} (confidence: {:.2})", state.intent_type, state.intent_confidence);
        println!("  Has tool keywords: {}", state.has_tool_keywords);
        println!("  Has query keywords: {}", state.has_query_keywords);
    }
    
    // Plan generation (placeholder)
    println!("\n📋 Planning steps...");
    let plan_steps = vec![
        "1. Analyze goal requirements",
        "2. Select appropriate tools",
        "3. Execute actions",
        "4. Verify goal completion"
    ];
    
    for step in &plan_steps {
        println!("  - {}", step);
    }
    
    if args.dry_run {
        println!("\n✋ Dry run complete - no actions executed");
        return Ok(());
    }
    
    // Execution loop (simplified)
    println!("\n🚀 Executing plan...");
    
    let mut step_count = 0;
    let mut goal_achieved = false;
    
    while !goal_achieved && step_count < args.max_steps {
        step_count += 1;
        
        // Use RL to select action
        let decision = rl_policy.infer(&args.goal, &state.intent_type)?;
        
        if args.verbose {
            println!("\n  Step {}: {} (confidence: {:.2})", 
                step_count, decision.model, decision.confidence);
        }
        
        // Simulate execution
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // Check termination (simplified)
        if step_count >= 3 || decision.tool.is_none() {
            goal_achieved = true;
        }
    }
    
    // Results
    println!("\n✅ Goal execution complete");
    println!("  Total steps: {}", step_count);
    println!("  Status: {}", if goal_achieved { "Success" } else { "Partial" });
    
    Ok(())
}