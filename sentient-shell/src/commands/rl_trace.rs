use anyhow::{Result, Context};
use clap::{Arg, ArgMatches, Command};
use colored::*;

use crate::rag_tool_fusion::TraceLogger;

pub fn cli() -> Command {
    Command::new("rl")
        .about("Reinforcement Learning trace analysis and management")
        .subcommand(
            Command::new("trace")
                .about("Analyze execution traces")
                .subcommand(
                    Command::new("summary")
                        .about("Show summary statistics of all traces")
                )
                .subcommand(
                    Command::new("list")
                        .about("List recent traces")
                        .arg(
                            Arg::new("limit")
                                .short('n')
                                .long("limit")
                                .help("Number of traces to show")
                                .value_name("N")
                                .default_value("10")
                        )
                )
                .subcommand(
                    Command::new("best")
                        .about("Show best performing model/tool combinations")
                )
                .subcommand(
                    Command::new("worst")
                        .about("Show worst performing model/tool combinations")
                )
        )
        .subcommand(
            Command::new("infer")
                .about("Test RL policy inference on a prompt")
                .arg(
                    Arg::new("prompt")
                        .help("The prompt to route")
                        .required(true)
                        .index(1)
                )
                .arg(
                    Arg::new("verbose")
                        .short('v')
                        .long("verbose")
                        .help("Show detailed state features")
                )
        )
        .subcommand(
            Command::new("retrain")
                .about("Retrain RL policy with new traces")
                .arg(
                    Arg::new("epochs")
                        .short('e')
                        .long("epochs")
                        .help("Number of training epochs")
                        .value_name("N")
                        .default_value("10")
                )
                .arg(
                    Arg::new("force")
                        .short('f')
                        .long("force")
                        .help("Force retrain even without enough new traces")
                )
                .arg(
                    Arg::new("skip-eval")
                        .long("skip-eval")
                        .help("Skip evaluation after training")
                )
        )
        .subcommand(
            Command::new("export")
                .about("Export traces for external analysis")
                .arg(
                    Arg::new("format")
                        .short('f')
                        .long("format")
                        .help("Export format")
                        .value_name("FORMAT")
                        .value_parser(["json", "csv"])
                        .default_value("json")
                )
                .arg(
                    Arg::new("output")
                        .short('o')
                        .long("output")
                        .help("Output file path")
                        .value_name("PATH")
                        .required(true)
                )
        )
}

pub async fn execute(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("trace", trace_matches)) => handle_trace_command(trace_matches).await,
        Some(("export", export_matches)) => handle_export_command(export_matches).await,
        Some(("infer", infer_matches)) => handle_infer_command(infer_matches).await,
        Some(("retrain", retrain_matches)) => handle_retrain_command(retrain_matches).await,
        _ => {
            println!("Use 'rl trace summary' to see trace statistics");
            Ok(())
        }
    }
}

async fn handle_trace_command(matches: &ArgMatches) -> Result<()> {
    let trace_logger = TraceLogger::new("logs/rl_trace.jsonl").await?;
    
    match matches.subcommand() {
        Some(("summary", _)) => show_summary(&trace_logger).await,
        Some(("list", list_matches)) => show_recent_traces(&trace_logger, list_matches).await,
        Some(("best", _)) => show_best_performers(&trace_logger).await,
        Some(("worst", _)) => show_worst_performers(&trace_logger).await,
        _ => show_summary(&trace_logger).await,
    }
}

async fn show_summary(logger: &TraceLogger) -> Result<()> {
    let summary = logger.get_summary().await?;
    
    println!("{}", "📊 Execution Trace Summary".bold().cyan());
    println!("{}", "═".repeat(50));
    
    println!("\n{}", "Overall Statistics:".bold());
    println!("  Total Executions: {}", summary.total_executions);
    println!("  Successful: {} ({:.1}%)", 
        summary.successful_executions,
        summary.success_rate * 100.0
    );
    println!("  Average Duration: {:.0}ms", summary.average_duration_ms);
    
    if summary.rewarded_count > 0 {
        println!("  Average Reward: {:.2}", summary.average_reward);
        println!("  Feedback Count: {}", summary.rewarded_count);
    }
    
    println!("\n{}", "Feature Usage:".bold());
    println!("  RAG Used: {} times", summary.rag_used_count);
    println!("  Tools Used: {} times", summary.tool_used_count);
    
    println!("\n{}", "Intent Distribution:".bold());
    for (intent, count) in &summary.intent_distribution {
        let percentage = (*count as f64 / summary.total_executions as f64) * 100.0;
        println!("  {}: {} ({:.1}%)", intent, count, percentage);
    }
    
    println!("\n{}", "Model Usage:".bold());
    for (model, count) in &summary.model_usage {
        let percentage = (*count as f64 / summary.total_executions as f64) * 100.0;
        println!("  {}: {} ({:.1}%)", model, count, percentage);
    }
    
    if !summary.tool_usage.is_empty() {
        println!("\n{}", "Tool Usage:".bold());
        for (tool, count) in &summary.tool_usage {
            println!("  {}: {} times", tool, count);
        }
    }
    
    Ok(())
}

