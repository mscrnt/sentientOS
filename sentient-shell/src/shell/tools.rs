//! Shell integration for tool execution

use crate::tools::{
    registry::{Tool, get_tool_registry},
    exec::{ToolExecutor, ExecutionMode, ExecutionResult, execute_tool_with_mode},
};
use crate::llm::functions::{FunctionParser, FunctionCall, FunctionFormatter};
use crate::ai_router::stream_parser::CommandPrefix;
use anyhow::{Result, bail};
use serde_json::Value;

/// Tool command handler for shell
pub struct ToolHandler {
    /// Whether to enable auto-discovery of tools from LLM responses
    auto_discovery: bool,
    
    /// Whether to show execution details
    verbose: bool,
}

impl ToolHandler {
    /// Create new tool handler
    pub fn new() -> Self {
        Self {
            auto_discovery: true,
            verbose: false,
        }
    }
    
    /// Enable verbose output
    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }
    
    /// Handle tool-related commands
    pub fn handle_command(&self, args: &[&str]) -> Result<()> {
        if args.is_empty() {
            self.show_usage();
            return Ok(());
        }
        
        match args[0] {
            "list" => self.list_tools(),
            "info" => self.show_tool_info(&args[1..]),
            "call" => self.call_tool(&args[1..]),
            "search" => self.search_tools(&args[1..]),
            "help" => self.show_help(&args[1..]),
            _ => {
                println!("Unknown tool command: {}", args[0]);
                self.show_usage();
                Ok(())
            }
        }
    }
    
    /// Process AI response for tool calls
    pub fn process_ai_response(&self, response: &str) -> Result<Vec<ExecutionResult>> {
        if !self.auto_discovery {
            return Ok(Vec::new());
        }
        
        let parser = FunctionParser::new();
        let calls = parser.parse(response)?;
        
        if calls.is_empty() {
            return Ok(Vec::new());
        }
        
        println!("\n🔧 Detected {} tool call(s) in response", calls.len());
        
        let mut results = Vec::new();
        for call in calls {
            match self.execute_function_call(&call) {
                Ok(result) => {
                    self.display_result(&call, &result);
                    results.push(result);
                },
                Err(e) => {
                    println!("❌ Failed to execute {}: {}", call.tool_id, e);
                }
            }
        }
        
        Ok(results)
    }
    
    /// Execute a function call
    fn execute_function_call(&self, call: &FunctionCall) -> Result<ExecutionResult> {
        let mode = match call.prefix {
            CommandPrefix::Validated => ExecutionMode::Safe,
            CommandPrefix::Dangerous => ExecutionMode::Safe, // Still safe, but with confirmation
            CommandPrefix::System => ExecutionMode::Privileged,
            CommandPrefix::Background => ExecutionMode::Background,
            CommandPrefix::Sandboxed => ExecutionMode::Sandboxed,
            CommandPrefix::None => ExecutionMode::Safe,
        };
        
        execute_tool_with_mode(&call.tool_id, call.arguments.clone(), mode)
    }
    
    /// Display execution result
    fn display_result(&self, call: &FunctionCall, result: &ExecutionResult) {
        println!("\n📋 Tool: {}", call.tool_id);
        
        if self.verbose {
            println!("   Mode: {:?}", call.prefix);
            if let Some(args) = &call.arguments {
                println!("   Args: {}", serde_json::to_string(args).unwrap_or_default());
            }
            println!("   Time: {}ms", result.duration_ms);
        }
        
        if result.exit_code == 0 {
            println!("   ✅ Success");
        } else {
            println!("   ❌ Failed (exit code: {})", result.exit_code);
        }
        
        if !result.stdout.is_empty() {
            println!("\nOutput:");
            for line in result.stdout.lines() {
                println!("  {}", line);
            }
        }
        
        if !result.stderr.is_empty() {
            println!("\nErrors:");
            for line in result.stderr.lines() {
                println!("  {}", line);
            }
        }
        
        if result.interrupted {
            println!("\n⚠️  Execution was interrupted (timeout)");
        }
    }
    
    /// Show usage information
    fn show_usage(&self) {
        println!("Usage: tool <subcommand>");
        println!("Subcommands:");
        println!("  list              - List all available tools");
        println!("  info <tool>       - Show detailed information about a tool");
        println!("  call <tool> [args] - Execute a tool");
        println!("  search <query>    - Search tools by name or tag");
        println!("  help [tool]       - Show help for tools or specific tool");
    }
    
    /// List all available tools
    fn list_tools(&self) -> Result<()> {
        let registry = get_tool_registry();
        let tools = registry.list();
        
        if tools.is_empty() {
            println!("No tools registered");
            return Ok(());
        }
        
        println!("Available tools:\n");
        
        // Group by category (using first tag as category)
        let mut by_category: std::collections::HashMap<String, Vec<&Tool>> = 
            std::collections::HashMap::new();
        
        for tool in &tools {
            let category = tool.tags.first()
                .map(|s| s.as_str())
                .unwrap_or("other");
            by_category.entry(category.to_string())
                .or_default()
                .push(tool);
        }
        
        // Sort categories
        let mut categories: Vec<_> = by_category.keys().cloned().collect();
        categories.sort();
        
        for category in categories {
            println!("{}:", category.to_uppercase());
            
            if let Some(tools) = by_category.get(&category) {
                for tool in tools {
                    let privilege = if tool.requires_privilege { "⚡" } else { "  " };
                    println!("{} {:15} - {}", privilege, tool.id, tool.description);
                }
            }
            println!();
        }
        
        println!("Legend: ⚡ = Requires elevated privileges");
        
        Ok(())
    }
    
    /// Show detailed tool information
    fn show_tool_info(&self, args: &[&str]) -> Result<()> {
        if args.is_empty() {
            bail!("Usage: tool info <tool_id>");
        }
        
        let tool_id = args[0];
        let registry = get_tool_registry();
        
        let tool = registry.get(tool_id)
            .ok_or_else(|| anyhow::anyhow!("Tool '{}' not found", tool_id))?;
        
        println!("Tool: {} ({})", tool.name, tool.id);
        println!("Description: {}", tool.description);
        println!("Command: {}", tool.command);
        
        if !tool.tags.is_empty() {
            println!("Tags: {}", tool.tags.join(", "));
        }
        
        println!("Timeout: {}s", tool.timeout);
        
        if tool.requires_privilege {
            println!("⚡ Requires elevated privileges");
        }
        
        if tool.requires_confirmation {
            println!("⚠️  Requires confirmation before execution");
        }
        
        if let Some(schema) = &tool.schema {
            println!("\nArguments:");
            println!("{}", serde_json::to_string_pretty(schema)?);
        }
        
        if !tool.examples.is_empty() {
            println!("\nExamples:");
            for example in &tool.examples {
                println!("  {}", example);
            }
        }
        
        Ok(())
    }
    
    /// Call a tool directly
    fn call_tool(&self, args: &[&str]) -> Result<()> {
        if args.is_empty() {
            bail!("Usage: tool call <tool_id> [arguments]");
        }
        
        let tool_id = args[0];
        let tool_args = if args.len() > 1 {
            // Parse arguments
            let args_str = args[1..].join(" ");
            
            // Try to parse as JSON
            if args_str.trim().starts_with('{') {
                Some(serde_json::from_str(&args_str)?)
            } else {
                // Parse as key=value pairs
                let mut map = serde_json::Map::new();
                for arg in &args[1..] {
                    if let Some((key, value)) = arg.split_once('=') {
                        map.insert(
                            key.to_string(),
                            parse_value(value)?,
                        );
                    }
                }
                if !map.is_empty() {
                    Some(Value::Object(map))
                } else {
                    None
                }
            }
        } else {
            None
        };
        
        // Execute with safe mode by default
        let result = execute_tool_with_mode(tool_id, tool_args, ExecutionMode::Safe)?;
        
        // Display result
        let call = FunctionCall {
            tool_id: tool_id.to_string(),
            arguments: None,
            prefix: CommandPrefix::Validated,
            raw_text: args.join(" "),
        };
        
        self.display_result(&call, &result);
        
        Ok(())
    }
    
    /// Search tools by query
    fn search_tools(&self, args: &[&str]) -> Result<()> {
        if args.is_empty() {
            bail!("Usage: tool search <query>");
        }
        
        let query = args.join(" ").to_lowercase();
        let registry = get_tool_registry();
        let tools = registry.list();
        
        let matches: Vec<_> = tools.into_iter()
            .filter(|tool| {
                tool.id.to_lowercase().contains(&query) ||
                tool.name.to_lowercase().contains(&query) ||
                tool.description.to_lowercase().contains(&query) ||
                tool.tags.iter().any(|tag| tag.to_lowercase().contains(&query))
            })
            .collect();
        
        if matches.is_empty() {
            println!("No tools found matching '{}'", query);
        } else {
            println!("Tools matching '{}':\n", query);
            for tool in matches {
                let privilege = if tool.requires_privilege { "⚡" } else { "  " };
                println!("{} {:15} - {}", privilege, tool.id, tool.description);
            }
        }
        
        Ok(())
    }
    
    /// Show help for tools
    fn show_help(&self, args: &[&str]) -> Result<()> {
        if args.is_empty() {
            // Show general tool help
            println!("{}", FunctionFormatter::generate_system_prompt());
        } else {
            // Show help for specific tool
            self.show_tool_info(args)?;
        }
        
        Ok(())
    }
}

/// Parse a value string
fn parse_value(value: &str) -> Result<Value> {
    // Try number
    if let Ok(n) = value.parse::<i64>() {
        return Ok(Value::Number(n.into()));
    }
    
    // Try boolean
    if value == "true" || value == "yes" {
        return Ok(Value::Bool(true));
    }
    if value == "false" || value == "no" {
        return Ok(Value::Bool(false));
    }
    
    // Default to string
    Ok(Value::String(value.to_string()))
}

/// Handle tool command in shell
pub fn handle_tool_command(args: &[&str]) -> Result<()> {
    let handler = ToolHandler::new();
    handler.handle_command(args)
}

/// Process AI response for tool calls
pub fn process_ai_response_for_tools(response: &str) -> Result<Vec<ExecutionResult>> {
    let handler = ToolHandler::new();
    handler.process_ai_response(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_value() {
        assert_eq!(parse_value("123").unwrap(), Value::Number(123.into()));
        assert_eq!(parse_value("true").unwrap(), Value::Bool(true));
        assert_eq!(parse_value("hello").unwrap(), Value::String("hello".to_string()));
    }
}