// SentientOS Control CLI
// Unified command-line interface for system management

use anyhow::{Result, Context};
use clap::{Parser, Subcommand};
use chrono::{DateTime, Utc};
use serde_json::json;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

mod rl_commands;

#[derive(Parser)]
#[command(name = "sentientctl")]
#[command(about = "SentientOS Control CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Inject a goal into the system
    InjectGoal {
        /// The goal to inject
        goal: String,
        
        /// Priority level (low, medium, high)
        #[arg(short, long, default_value = "medium")]
        priority: String,
        
        /// Source of the goal
        #[arg(short, long, default_value = "cli")]
        source: String,
    },
    
    /// View system logs
    Logs {
        /// Number of lines to show
        #[arg(short = 'n', long, default_value = "20")]
        lines: usize,
        
        /// Filter by goal ID
        #[arg(long)]
        goal_id: Option<String>,
        
        /// Filter by tool
        #[arg(long)]
        tool: Option<String>,
        
        /// Show logs since timestamp
        #[arg(long)]
        since: Option<String>,
        
        /// Show only failed executions
        #[arg(long)]
        failed_only: bool,
        
        /// Follow log output
        #[arg(short, long)]
        follow: bool,
    },
    
    /// Service management
    Service {
        #[command(subcommand)]
        action: ServiceAction,
    },
    
    /// System monitoring
    Monitor {
        /// Monitoring interval in seconds
        #[arg(short, long, default_value = "5")]
        interval: u64,
    },
    
    /// Validate system configuration
    Validate {
        /// Component to validate
        component: Option<String>,
    },
    
    /// Reinforcement Learning commands
    #[command(subcommand)]
    RL(RLCommands),
}

#[derive(Subcommand)]
enum RLCommands {
    /// Start training an RL agent
    Train {
        /// Agent type (ppo, dqn, random)
        #[arg(short, long, default_value = "ppo")]
        agent: String,
        
        /// Environment name (cartpole, jsonl, goal-task)
        #[arg(short, long, default_value = "goal-task")]
        env: String,
        
        /// Number of training episodes
        #[arg(long, default_value = "1000")]
        episodes: usize,
        
        /// Path to JSONL trace file (for jsonl env)
        #[arg(long)]
        trace_file: Option<String>,
        
        /// Save checkpoint interval
        #[arg(long, default_value = "100")]
        checkpoint_interval: usize,
    },
    
    /// Show policy information
    Policy {
        #[command(subcommand)]
        action: PolicyAction,
    },
    
    /// Display reward graph
    RewardGraph {
        /// Number of recent episodes to show
        #[arg(short = 'n', long, default_value = "100")]
        episodes: usize,
    },
    
    /// Inject goal from trained policy
    InjectPolicy {
        /// Policy checkpoint ID
        checkpoint_id: Option<String>,
    },
}

#[derive(Subcommand)]
enum PolicyAction {
    /// List all policy checkpoints
    List,
    
    /// Show detailed policy information
    Show {
        /// Checkpoint ID
        id: String,
    },
    
    /// Load a policy checkpoint
    Load {
        /// Checkpoint ID
        id: String,
    },
    
    /// Compare two policies
    Compare {
        /// First checkpoint ID
        id1: String,
        /// Second checkpoint ID
        id2: String,
    },
}

#[derive(Subcommand)]
enum ServiceAction {
    /// List all services
    List,
    
    /// Show service status
    Status {
        /// Service name
        name: String,
    },
    
    /// Start a service
    Start {
        /// Service name
        name: String,
    },
    
    /// Stop a service
    Stop {
        /// Service name
        name: String,
    },
    
    /// Restart a service
    Restart {
        /// Service name
        name: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::InjectGoal { goal, priority, source } => {
            inject_goal(&goal, &priority, &source)?;
        }
        
        Commands::Logs {
            lines,
            goal_id,
            tool,
            since,
            failed_only,
            follow,
        } => {
            if follow {
                follow_logs()?;
            } else {
                show_logs(lines, goal_id, tool, since, failed_only)?;
            }
        }
        
        Commands::Service { action } => {
            handle_service_command(action)?;
        }
        
        Commands::Monitor { interval } => {
            monitor_system(interval)?;
        }
        
        Commands::Validate { component } => {
            validate_system(component)?;
        }
        
        Commands::RL(rl_cmd) => {
            rl_commands::handle_rl_command(rl_cmd)?;
        }
    }
    
    Ok(())
}