async fn show_recent_traces(logger: &TraceLogger, matches: &ArgMatches) -> Result<()> {
    let limit: usize = matches.get_one::<String>("limit")
        .unwrap()
        .parse()
        .context("Invalid limit value")?;
    
    let traces = logger.load_traces().await?;
    let recent: Vec<_> = traces.entries.iter().rev().take(limit).collect();
    
    println!("{}", format!("📜 Recent {} Traces", recent.len()).bold().cyan());
    println!("{}", "═".repeat(80));
    
    for (i, trace) in recent.iter().enumerate() {
        let status_icon = if trace.success { "✅" } else { "❌" };
        let reward_str = trace.reward
            .map(|r| format!(" [Reward: {:.2}]", r))
            .unwrap_or_default();
        
        println!("\n{}. {} {} {}{}",
            i + 1,
            status_icon,
            trace.timestamp.format("%Y-%m-%d %H:%M:%S"),
            trace.intent.yellow(),
            reward_str.green()
        );
        
        println!("   Prompt: {}", 
            if trace.prompt.len() > 60 {
                format!("{}...", &trace.prompt[..60])
            } else {
                trace.prompt.clone()
            }
        );
        
        println!("   Model: {} | Duration: {}ms", 
            trace.model_used.blue(),
            trace.duration_ms
        );
        
        if let Some(tool) = &trace.tool_executed {
            println!("   Tool: {}", tool.magenta());
        }
        
        if trace.rag_used {
            println!("   RAG: Used");
        }
    }
    
    Ok(())
}

async fn show_best_performers(logger: &TraceLogger) -> Result<()> {
    let traces = logger.load_traces().await?;
    
    // Filter traces with rewards
    let rewarded: Vec<_> = traces.entries.iter()
        .filter(|t| t.reward.is_some())
        .collect();
    
    if rewarded.is_empty() {
        println!("No traces with feedback found. Use the system and provide feedback!");
        return Ok(());
    }
    
    // Group by model and calculate average rewards
    let mut model_performance: std::collections::HashMap<String, (f64, usize)> = 
        std::collections::HashMap::new();
    
    for trace in &rewarded {
        let reward = trace.reward.unwrap();
        let entry = model_performance.entry(trace.model_used.clone()).or_insert((0.0, 0));
        entry.0 += reward;
        entry.1 += 1;
    }
    
    let mut model_averages: Vec<(String, f64)> = model_performance.iter()
        .map(|(model, (total, count))| (model.clone(), total / *count as f64))
        .collect();
    
    model_averages.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    
    println!("{}", "🏆 Best Performing Models".bold().green());
    println!("{}", "═".repeat(50));
    
    for (model, avg_reward) in model_averages.iter().take(5) {
        let count = model_performance[model].1;
        println!("  {} - Avg Reward: {:.2} ({} samples)", 
            model.blue(), 
            avg_reward, 
            count
        );
    }
    
    // Group by intent
    let mut intent_performance: std::collections::HashMap<String, (f64, usize)> = 
        std::collections::HashMap::new();
    
    for trace in &rewarded {
        let reward = trace.reward.unwrap();
        let entry = intent_performance.entry(trace.intent.clone()).or_insert((0.0, 0));
        entry.0 += reward;
        entry.1 += 1;
    }
    
    let mut intent_averages: Vec<(String, f64)> = intent_performance.iter()
        .map(|(intent, (total, count))| (intent.clone(), total / *count as f64))
        .collect();
    
    intent_averages.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    
    println!("\n{}", "🎯 Best Performing Intents".bold().green());
    println!("{}", "═".repeat(50));
    
    for (intent, avg_reward) in intent_averages {
        let count = intent_performance[&intent].1;
        println!("  {} - Avg Reward: {:.2} ({} samples)", 
            intent.yellow(), 
            avg_reward, 
            count
        );
    }
    
    Ok(())
}

