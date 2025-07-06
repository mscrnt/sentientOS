//! LLM function call parser and formatter

use crate::tools::registry::{Tool, get_tool_registry};
use crate::ai_router::stream_parser::CommandPrefix;
use anyhow::{Result, bail, Context};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use regex::Regex;
use lazy_static::lazy_static;

/// Function call extracted from LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Tool ID to execute
    pub tool_id: String,
    
    /// Arguments for the tool
    pub arguments: Option<Value>,
    
    /// Execution prefix (!@, !#, etc.)
    pub prefix: CommandPrefix,
    
    /// Raw text that was parsed
    pub raw_text: String,
}

/// Function call format
#[derive(Debug, Clone, PartialEq)]
pub enum CallFormat {
    /// JSON format: {"tool": "disk_info", "args": {}}
    Json,
    
    /// Natural language: "call disk_info"
    Natural,
    
    /// Command format: "!@ disk_info"
    Command,
    
    /// Structured format: <function>disk_info()</function>
    Structured,
}

lazy_static! {
    /// Regex for command format: !@ tool_name {args}
    static ref CMD_REGEX: Regex = Regex::new(
        r"^(![@#$&~])\s+(?:call\s+)?(\w+)(?:\s+(.+))?$"
    ).unwrap();
    
    /// Regex for structured format: <function>tool(args)</function>
    static ref STRUCT_REGEX: Regex = Regex::new(
        r"<function>(\w+)\((.*?)\)</function>"
    ).unwrap();
    
    /// Regex for natural language: "call tool_name with args"
    static ref NATURAL_REGEX: Regex = Regex::new(
        r"(?i)(?:please\s+)?(?:call|execute|run)\s+(\w+)(?:\s+with\s+(.+))?"
    ).unwrap();
}

/// Function call parser
pub struct FunctionParser {
    /// Supported formats
    formats: Vec<CallFormat>,
    
    /// Whether to validate tools exist
    validate_tools: bool,
}

impl FunctionParser {
    /// Create new parser with all formats
    pub fn new() -> Self {
        Self {
            formats: vec![
                CallFormat::Command,
                CallFormat::Json,
                CallFormat::Structured,
                CallFormat::Natural,
            ],
            validate_tools: true,
        }
    }
    
    /// Create parser for specific format
    pub fn with_format(format: CallFormat) -> Self {
        Self {
            formats: vec![format],
            validate_tools: true,
        }
    }
    
    /// Disable tool validation
    pub fn without_validation(mut self) -> Self {
        self.validate_tools = false;
        self
    }
    
    /// Parse LLM response for function calls
    pub fn parse(&self, text: &str) -> Result<Vec<FunctionCall>> {
        let mut calls = Vec::new();
        
        // Try each line for potential function calls
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            
            // Try each format
            for format in &self.formats {
                if let Ok(Some(call)) = self.parse_with_format(line, format) {
                    calls.push(call);
                    break; // Found a match, skip other formats
                }
            }
        }
        
        // Validate tools if enabled
        if self.validate_tools {
            let registry = get_tool_registry();
            for call in &calls {
                if registry.get(&call.tool_id).is_none() {
                    bail!("Unknown tool: {}", call.tool_id);
                }
            }
        }
        
