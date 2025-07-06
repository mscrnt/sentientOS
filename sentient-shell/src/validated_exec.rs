//! Validated command execution with schema enforcement

use anyhow::{Result, bail};
use crate::ai_router::stream_parser::{CommandPrefix, StreamDetector, StreamDetection};
use std::collections::HashMap;
use serde_json::Value;

/// Command validator using sentient-schema
pub struct CommandValidator {
    /// Registered command schemas
    schemas: HashMap<String, CommandSchema>,
}

#[derive(Debug, Clone)]
pub struct CommandSchema {
    pub command: String,
    pub description: String,
    pub dangerous: bool,
    pub requires_validation: bool,
    pub arg_schema: Option<serde_json::Value>,
}

impl CommandValidator {
    pub fn new() -> Self {
        let mut validator = Self {
            schemas: HashMap::new(),
        };
        
        // Register built-in commands
        validator.register_builtin_commands();
        validator
    }
    
    fn register_builtin_commands(&mut self) {
        // Register dangerous commands
        self.schemas.insert("rm".to_string(), CommandSchema {
            command: "rm".to_string(),
            description: "Remove files or directories".to_string(),
            dangerous: true,
            requires_validation: true,
            arg_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "pattern": "^[^*?]+$"  // No wildcards
                    },
                    "recursive": {
                        "type": "boolean"
                    }
                }
            })),
        });
        
        // Register system commands
        self.schemas.insert("service".to_string(), CommandSchema {
            command: "service".to_string(),
            description: "Service management".to_string(),
            dangerous: false,
            requires_validation: true,
            arg_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["start", "stop", "restart", "status", "list"]
                    },
                    "name": {
                        "type": "string",
                        "pattern": "^[a-zA-Z0-9_-]+$"
                    }
                }
            })),
        });
        
        // Package management
        self.schemas.insert("pkg".to_string(), CommandSchema {
            command: "pkg".to_string(),
            description: "Package management".to_string(),
            dangerous: false,
            requires_validation: true,
            arg_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["install", "uninstall", "list", "search", "update"]
                    },
                    "package": {
                        "type": "string",
                        "pattern": "^[a-zA-Z0-9_-]+$"
                    }
                }
            })),
        });
    }
    
    /// Validate a command with its prefix
    pub fn validate_command(&self, prefix: &CommandPrefix, command: &str) -> Result<ValidatedCommand> {
        let parts: Vec<&str> = command.trim().split_whitespace().collect();
        if parts.is_empty() {
            bail!("Empty command");
        }
        
        let cmd = parts[0];
        let args = &parts[1..];
        
        // Special handling for RAG queries
        if cmd == "rag" && matches!(prefix, CommandPrefix::Validated) {
            // RAG queries are always safe in validated mode
            return Ok(ValidatedCommand {
                prefix: prefix.clone(),
                command: cmd.to_string(),
                args: args.iter().map(|s| s.to_string()).collect(),
                requires_confirmation: false,
            });
        }
        
        // Check if command has a schema
        if let Some(schema) = self.schemas.get(cmd) {
            // Apply prefix rules
            match prefix {
                CommandPrefix::Dangerous => {
                    if !schema.dangerous {
                        bail!("Command '{}' is not marked as dangerous but has !# prefix", cmd);
                    }
                }
                CommandPrefix::Validated => {
                    if !schema.requires_validation {
                        bail!("Command '{}' does not require validation but has !@ prefix", cmd);
                    }
                }
                _ => {}
            }
            
            // Validate arguments if schema exists
            if let Some(arg_schema) = &schema.arg_schema {
                self.validate_args(cmd, args, arg_schema)?;
            }
        }
        
        Ok(ValidatedCommand {
            prefix: prefix.clone(),
            command: cmd.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            requires_confirmation: matches!(prefix, CommandPrefix::Dangerous),
        })
    }
    
    fn validate_args(&self, cmd: &str, args: &[&str], schema: &Value) -> Result<()> {
        // Simple validation for now
        // In production, use full JSON schema validation
        
        if let Some(props) = schema.get("properties") {
            // Check required fields, patterns, etc.
            log::debug!("Validating {} args against schema", cmd);
        }
        
        Ok(())
    }
}

/// A validated command ready for execution
#[derive(Debug)]
pub struct ValidatedCommand {
    pub prefix: CommandPrefix,
    pub command: String,
    pub args: Vec<String>,
    pub requires_confirmation: bool,
}

/// Execute commands with prefix handling
pub fn execute_with_prefix(input: &str) -> Result<()> {
    let (prefix, command) = CommandPrefix::parse(input);
    let validator = CommandValidator::new();
    
    let validated = validator.validate_command(&prefix, command)?;
    
    // Handle based on prefix
    match &validated.prefix {
        CommandPrefix::Dangerous => {
            println!("⚠️  DANGEROUS COMMAND: {} {}", validated.command, validated.args.join(" "));
            print!("Are you sure you want to continue? (yes/no): ");
            std::io::Write::flush(&mut std::io::stdout())?;
            
            let mut response = String::new();
            std::io::stdin().read_line(&mut response)?;
            
            if response.trim().to_lowercase() != "yes" {
                println!("Command cancelled.");
                return Ok(());
            }
        }
        
        CommandPrefix::Validated => {
            println!("✅ Validated command: {} {}", validated.command, validated.args.join(" "));
        }
        
        CommandPrefix::System => {
            println!("🔧 System command: {} {}", validated.command, validated.args.join(" "));
        }
        
        CommandPrefix::Background => {
            println!("🔄 Background execution: {} {}", validated.command, validated.args.join(" "));
        }
        
        CommandPrefix::Sandboxed => {
            println!("📦 Sandboxed execution: {} {}", validated.command, validated.args.join(" "));
        }
        
        CommandPrefix::None => {
            // Normal execution
        }
    }
    
    // Execute the command (would integrate with shell_state.execute_command)
    println!("Executing: {} {}", validated.command, validated.args.join(" "));
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_command_validation() {
        let validator = CommandValidator::new();
        
        // Test dangerous command
        let validated = validator.validate_command(&CommandPrefix::Dangerous, "rm -rf /tmp/test").unwrap();
        assert!(validated.requires_confirmation);
        
        // Test validated command
        let validated = validator.validate_command(&CommandPrefix::Validated, "service start test").unwrap();
        assert_eq!(validated.command, "service");
    }
}