async fn show_worst_performers(logger: &TraceLogger) -> Result<()> {
    let traces = logger.load_traces().await?;
    
    // Find failed executions and negative rewards
    let failures: Vec<_> = traces.entries.iter()
        .filter(|t| !t.success || t.reward.map(|r| r < 0.0).unwrap_or(false))
        .collect();
    
    if failures.is_empty() {
        println!("No failures or negative feedback found. Great job! 🎉");
        return Ok(());
    }
    
    println!("{}", "⚠️  Common Failure Patterns".bold().red());
    println!("{}", "═".repeat(50));
    
    // Group failures by tool
    let mut tool_failures: std::collections::HashMap<String, usize> = 
        std::collections::HashMap::new();
    
    for trace in &failures {
        if let Some(tool) = &trace.tool_executed {
            *tool_failures.entry(tool.clone()).or_insert(0) += 1;
        }
    }
    
    let mut tool_failure_list: Vec<_> = tool_failures.into_iter().collect();
    tool_failure_list.sort_by(|a, b| b.1.cmp(&a.1));
    
    if !tool_failure_list.is_empty() {
        println!("\n{}", "Failed Tools:".bold());
        for (tool, count) in tool_failure_list.iter().take(5) {
            println!("  {} - {} failures", tool.red(), count);
        }
    }
    
    // Show recent failures
    println!("\n{}", "Recent Failures:".bold());
    for (i, trace) in failures.iter().rev().take(5).enumerate() {
        println!("\n{}. {} {}", 
            i + 1,
            trace.timestamp.format("%Y-%m-%d %H:%M:%S"),
            trace.intent.yellow()
        );
        println!("   Prompt: {}", 
            if trace.prompt.len() > 60 {
                format!("{}...", &trace.prompt[..60])
            } else {
                trace.prompt.clone()
            }
        );
        if let Some(tool) = &trace.tool_executed {
            println!("   Tool: {} (failed)", tool.red());
        }
        if let Some(reward) = trace.reward {
            println!("   User Feedback: {:.1}", reward);
        }
    }
    
    Ok(())
}

async fn handle_export_command(matches: &ArgMatches) -> Result<()> {
    let format = matches.get_one::<String>("format").unwrap();
    let output_path = matches.get_one::<String>("output").unwrap();
    
    let trace_logger = TraceLogger::new("logs/rl_trace.jsonl").await?;
    let traces = trace_logger.load_traces().await?;
    
    match format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&traces)?;
            tokio::fs::write(output_path, json).await?;
            println!("✅ Exported {} traces to {}", traces.entries.len(), output_path);
        }
        "csv" => {
            let mut csv_content = String::from("trace_id,timestamp,prompt,intent,model,tool,rag_used,success,duration_ms,reward\n");
            
            for entry in &traces.entries {
                csv_content.push_str(&format!(
                    "{},{},{:?},{},{},{},{},{},{},{}\n",
                    entry.trace_id,
                    entry.timestamp.to_rfc3339(),
                    entry.prompt.replace(',', ";"),
                    entry.intent,
                    entry.model_used,
                    entry.tool_executed.as_ref().unwrap_or(&"".to_string()),
                    entry.rag_used,
                    entry.success,
                    entry.duration_ms,
                    entry.reward.map(|r| r.to_string()).unwrap_or_default()
                ));
            }
            
            tokio::fs::write(output_path, csv_content).await?;
            println!("✅ Exported {} traces to {}", traces.entries.len(), output_path);
        }
        _ => unreachable!(),
    }
    
    Ok(())
}

async fn handle_infer_command(matches: &ArgMatches) -> Result<()> {
    let prompt = matches.get_one::<String>("prompt")
        .ok_or_else(|| anyhow::anyhow!("Prompt is required"))?;
    let verbose = matches.get_flag("verbose");
    
    let args = crate::commands::rl_infer::RlInferArgs {
        prompt: prompt.clone(),
        verbose,
        save_trace: false,
        collect_feedback: false,
    };
    
    crate::commands::rl_infer::execute(args).await
}

async fn handle_retrain_command(matches: &ArgMatches) -> Result<()> {
    let epochs = matches.get_one::<String>("epochs")
        .unwrap()
        .parse::<u32>()
        .context("Invalid epochs value")?;
    let force = matches.get_flag("force");
    let skip_eval = matches.get_flag("skip-eval");
    
    let args = crate::commands::rl_retrain::RlRetrainArgs {
        epochs,
        force,
        skip_eval,
    };
    
    crate::commands::rl_retrain::execute(args).await
}

pub async fn handle_command(args: &[&str]) -> Result<()> {
    let app = cli();
    let matches = app.try_get_matches_from(
        std::iter::once("rl").chain(args.iter().map(|&s| s)),
    )?;
    
    execute(&matches).await
}