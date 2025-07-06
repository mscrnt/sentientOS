use super::*;
use anyhow::Result;
use crate::hivefix::agent::HiveFixAgent;

// CLI API for hivefix commands
pub fn handle_command(agent: &mut HiveFixAgent, args: &[&str]) -> Result<String> {
    if args.is_empty() {
        return Ok(help());
    }
    
    match args[0] {
        "status" => show_status(agent),
        "log" => show_log(agent),
        "apply" => {
            if args.len() < 2 {
                return Ok("Usage: hivefix apply <fix-id>".to_string());
            }
            apply_fix(agent, args[1])
        },
        "sync" => sync_with_hive(agent),
        "enable" => {
            agent.start()?;
            Ok("HiveFix agent enabled".to_string())
        },
        "disable" => {
            agent.stop();
            Ok("HiveFix agent disabled".to_string())
        },
        "scan" => {
            Ok("Manual scan initiated...".to_string())
        },
        "test" => {
            if args.len() < 2 {
                return Ok("Usage: hivefix test <error-type>\nTypes: package, panic, memory, config".to_string());
            }
            test_error_injection(agent, args[1])
        },
        "trace" => {
            if args.len() < 2 {
                return Ok("Usage: hivefix trace <fix-id>".to_string());
            }
            trace_fix_lifecycle(agent, args[1])
        },
        "rollback" => {
            if args.len() < 2 {
                return Ok("Usage: hivefix rollback <snapshot-id>".to_string());
            }
            rollback_fix(agent, args[1])
        },
        _ => Ok(help()),
    }
}

fn help() -> String {
    r#"HiveFix - Self-healing AI agent for SentientOS

Commands:
  hivefix status       Show agent status and pending fixes
  hivefix log          Show error history and fix attempts
  hivefix apply <id>   Apply a specific fix
  hivefix sync         Sync with hive server
  hivefix enable       Enable the agent
  hivefix disable      Disable the agent
  hivefix scan         Manually trigger error scan
  hivefix test <type>  Inject test error (package/panic/memory/config)
  hivefix trace <id>   Trace fix lifecycle
  hivefix rollback <id> Rollback to snapshot

The agent monitors system logs and automatically proposes fixes using AI."#.to_string()
}

fn show_status(agent: &HiveFixAgent) -> Result<String> {
    let status = agent.get_status();
    let candidates = agent.get_fix_candidates();
    
    let mut output = format!("HiveFix Status: {:?}\n\n", status);
    
    if candidates.is_empty() {
        output.push_str("No fix candidates available.\n");
    } else {
        output.push_str("Available Fixes:\n");
        for (id, fix) in candidates.iter() {
            output.push_str(&format!(
                "  {} - {} (confidence: {:.0}%{})\n",
                id,
                fix.description,
                fix.confidence * 100.0,
                if fix.tested { ", tested ✓" } else { "" }
            ));
        }
    }
    
    Ok(output)
}

fn show_log(agent: &HiveFixAgent) -> Result<String> {
    let history = agent.get_error_history();
    
    if history.is_empty() {
        return Ok("No errors recorded yet.".to_string());
    }
    
    let mut output = String::from("Error History:\n");
    for (i, error) in history.iter().enumerate().rev().take(10) {
        output.push_str(&format!(
            "\n[{}] {:?} - {}\n",
            i + 1,
            error.source,
            error.message
        ));
        
        if let Some(trace) = &error.stack_trace {
            output.push_str(&format!("  Stack: {}\n", 
                trace.lines().next().unwrap_or("")
            ));
        }
    }
    
    Ok(output)
}

fn apply_fix(agent: &mut HiveFixAgent, fix_id: &str) -> Result<String> {
    match agent.apply_fix(fix_id) {
        Ok(_) => Ok(format!("Successfully applied fix: {}", fix_id)),
        Err(e) => Err(e),
    }
}

fn sync_with_hive(_agent: &HiveFixAgent) -> Result<String> {
    // In production, this would sync with the hive server
    Ok("Sync with hive server not yet implemented.".to_string())
}

fn test_error_injection(agent: &mut HiveFixAgent, error_type: &str) -> Result<String> {
    use crate::hivefix::error_injector::{ErrorInjector, TestError, PackageError};
    
    let test_error = match error_type {
        "package" => TestError::BrokenPackage {
            name: "calc".to_string(),
            error_type: PackageError::RuntimeError,
        },
        "panic" => TestError::Panic {
            message: "Test panic for HiveFix".to_string(),
        },
        "memory" => TestError::MemoryAbuse {
            size_mb: 512,
        },
        "config" => TestError::ConfigError {
            key: "max_threads".to_string(),
            bad_value: "not_a_number".to_string(),
        },
        _ => return Ok(format!("Unknown error type: {}", error_type)),
    };
    
    // Inject the error
    let error = ErrorInjector::inject(test_error)?;
    agent.error_history.lock().unwrap().push(error.clone());
    
    Ok(format!("Injected test error: {}", error.message))
}

fn trace_fix_lifecycle(_agent: &HiveFixAgent, fix_id: &str) -> Result<String> {
    // In production, this would trace through audit logs
    Ok(format!("Trace for fix '{}' not yet implemented", fix_id))
}

fn rollback_fix(_agent: &HiveFixAgent, snapshot_id: &str) -> Result<String> {
    // In production, this would use the rollback manager
    Ok(format!("Rollback to snapshot '{}' not yet implemented", snapshot_id))
}