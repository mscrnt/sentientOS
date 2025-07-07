use anyhow::{Result, Context};
use clap::{Arg, ArgMatches, Command};
use std::path::Path;

use crate::rag_tool_fusion::{RagToolRouter, HybridIntent};
use crate::rag::{RAGConfig, RAGSystem};

pub fn cli() -> Command {
    Command::new("rag_tool")
        .about("Execute hybrid RAG + Tool queries with intelligent routing")
        .arg(
            Arg::new("prompt")
                .help("The query or command to execute")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("explain")
                .long("explain")
                .help("Show detailed execution pipeline")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .help("Show what would be executed without running tools")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("conditions")
                .long("conditions")
                .short('c')
                .help("Path to conditions configuration file")
                .value_name("PATH")
                .default_value("config/conditions.yaml"),
        )
}

pub async fn execute(matches: &ArgMatches) -> Result<()> {
    let prompt = matches.get_one::<String>("prompt").unwrap();
    let explain = matches.get_flag("explain");
    let dry_run = matches.get_flag("dry-run");
    let conditions_path = matches.get_one::<String>("conditions").unwrap();
    
    println!("🧠 Processing hybrid query: {}", prompt);
    
    // Initialize router with configurations
    let router_config = Path::new("config/router_config.toml");
    let rag_config = RAGConfig::default();
    let tool_registry = Path::new("config/tool_registry.toml");
    let conditions = Path::new(conditions_path);
    
    let mut router = RagToolRouter::new(
        router_config,
        rag_config,
        tool_registry,
        conditions,
    ).await.context("Failed to initialize RAG-Tool router")?;
    
    // Execute the pipeline
    let pipeline = router.execute(prompt, explain).await?;
    
    // Display results
    println!("\n📋 Execution Summary:");
    println!("   Intent: {:?}", pipeline.intent);
    println!("   Duration: {}ms", pipeline.duration_ms);
    
    if let Some(rag_resp) = &pipeline.rag_response {
        println!("\n📚 RAG Response:");
        println!("   {}", rag_resp.answer);
        if explain {
            println!("   Sources: {:?}", rag_resp.sources);
            println!("   Confidence: {:.2}", rag_resp.confidence);
        }
    }
    
    if let Some(tool_exec) = &pipeline.tool_execution {
        if !dry_run {
            println!("\n🔧 Tool Execution:");
            println!("   Tool: {}", tool_exec.tool_name);
            println!("   Exit Code: {}", tool_exec.exit_code);
            println!("   Output: {}", tool_exec.output);
        } else {
            println!("\n🔧 Tool Execution (DRY RUN):");
            println!("   Would execute: {}", tool_exec.tool_name);
        }
    }
    
    if !pipeline.conditions_evaluated.is_empty() && explain {
        println!("\n✅ Conditions Evaluated:");
        for condition in &pipeline.conditions_evaluated {
            println!("   - {}", condition);
        }
    }
    
    println!("\n💬 Final Response:");
    println!("{}", pipeline.final_response);
    
    // Prompt for feedback if in interactive mode
    if atty::is(atty::Stream::Stdin) {
        println!("\n✅ Result complete. Was this helpful? [Y/n/s(kip)]");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        let reward = match input.trim().to_lowercase().as_str() {
            "y" | "yes" | "" => Some(1.0),
            "n" | "no" => Some(-1.0),
            "s" | "skip" => None,
            _ => Some(0.0),
        };
        
        if let Some(reward_value) = reward {
            // Update trace with reward
            let trace_logger = crate::rag_tool_fusion::TraceLogger::new("logs/rl_trace.jsonl").await?;
            trace_logger.update_reward(&pipeline.trace_id, reward_value).await?;
            println!("📝 Feedback recorded: {}", reward_value);
        }
    }
    
    Ok(())
}

pub async fn handle_command(args: &[&str]) -> Result<()> {
    let app = cli();
    let matches = app.try_get_matches_from(
        std::iter::once("rag_tool").chain(args.iter().map(|&s| s)),
    )?;
    
    execute(&matches).await
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cli_construction() {
        let cli = cli();
        assert_eq!(cli.get_name(), "rag_tool");
    }
}