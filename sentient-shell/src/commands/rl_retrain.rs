use anyhow::{Result, Context};
use clap::Args;
use std::process::Command;
use std::path::Path;

#[derive(Debug, Args)]
pub struct RlRetrainArgs {
    /// Number of epochs for retraining
    #[arg(short = 'e', long = "epochs", default_value = "10")]
    pub epochs: u32,
    
    /// Force retrain even if not enough new traces
    #[arg(short = 'f', long = "force")]
    pub force: bool,
    
    /// Skip evaluation after training
    #[arg(long = "skip-eval")]
    pub skip_eval: bool,
}

pub async fn execute(args: RlRetrainArgs) -> Result<()> {
    println!("🔄 RL Policy Retraining");
    println!("━━━━━━━━━━━━━━━━━━━━━━");
    
    // Check trace file
    let trace_file = Path::new("logs/rl_trace.jsonl");
    if !trace_file.exists() {
        return Err(anyhow::anyhow!("No trace file found at logs/rl_trace.jsonl"));
    }
    
    // Count traces
    let trace_count = std::fs::read_to_string(trace_file)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    
    println!("📊 Total traces available: {}", trace_count);
    
    // Check if we have enough new traces
    let checkpoint_marker = Path::new("rl_agent/.last_retrain_count");
    let last_count = if checkpoint_marker.exists() {
        std::fs::read_to_string(&checkpoint_marker)?
            .trim()
            .parse::<usize>()
            .unwrap_or(0)
    } else {
        0
    };
    
    let new_traces = trace_count.saturating_sub(last_count);
    println!("🆕 New traces since last retrain: {}", new_traces);
    
    if new_traces < 50 && !args.force {
        println!("\n⚠️  Not enough new traces for retraining (minimum: 50)");
        println!("   Use --force to retrain anyway");
        return Ok(());
    }
    
    // Prepare data for retraining
    println!("\n📚 Preparing data for retraining...");
    
    // Split traces into train/test
    let output = Command::new("python3")
        .arg("validate_traces.py")
        .arg("--split")
        .current_dir(".")
        .output()
        .context("Failed to prepare training data")?;
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Data preparation failed: {}", error));
    }
    
    // Run incremental training
    println!("\n🏃 Running incremental training...");
    let mut train_cmd = Command::new("python3");
    train_cmd
        .arg("rl_agent/train_agent.py")
        .arg("--epochs")
        .arg(args.epochs.to_string())
        .current_dir(".");
    
    if args.skip_eval {
        train_cmd.arg("--skip-evaluation");
    }
    
    let output = train_cmd
        .output()
        .context("Failed to run training")?;
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Training failed: {}", error));
    }
    
    // Update checkpoint marker
    std::fs::write(&checkpoint_marker, trace_count.to_string())?;
    
    // Signal live reload
    let reload_signal = Path::new("rl_agent/.live_update");
    std::fs::write(&reload_signal, "")?;
    println!("📡 Live reload signal sent");
    
    // Show results
    println!("\n✅ Retraining Complete!");
    println!("\n📊 Results:");
    
    // Check if new policy exists
    let policy_path = Path::new("rl_agent/rl_policy.pth");
    if policy_path.exists() {
        let metadata = policy_path.metadata()?;
        let modified = metadata.modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        if now - modified < 300 {  // Updated within last 5 minutes
            println!("  ✓ Policy updated successfully");
        }
    }
    
    // Check for training report
    let report_path = Path::new("rl_agent/RL_TRAINING_REPORT.md");
    if report_path.exists() {
        println!("  ✓ Training report available: {}", report_path.display());
    }
    
    // Check for checkpoint
    let checkpoint_dir = Path::new("rl_checkpoints");
    if checkpoint_dir.exists() {
        let checkpoint_count = std::fs::read_dir(checkpoint_dir)?
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().map(|ext| ext == "pkl").unwrap_or(false))
            .count();
        println!("  ✓ {} checkpoints saved", checkpoint_count);
    }
    
    println!("\n💡 Next steps:");
    println!("  - Test the updated policy: rl infer -t \"your prompt\"");
    println!("  - Monitor performance: rl trace summary");
    println!("  - Enable online learning: python3 rl_agent/train_agent.py --live-train");
    
    Ok(())
}