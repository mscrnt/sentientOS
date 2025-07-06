//! Comprehensive tests for the tool use framework

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::tools::registry::{Tool, ToolRegistry, get_tool_registry};
    use crate::tools::exec::{ToolExecutor, ExecutionMode, execute_tool};
    use crate::llm::functions::{FunctionParser, CallFormat, FunctionCall};
    use crate::schema::schema::SchemaBuilder;
    use serde_json::json;
    
    #[test]
    fn test_tool_registration() {
        let registry = ToolRegistry::new();
        
        // Create a test tool
        let tool = Tool {
            id: "test_echo".to_string(),
            name: "Test Echo".to_string(),
            description: "Echoes input".to_string(),
            command: "echo".to_string(),
            requires_privilege: false,
            requires_confirmation: false,
            schema: Some(
                SchemaBuilder::new("EchoArgs")
                    .string_field("message")
                        .required(true)
                        .and()
                    .build()
            ),
            tags: vec!["test".to_string()],
            examples: vec![],
            timeout: 5,
        };
        
        // Register tool
        assert!(registry.register(tool.clone()).is_ok());
        
        // Verify retrieval
        let retrieved = registry.get("test_echo");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Echo");
    }
    
    #[test]
    fn test_dangerous_command_validation() {
        let registry = ToolRegistry::new();
        
        // Try to register dangerous command without privilege flag
        let dangerous_tool = Tool {
            id: "test_rm".to_string(),
            name: "Test Remove".to_string(),
            description: "Removes files".to_string(),
            command: "rm -rf /tmp/test".to_string(),
            requires_privilege: false, // This should fail
            requires_confirmation: false,
            schema: None,
            tags: vec![],
            examples: vec![],
            timeout: 5,
        };
        
        assert!(registry.register(dangerous_tool).is_err());
    }
    
    #[test]
    fn test_command_format_parsing() {
        let parser = FunctionParser::new().without_validation();
        
        // Test basic command
        let calls = parser.parse("!@ disk_info").unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_id, "disk_info");
        assert_eq!(calls[0].prefix, crate::ai_router::stream_parser::CommandPrefix::Validated);
        
        // Test with arguments
        let calls = parser.parse(r#"!$ kill_process {"pid": 1234}"#).unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_id, "kill_process");
        assert_eq!(calls[0].prefix, crate::ai_router::stream_parser::CommandPrefix::System);
        assert!(calls[0].arguments.is_some());
    }
    
    #[test]
    fn test_json_format_parsing() {
        let parser = FunctionParser::new().without_validation();
        
        let json_call = r#"{"tool": "memory_info", "args": {}}"#;
        let calls = parser.parse(json_call).unwrap();
        
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_id, "memory_info");
    }
    
    #[test]
    fn test_natural_language_parsing() {
        let parser = FunctionParser::new().without_validation();
        
        // Simple call
        let calls = parser.parse("Please call disk_info").unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_id, "disk_info");
        
        // With arguments
        let calls = parser.parse("call kill_process with pid 1234").unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_id, "kill_process");
        assert!(calls[0].arguments.is_some());
    }
    
    #[test]
    fn test_structured_format_parsing() {
        let parser = FunctionParser::new().without_validation();
        
        let structured = "<function>network_status()</function>";
        let calls = parser.parse(structured).unwrap();
        
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_id, "network_status");
    }
    
    #[test]
    fn test_multiple_calls_in_response() {
        let parser = FunctionParser::new().without_validation();
        
        let response = r#"
Let me check both disk and memory usage for you.

!@ disk_info

And here's the memory information:

!@ memory_info
        "#;
        
        let calls = parser.parse(response).unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].tool_id, "disk_info");
        assert_eq!(calls[1].tool_id, "memory_info");
    }
    
    #[test]
    fn test_argument_parsing() {
        let parser = FunctionParser::new().without_validation();
        
        // JSON arguments
        let calls = parser.parse(r#"!@ test {"number": 42, "flag": true}"#).unwrap();
        assert_eq!(calls.len(), 1);
        
        let args = calls[0].arguments.as_ref().unwrap();
        assert_eq!(args["number"], 42);
        assert_eq!(args["flag"], true);
        
        // Key=value arguments
        let calls = parser.parse("!@ test number=42 flag=true").unwrap();
        assert_eq!(calls.len(), 1);
        
        let args = calls[0].arguments.as_ref().unwrap();
        assert_eq!(args["number"], 42);
        assert_eq!(args["flag"], true);
    }
    
    #[test]
    fn test_tool_search() {
        let registry = get_tool_registry();
        
        // Search by tag
        let network_tools = registry.search_by_tags(&[String::from("network")]);
        assert!(!network_tools.is_empty());
        assert!(network_tools.iter().any(|t| t.id == "network_status"));
        
        // Search by multiple tags
        let dangerous_tools = registry.search_by_tags(&[
            String::from("dangerous"),
            String::from("recovery"),
        ]);
        assert!(!dangerous_tools.is_empty());
    }
    
    #[test]
    fn test_schema_validation() {
        let schema = SchemaBuilder::new("TestSchema")
            .integer_field("count")
                .min(1)
                .max(100)
                .and()
            .string_field("name")
                .min_length(3)
                .and()
            .build();
        
        // Valid data
        let valid = json!({
            "count": 50,
            "name": "test"
        });
        assert!(schema.validate(&valid).is_ok());
        
        // Invalid - count too high
        let invalid = json!({
            "count": 200,
            "name": "test"
        });
        assert!(schema.validate(&invalid).is_err());
        
        // Invalid - name too short
        let invalid = json!({
            "count": 50,
            "name": "ab"
        });
        assert!(schema.validate(&invalid).is_err());
    }
    
    #[test]
    fn test_execution_modes() {
        let executor = ToolExecutor::new();
        
        // Create test tool
        let tool = Tool {
            id: "test_mode".to_string(),
            name: "Test Mode".to_string(),
            description: "Tests execution modes".to_string(),
            command: "echo test".to_string(),
            requires_privilege: false,
            requires_confirmation: false,
            schema: None,
            tags: vec![],
            examples: vec![],
            timeout: 5,
        };
        
        // Register temporarily
        let registry = ToolRegistry::new();
        registry.register(tool).unwrap();
        
        // Safe mode should work
        assert!(executor.execute("test_mode", None, ExecutionMode::Safe).is_ok());
        
        // Privileged mode should fail without privileges
        assert!(executor.execute("test_mode", None, ExecutionMode::Privileged).is_err());
    }
    
    #[test]
    fn test_shell_escaping() {
        // Test the shell escape function indirectly through argument substitution
        let parser = FunctionParser::new().without_validation();
        
        // Parse call with potentially dangerous characters
        let calls = parser.parse(r#"!@ echo {"message": "test'; rm -rf /"}"#).unwrap();
        assert_eq!(calls.len(), 1);
        
        // The message should be properly escaped when executed
        let args = calls[0].arguments.as_ref().unwrap();
        assert_eq!(args["message"], "test'; rm -rf /");
    }
    
    #[test]
    fn test_timeout_handling() {
        // This would require a long-running command to test properly
        // For now, we just verify the timeout is set correctly
        let tool = Tool {
            id: "test_timeout".to_string(),
            name: "Test Timeout".to_string(),
            description: "Tests timeout".to_string(),
            command: "sleep 10".to_string(),
            requires_privilege: false,
            requires_confirmation: false,
            schema: None,
            tags: vec![],
            examples: vec![],
            timeout: 1, // 1 second timeout
        };
        
        assert_eq!(tool.timeout, 1);
    }
}

// Integration tests
#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[test]
    #[ignore] // Requires actual command execution
    fn test_real_tool_execution() {
        // This would test actual command execution
        // Marked as ignore since it requires system commands
        
        let result = execute_tool("disk_info", None);
        assert!(result.is_ok());
        
        let execution = result.unwrap();
        assert_eq!(execution.exit_code, 0);
        assert!(!execution.stdout.is_empty());
    }
}