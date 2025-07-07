use anyhow::{Result, Context};
use clap::Args;
use crate::bindings::rl_policy::{SimplePythonRL, extract_state_from_prompt};
use crate::rag_tool_fusion::{TraceLogger, TraceEntry};
use chrono::Utc;
use uuid::Uuid;

#[derive(Debug, Args)]
pub struct RlInferArgs {
    /// The prompt to route
    #[arg(help = "Input prompt to route with RL policy")]
    pub prompt: String,
    
    /// Show detailed RL state features
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,
    
    /// Save trace for online learning
    #[arg(short = 't', long = "trace", help = "Append inference to trace log")]
    pub save_trace: bool,
    
    /// Collect user feedback for the trace
    #[arg(short = 'f', long = "feedback", help = "Collect feedback after inference")]
    pub collect_feedback: bool,
}

pub async fn execute(args: RlInferArgs) -> Result<()> {
    println!("🤖 Running RL Policy Inference");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Initialize Python RL
    let mut rl_policy = SimplePythonRL::new();
    rl_policy.initialize()
        .context("Failed to initialize Python RL environment")?;
    
    // Extract state from prompt
    let state = extract_state_from_prompt(&args.prompt);
    
    if args.verbose {
        println!("\n📊 Extracted State:");
        println!("  Intent: {} (confidence: {:.2})", state.intent_type, state.intent_confidence);
        println!("  Prompt length: {}", state.prompt_length);
        println!("  Has tool keywords: {}", state.has_tool_keywords);
        println!("  Has query keywords: {}", state.has_query_keywords);
        println!("  Has code keywords: {}", state.has_code_keywords);
        println!("  Time of day: {}h", state.time_of_day);
        println!("  RAG available: {}", state.rag_available);
        println!();
    }
    
    // Run inference
    println!("🎯 Prompt: \"{}\"", args.prompt);
    println!();
    
    match rl_policy.infer(&args.prompt, &state.intent_type) {
        Ok(decision) => {
            println!("✅ RL Policy Decision:");
            println!("  Model: {}", decision.model);
            println!("  Use RAG: {}", decision.use_rag);
            println!("  Tool: {}", decision.tool.as_ref().unwrap_or(&"None".to_string()));
            println!("  Confidence: {:.2}", decision.confidence);
            println!("  Value estimate: {:.2}", decision.value_estimate);
            println!("  Fallback used: {}", decision.fallback_used);
            
            if decision.fallback_used {
                println!("\n⚠️  Note: Using fallback heuristics (no trained policy found)");
            }
            
            // Show routing logic
            println!("\n🔄 Routing Logic:");
            if decision.use_rag && decision.tool.is_none() {
                println!("  → Pure RAG query to {}", decision.model);
            } else if !decision.use_rag && decision.tool.is_some() {
                println!("  → Direct tool execution: {}", decision.tool.as_ref().unwrap());
            } else if decision.use_rag && decision.tool.is_some() {
                println!("  → Hybrid: RAG first, then tool {}", decision.tool.as_ref().unwrap());
            } else {
                println!("  → LLM query to {}", decision.model);
            }
            
            // Save trace if requested
            if args.save_trace {
                let trace_id = Uuid::new_v4().to_string();
                let start_time = std::time::Instant::now();
                
                // Simulate execution time
                let duration_ms = (50.0 + (decision.confidence * 100.0)) as u64;
                
                // Collect feedback if requested
                let (reward, feedback) = if args.collect_feedback {
                    println!("\n💬 Was this routing decision helpful?");
                    print!("   [y]es / [n]o / [s]kip: ");
                    use std::io::{self, Write};
                    io::stdout().flush()?;
                    
                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    match input.trim().to_lowercase().as_str() {
                        "y" | "yes" => (Some(1.0), Some("positive".to_string())),
                        "n" | "no" => (Some(-1.0), Some("negative".to_string())),
                        _ => (None, None),
                    }
                } else {
                    (None, None)
                };
                
                // Create trace entry
                let trace_entry = TraceEntry {
                    trace_id,
                    timestamp: Utc::now(),
                    prompt: args.prompt.clone(),
                    intent: state.intent_type.clone(),
                    model_used: decision.model.clone(),
                    tool_executed: decision.tool.clone(),
                    rag_used: decision.use_rag,
                    conditions_met: vec![],
                    success: true, // Assume success for inference
                    duration_ms,
                    error_message: None,
                    reward,
                    feedback,
                };
                
                // Append to trace log
                let mut trace_logger = TraceLogger::new("logs/rl_trace.jsonl").await?;
                trace_logger.log_trace(trace_entry).await?;
                
                println!("\n📝 Trace saved to logs/rl_trace.jsonl");
                
                // Check if retraining is needed
                let summary = trace_logger.get_summary().await?;
                let new_traces = summary.total_executions % 50; // Retrain every 50 traces
                if new_traces == 0 && summary.total_executions > 0 {
                    println!("\n🔄 {} traces collected. Consider running 'rl retrain' to update the policy.", 
                        summary.total_executions);
                }
            }
        }
        Err(e) => {
            eprintln!("❌ RL inference failed: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
}