fn inject_goal(goal: &str, priority: &str, source: &str) -> Result<()> {
    let injection = json!({
        "goal": goal,
        "source": source,
        "timestamp": Utc::now().to_rfc3339(),
        "reasoning": format!("Manual injection from {}", source),
        "priority": priority,
        "injected": true,
        "processed": false,
    });
    
    // Write to goal injection file
    let logs_dir = Path::new("logs");
    std::fs::create_dir_all(logs_dir)?;
    
    let injection_file = logs_dir.join("goal_injections.jsonl");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&injection_file)?;
    
    writeln!(file, "{}", serde_json::to_string(&injection)?)?;
    
    println!("✅ Goal injected successfully");
    println!("   Goal: {}", goal);
    println!("   Priority: {}", priority);
    println!("   Source: {}", source);
    
    Ok(())
}

fn show_logs(
    lines: usize,
    goal_id: Option<String>,
    tool: Option<String>,
    since: Option<String>,
    failed_only: bool,
) -> Result<()> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    
    let logs_dir = Path::new("logs");
    let today = Utc::now().format("%Y%m%d");
    
    // Find log files
    let log_files = vec![
        format!("activity_loop_log_{}.jsonl", today),
        format!("goal_processor_log_{}.jsonl", today),
        format!("sentient_log_{}.jsonl", today),
    ];
    
    let mut entries = Vec::new();
    
    for filename in log_files {
        let path = logs_dir.join(&filename);
        if path.exists() {
            let file = File::open(&path)?;
            let reader = BufReader::new(file);
            
            for line in reader.lines() {
                let line = line?;
                if let Ok(entry) = serde_json::from_str::<serde_json::Value>(&line) {
                    // Apply filters
                    if let Some(ref id) = goal_id {
                        if entry.get("goal_id").and_then(|v| v.as_str()) != Some(id) {
                            continue;
                        }
                    }
                    
                    if let Some(ref t) = tool {
                        if !entry.get("tool").and_then(|v| v.as_str())
                            .map(|s| s.contains(t))
                            .unwrap_or(false) {
                            continue;
                        }
                    }
                    
                    if failed_only {
                        if entry.get("success").and_then(|v| v.as_bool()) != Some(false) {
                            continue;
                        }
                    }
                    
                    entries.push((entry, filename.clone()));
                }
            }
        }
    }
    
    // Sort by timestamp
    entries.sort_by_key(|(e, _)| {
        e.get("timestamp")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    });
    
    // Show last N entries
    for (entry, source) in entries.iter().rev().take(lines) {
        let timestamp = entry.get("timestamp").and_then(|v| v.as_str()).unwrap_or("?");
        let goal = entry.get("goal").and_then(|v| v.as_str()).unwrap_or("?");
        let success = entry.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        let reward = entry.get("reward").and_then(|v| v.as_f64()).unwrap_or(0.0);
        
        let status = if success { "✅" } else { "❌" };
        
        println!("{} {} [{}] {} (reward: {:.2})", 
                 timestamp, status, source, goal, reward);
        
        if let Some(output) = entry.get("output").and_then(|v| v.as_str()) {
            println!("   Output: {}...", &output[..50.min(output.len())]);
        }
    }
    
    Ok(())
}

fn follow_logs() -> Result<()> {
    use notify::{Watcher, RecursiveMode, watcher};
    use std::sync::mpsc::channel;
    use std::time::Duration;
    
    println!("📜 Following logs (press Ctrl+C to stop)...\n");
    
    // Set up file watcher
    let (tx, rx) = channel();
    let mut watcher = watcher(tx, Duration::from_secs(1))?;
    
    let logs_dir = Path::new("logs");
    watcher.watch(logs_dir, RecursiveMode::NonRecursive)?;
    
    // Print new log entries as they arrive
    loop {
        match rx.recv() {
            Ok(event) => {
                // Handle file change events
                // Read and display new entries
            }
            Err(e) => eprintln!("Watch error: {:?}", e),
        }
    }
}