        Ok(calls)
    }
    
    /// Parse single line with specific format
    fn parse_with_format(&self, line: &str, format: &CallFormat) -> Result<Option<FunctionCall>> {
        match format {
            CallFormat::Command => self.parse_command_format(line),
            CallFormat::Json => self.parse_json_format(line),
            CallFormat::Structured => self.parse_structured_format(line),
            CallFormat::Natural => self.parse_natural_format(line),
        }
    }
    
    /// Parse command format: !@ tool_name {args}
    fn parse_command_format(&self, line: &str) -> Result<Option<FunctionCall>> {
        if let Some(captures) = CMD_REGEX.captures(line) {
            let prefix_str = &captures[1];
            let tool_id = captures[2].to_string();
            let args_str = captures.get(3).map(|m| m.as_str());
            
            let prefix = match prefix_str {
                "!@" => CommandPrefix::Validated,
                "!#" => CommandPrefix::Dangerous,
                "!$" => CommandPrefix::System,
                "!&" => CommandPrefix::Background,
                "!~" => CommandPrefix::Sandboxed,
                _ => return Ok(None),
            };
            
            let arguments = if let Some(args) = args_str {
                Some(self.parse_arguments(args)?)
            } else {
                None
            };
            
            Ok(Some(FunctionCall {
                tool_id,
                arguments,
                prefix,
                raw_text: line.to_string(),
            }))
        } else {
            Ok(None)
        }
    }
    
    /// Parse JSON format
    fn parse_json_format(&self, line: &str) -> Result<Option<FunctionCall>> {
        // Look for JSON object
        if !line.starts_with('{') || !line.ends_with('}') {
            return Ok(None);
        }
        
        let value: Value = match serde_json::from_str::<Value>(line) {
            Ok(v) if v.is_object() => v,
            _ => return Ok(None),
        };
        
        let obj = value.as_object().unwrap();
        
        // Check for tool/function field
        let tool_id = obj.get("tool")
            .or_else(|| obj.get("function"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing tool field"))?;
        
        let arguments = obj.get("args")
            .or_else(|| obj.get("arguments"))
            .cloned();
        
        // Infer prefix from metadata
        let prefix = if obj.get("privileged") == Some(&Value::Bool(true)) {
            CommandPrefix::System
        } else if obj.get("dangerous") == Some(&Value::Bool(true)) {
            CommandPrefix::Dangerous
        } else if obj.get("background") == Some(&Value::Bool(true)) {
            CommandPrefix::Background
        } else if obj.get("sandboxed") == Some(&Value::Bool(true)) {
            CommandPrefix::Sandboxed
        } else {
            CommandPrefix::Validated
        };
        
        Ok(Some(FunctionCall {
            tool_id: tool_id.to_string(),
            arguments,
            prefix,
            raw_text: line.to_string(),
        }))
    }
    
    /// Parse structured format: <function>tool(args)</function>
    fn parse_structured_format(&self, line: &str) -> Result<Option<FunctionCall>> {
        if let Some(captures) = STRUCT_REGEX.captures(line) {
            let tool_id = captures[1].to_string();
            let args_str = captures[2].trim();
            
            let arguments = if !args_str.is_empty() {
                Some(self.parse_arguments(args_str)?)
            } else {
                None
            };
            
            Ok(Some(FunctionCall {
                tool_id,
                arguments,
                prefix: CommandPrefix::Validated,
                raw_text: line.to_string(),
            }))
        } else {
            Ok(None)
        }
    }
    
    /// Parse natural language format
    fn parse_natural_format(&self, line: &str) -> Result<Option<FunctionCall>> {
        if let Some(captures) = NATURAL_REGEX.captures(line) {
            let tool_id = captures[1].to_string();
            let args_str = captures.get(2).map(|m| m.as_str());
            
            let arguments = if let Some(args) = args_str {
                Some(self.parse_natural_args(args)?)
            } else {
                None
            };
            
            Ok(Some(FunctionCall {
                tool_id,
                arguments,
                prefix: CommandPrefix::Validated,
                raw_text: line.to_string(),
            }))
        } else {
            Ok(None)
        }
    }
    
    /// Parse argument string into JSON value
    fn parse_arguments(&self, args: &str) -> Result<Value> {
        let args = args.trim();
        
        // Try JSON first
        if args.starts_with('{') && args.ends_with('}') {
            return serde_json::from_str(args)
                .context("Invalid JSON arguments");
        }
        
        // Try key=value pairs
        if args.contains('=') {
            let mut map = serde_json::Map::new();
            for pair in args.split_whitespace() {
                if let Some((key, value)) = pair.split_once('=') {
                    map.insert(
                        key.to_string(),
                        self.parse_value(value)?,
                    );
                }
            }
            return Ok(Value::Object(map));
        }
        
        // Single value
        Ok(self.parse_value(args)?)
    }
    
    /// Parse natural language arguments
    fn parse_natural_args(&self, args: &str) -> Result<Value> {
        let mut map = serde_json::Map::new();
        
        // Parse patterns like "pid 1234 and force"
        let words: Vec<&str> = args.split_whitespace().collect();
        let mut i = 0;
        
        while i < words.len() {
            let word = words[i];
            
            // Check for key-value pairs
            if i + 1 < words.len() && !words[i + 1].chars().all(|c| c.is_alphabetic()) {
                map.insert(word.to_string(), self.parse_value(words[i + 1])?);
                i += 2;
            }
            // Check for boolean flags
            else if word == "force" || word == "confirm" || word == "yes" {
                map.insert(word.to_string(), Value::Bool(true));
                i += 1;
            }
            // Skip connectors
            else if word == "and" || word == "with" {
                i += 1;
            }
            else {
                i += 1;
            }
        }
        
        Ok(Value::Object(map))
    }
    
    /// Parse single value
    fn parse_value(&self, value: &str) -> Result<Value> {
        // Try number
        if let Ok(n) = value.parse::<i64>() {
            return Ok(Value::Number(n.into()));
        }
        if let Ok(n) = value.parse::<f64>() {
            return Ok(Value::Number(serde_json::Number::from_f64(n).unwrap()));
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
}

/// Format tool information for LLM context
pub struct FunctionFormatter;

impl FunctionFormatter {
    /// Generate system prompt with available tools
    pub fn generate_system_prompt() -> String {
        let registry = get_tool_registry();
        let tools = registry.list();
        
        let mut prompt = String::from(
            "You have access to the following system tools:\n\n"
        );
        
        for tool in tools {
            prompt.push_str(&format!("## {}\n", tool.name));
            prompt.push_str(&format!("ID: {}\n", tool.id));
            prompt.push_str(&format!("Description: {}\n", tool.description));
            
            if tool.requires_privilege {
                prompt.push_str("⚡ Requires elevated privileges\n");
            }
            
            if !tool.tags.is_empty() {
                prompt.push_str(&format!("Tags: {}\n", tool.tags.join(", ")));
            }
            
            if let Some(schema) = &tool.schema {
                prompt.push_str(&format!("Arguments: {}\n", 
                    serde_json::to_string_pretty(schema).unwrap_or_default()));
            }
            
            if !tool.examples.is_empty() {
                prompt.push_str("Examples:\n");
                for example in &tool.examples {
                    prompt.push_str(&format!("  {}\n", example));
                }
            }
            
            prompt.push('\n');
        }
        
        prompt.push_str("\nTo call a tool, use one of these formats:\n");
        prompt.push_str("- Command: !@ tool_id {\"arg\": \"value\"}\n");
        prompt.push_str("- JSON: {\"tool\": \"tool_id\", \"args\": {\"arg\": \"value\"}}\n");
        prompt.push_str("- Natural: Please call tool_id with arg value\n");
        prompt.push_str("\nPrefixes:\n");
        prompt.push_str("- !@ - Validated execution (default)\n");
        prompt.push_str("- !# - Dangerous (requires confirmation)\n");
        prompt.push_str("- !$ - System/privileged\n");
        prompt.push_str("- !& - Background execution\n");
        prompt.push_str("- !~ - Sandboxed execution\n");
        
        prompt
    }
    
    /// Format tool for specific LLM model
    pub fn format_for_model(tool: &Tool, model: &str) -> String {
        match model {
            m if m.contains("gpt") => {
                // OpenAI function calling format
                serde_json::json!({
                    "name": tool.id,
                    "description": tool.description,
                    "parameters": tool.schema,
                }).to_string()
            },
            m if m.contains("claude") => {
                // Anthropic tool use format
                format!("<tool>{}</tool>\n{}\n", tool.id, tool.description)
            },
            _ => {
                // Generic format
                format!("Tool: {} - {}", tool.id, tool.description)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_command_format_parsing() {
        let parser = FunctionParser::new().without_validation();
        
        let result = parser.parse("!@ disk_info").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].tool_id, "disk_info");
        assert_eq!(result[0].prefix, CommandPrefix::Validated);
        
        let result = parser.parse("!$ kill_process {\"pid\": 1234}").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].tool_id, "kill_process");
        assert_eq!(result[0].prefix, CommandPrefix::System);
        assert!(result[0].arguments.is_some());
    }
    
    #[test]
    fn test_json_format_parsing() {
        let parser = FunctionParser::new().without_validation();
        
        let json = r#"{"tool": "memory_info", "args": {}}"#;
        let result = parser.parse(json).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].tool_id, "memory_info");
    }
    
    #[test]
    fn test_natural_language_parsing() {
        let parser = FunctionParser::new().without_validation();
        
        let result = parser.parse("Please call disk_info").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].tool_id, "disk_info");
        
        let result = parser.parse("call kill_process with pid 1234").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].tool_id, "kill_process");
        assert!(result[0].arguments.is_some());
    }
}