fn handle_service_command(action: ServiceAction) -> Result<()> {
    use std::process::Command;
    
    match action {
        ServiceAction::List => {
            let output = Command::new("sentient-shell")
                .args(&["service", "list"])
                .output()?;
            
            print!("{}", String::from_utf8_lossy(&output.stdout));
        }
        
        ServiceAction::Status { name } => {
            let output = Command::new("sentient-shell")
                .args(&["service", "status", &name])
                .output()?;
            
            print!("{}", String::from_utf8_lossy(&output.stdout));
        }
        
        ServiceAction::Start { name } => {
            println!("Starting service: {}", name);
            
            let output = Command::new("sentient-shell")
                .args(&["service", "start", &name])
                .output()?;
            
            if output.status.success() {
                println!("✅ Service started successfully");
            } else {
                eprintln!("❌ Failed to start service");
                eprintln!("{}", String::from_utf8_lossy(&output.stderr));
            }
        }
        
        ServiceAction::Stop { name } => {
            println!("Stopping service: {}", name);
            
            let output = Command::new("sentient-shell")
                .args(&["service", "stop", &name])
                .output()?;
            
            if output.status.success() {
                println!("✅ Service stopped successfully");
            } else {
                eprintln!("❌ Failed to stop service");
                eprintln!("{}", String::from_utf8_lossy(&output.stderr));
            }
        }
        
        ServiceAction::Restart { name } => {
            println!("Restarting service: {}", name);
            
            let output = Command::new("sentient-shell")
                .args(&["service", "restart", &name])
                .output()?;
            
            if output.status.success() {
                println!("✅ Service restarted successfully");
            } else {
                eprintln!("❌ Failed to restart service");
                eprintln!("{}", String::from_utf8_lossy(&output.stderr));
            }
        }
    }
    
    Ok(())
}

fn monitor_system(interval: u64) -> Result<()> {
    use sysinfo::{System, SystemExt, CpuExt, DiskExt};
    use std::thread;
    use std::time::Duration;
    
    println!("🖥️  SentientOS System Monitor");
    println!("   Refresh interval: {}s", interval);
    println!("   Press Ctrl+C to stop\n");
    
    let mut system = System::new_all();
    
    loop {
        system.refresh_all();
        
        // Clear screen (ANSI escape code)
        print!("\x1B[2J\x1B[1;1H");
        
        println!("🖥️  SentientOS System Monitor - {}", Utc::now().format("%H:%M:%S"));
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        
        // CPU
        let cpu_usage = system.global_cpu_info().cpu_usage();
        println!("CPU Usage: {:.1}%", cpu_usage);
        
        // Memory
        let total_mem = system.total_memory();
        let used_mem = system.used_memory();
        let mem_percent = (used_mem as f32 / total_mem as f32) * 100.0;
        println!("Memory: {:.1}% ({:.1} GB / {:.1} GB)", 
                 mem_percent,
                 used_mem as f32 / 1_073_741_824.0,
                 total_mem as f32 / 1_073_741_824.0);
        
        // Disk
        for disk in system.disks() {
            if disk.mount_point() == Path::new("/") {
                let total = disk.total_space();
                let available = disk.available_space();
                let used = total - available;
                let disk_percent = (used as f32 / total as f32) * 100.0;
                
                println!("Disk (/): {:.1}% ({:.1} GB / {:.1} GB)",
                         disk_percent,
                         used as f32 / 1_073_741_824.0,
                         total as f32 / 1_073_741_824.0);
            }
        }
        
        // Processes
        println!("\nTop Processes:");
        let mut processes: Vec<_> = system.processes().values().collect();
        processes.sort_by(|a, b| b.cpu_usage().partial_cmp(&a.cpu_usage()).unwrap());
        
        for (i, process) in processes.iter().take(5).enumerate() {
            println!("{:2}. {:20} CPU: {:5.1}% MEM: {:5.1} MB",
                     i + 1,
                     process.name(),
                     process.cpu_usage(),
                     process.memory() as f32 / 1_048_576.0);
        }
        
        thread::sleep(Duration::from_secs(interval));
    }
}

fn validate_system(component: Option<String>) -> Result<()> {
    println!("🔍 Validating SentientOS configuration...\n");
    
    let mut all_valid = true;
    
    // Check logs directory
    print!("Checking logs directory... ");
    if Path::new("logs").exists() {
        println!("✅");
    } else {
        println!("❌ Missing");
        all_valid = false;
    }
    
    // Check service configs
    print!("Checking service configs... ");
    let config_dir = Path::new("config/services");
    if config_dir.exists() {
        let count = std::fs::read_dir(config_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("toml"))
            .count();
        println!("✅ ({} services)", count);
    } else {
        println!("❌ Missing");
        all_valid = false;
    }
    
    // Check binary
    print!("Checking sentient-shell binary... ");
    if std::process::Command::new("sentient-shell")
        .arg("--version")
        .output()
        .is_ok() {
        println!("✅");
    } else {
        println!("❌ Not found in PATH");
        all_valid = false;
    }
    
    // Check Ollama connectivity
    print!("Checking Ollama connectivity... ");
    match reqwest::blocking::get("http://192.168.69.197:11434/api/tags") {
        Ok(resp) if resp.status().is_success() => println!("✅"),
        _ => {
            println!("❌ Cannot reach Ollama");
            all_valid = false;
        }
    }
    
    println!();
    if all_valid {
        println!("✅ All checks passed!");
    } else {
        println!("❌ Some checks failed. Please fix the issues above.");
    }
    
    Ok(())
}