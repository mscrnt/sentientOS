//! Tool registry for function calling framework

use crate::schema::{Schema, schema::{SchemaBuilder, FieldType}};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use lazy_static::lazy_static;

/// Tool definition with metadata and schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Unique tool identifier
    pub id: String,
    
    /// Display name
    pub name: String,
    
    /// Description for LLM context
    pub description: String,
    
    /// Command or function to execute
    pub command: String,
    
    /// Required privilege level
    pub requires_privilege: bool,
    
    /// Requires confirmation before execution
    pub requires_confirmation: bool,
    
    /// Arguments schema
    pub schema: Option<Schema>,
    
    /// Tags for categorization
    pub tags: Vec<String>,
    
    /// Example usage
    pub examples: Vec<String>,
    
    /// Maximum execution time in seconds
    pub timeout: u64,
}

/// Tool categories
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolCategory {
    System,
    Network,
    Storage,
    Process,
    Security,
    Diagnostic,
    Recovery,
    Custom(String),
}

/// Tool registry singleton
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Tool>>>,
    categories: Arc<RwLock<HashMap<ToolCategory, Vec<String>>>>,
}

impl ToolRegistry {
    /// Create new registry
    pub fn new() -> Self {
        let registry = Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            categories: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Register default tools
        registry.register_default_tools();
        registry
    }
    
    /// Register a tool
    pub fn register(&self, tool: Tool) -> Result<()> {
        let id = tool.id.clone();
        
        // Validate tool
        self.validate_tool(&tool)?;
        
        // Add to registry
        self.tools.write().unwrap().insert(id.clone(), tool);
        
        log::info!("Registered tool: {}", id);
        Ok(())
    }
    
    /// Get tool by ID
    pub fn get(&self, id: &str) -> Option<Tool> {
        self.tools.read().unwrap().get(id).cloned()
    }
    
    /// List all tools
    pub fn list(&self) -> Vec<Tool> {
        self.tools.read().unwrap().values().cloned().collect()
    }
    
    /// List tools by category
    pub fn list_by_category(&self, category: &ToolCategory) -> Vec<Tool> {
        if let Some(tool_ids) = self.categories.read().unwrap().get(category) {
            tool_ids
                .iter()
                .filter_map(|id| self.get(id))
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Search tools by tags
    pub fn search_by_tags(&self, tags: &[String]) -> Vec<Tool> {
        self.list()
            .into_iter()
            .filter(|tool| {
                tags.iter().any(|tag| tool.tags.contains(tag))
            })
            .collect()
    }
    
    /// Validate tool definition
    fn validate_tool(&self, tool: &Tool) -> Result<()> {
        if tool.id.is_empty() {
            bail!("Tool ID cannot be empty");
        }
        
        if tool.command.is_empty() {
            bail!("Tool command cannot be empty");
        }
        
        // Validate command safety
        if !tool.requires_privilege && self.is_dangerous_command(&tool.command) {
            bail!("Dangerous command requires privilege flag");
        }
        
        Ok(())
    }
    
    /// Check if command is potentially dangerous
    fn is_dangerous_command(&self, command: &str) -> bool {
        let dangerous_patterns = [
            "rm ", "dd ", "format", "mkfs", "fdisk",
            "shutdown", "reboot", "systemctl", "kill",
            "> /dev/", "sudo", "su ",
        ];
        
        dangerous_patterns.iter().any(|pattern| command.contains(pattern))
    }
    
    /// Register default system tools
    fn register_default_tools(&self) {
        // System information
        self.register(Tool {
            id: "disk_info".to_string(),
            name: "Disk Information".to_string(),
            description: "Get disk space usage information".to_string(),
            command: "df -h".to_string(),
            requires_privilege: false,
            requires_confirmation: false,
            schema: None,
            tags: vec!["system".to_string(), "storage".to_string()],
            examples: vec!["!@ call disk_info".to_string()],
            timeout: 5,
        }).ok();
        
        self.register(Tool {
            id: "memory_info".to_string(),
            name: "Memory Information".to_string(),
            description: "Show system memory usage".to_string(),
            command: "free -h".to_string(),
            requires_privilege: false,
            requires_confirmation: false,
            schema: None,
            tags: vec!["system".to_string(), "diagnostic".to_string()],
            examples: vec!["!@ call memory_info".to_string()],
            timeout: 5,
        }).ok();
        
        // Process management
        self.register(Tool {
            id: "process_list".to_string(),
            name: "Process List".to_string(),
            description: "List running processes".to_string(),
            command: "ps aux | head -20".to_string(),
            requires_privilege: false,
            requires_confirmation: false,
            schema: None,
            tags: vec!["process".to_string(), "diagnostic".to_string()],
            examples: vec!["!@ call process_list".to_string()],
            timeout: 5,
        }).ok();
        
        self.register(Tool {
            id: "kill_process".to_string(),
            name: "Kill Process".to_string(),
            description: "Terminate a process by PID".to_string(),
            command: "kill".to_string(),
            requires_privilege: true,
            requires_confirmation: true,
            schema: Some(
                SchemaBuilder::new("KillProcess")
                    .integer_field("pid")
                        .min(1)
                        .and()
                    .boolean_field("force")
                        .default_value(serde_json::json!(false))
                        .and()
                    .build()
            ),
            tags: vec!["process".to_string(), "dangerous".to_string()],
            examples: vec![
                r#"!$ call kill_process {"pid": 1234}"#.to_string(),
                r#"!$ call kill_process {"pid": 5678, "force": true}"#.to_string(),
            ],
            timeout: 5,
        }).ok();
        
        // Network tools
        self.register(Tool {
            id: "network_status".to_string(),
            name: "Network Status".to_string(),
            description: "Show network interface status".to_string(),
            command: "ip addr show".to_string(),
            requires_privilege: false,
            requires_confirmation: false,
            schema: None,
            tags: vec!["network".to_string(), "diagnostic".to_string()],
            examples: vec!["!@ call network_status".to_string()],
            timeout: 5,
        }).ok();
        
        self.register(Tool {
            id: "reset_network".to_string(),
            name: "Reset Network".to_string(),
            description: "Reset network configuration".to_string(),
            command: "/opt/sentient/scripts/reset-network.sh".to_string(),
            requires_privilege: true,
            requires_confirmation: true,
            schema: Some(
                SchemaBuilder::new("ResetNetwork")
                    .boolean_field("confirm")
                        .required(true)
                        .and()
                    .string_field("interface")
                        .default_value(serde_json::json!("all"))
                        .and()
                    .build()
            ),
            tags: vec!["network".to_string(), "recovery".to_string(), "dangerous".to_string()],
            examples: vec![
                r#"!$ call reset_network {"confirm": true}"#.to_string(),
                r#"!$ call reset_network {"confirm": true, "interface": "eth0"}"#.to_string(),
            ],
            timeout: 30,
        }).ok();
        
        // System services
        self.register(Tool {
            id: "service_status".to_string(),
            name: "Service Status".to_string(),
            description: "Check status of a system service".to_string(),
            command: "service".to_string(),
            requires_privilege: false,
            requires_confirmation: false,
            schema: Some(
                SchemaBuilder::new("ServiceStatus")
                    .string_field("name")
                        .min_length(1)
                        .and()
                    .build()
            ),
            tags: vec!["service".to_string(), "diagnostic".to_string()],
            examples: vec![
                r#"!@ call service_status {"name": "sentd"}"#.to_string(),
            ],
            timeout: 5,
        }).ok();
        
        // Recovery tools
        self.register(Tool {
            id: "safe_mode".to_string(),
            name: "Enter Safe Mode".to_string(),
            description: "Restart system in safe mode".to_string(),
            command: "/opt/sentient/scripts/safe-mode.sh".to_string(),
            requires_privilege: true,
            requires_confirmation: true,
            schema: Some(
                SchemaBuilder::new("SafeMode")
                    .boolean_field("confirm")
                        .required(true)
                        .and()
                    .build()
            ),
            tags: vec!["recovery".to_string(), "dangerous".to_string()],
            examples: vec![
                r#"!$ call safe_mode {"confirm": true}"#.to_string(),
            ],
            timeout: 60,
        }).ok();
        
        // HiveFix integration
        self.register(Tool {
            id: "hivefix_status".to_string(),
            name: "HiveFix Status".to_string(),
            description: "Check HiveFix self-healing status".to_string(),
            command: "hivefix status".to_string(),
            requires_privilege: false,
            requires_confirmation: false,
            schema: None,
            tags: vec!["hivefix".to_string(), "diagnostic".to_string()],
            examples: vec!["!@ call hivefix_status".to_string()],
            timeout: 10,
        }).ok();
        
        self.register(Tool {
            id: "hivefix_analyze".to_string(),
            name: "HiveFix Analyze".to_string(),
            description: "Analyze system logs for issues".to_string(),
            command: "hivefix analyze".to_string(),
            requires_privilege: false,
            requires_confirmation: false,
            schema: Some(
                SchemaBuilder::new("HiveFixAnalyze")
                    .string_field("log_path")
                        .default_value(serde_json::json!("/var/log/sentient/system.log"))
                        .and()
                    .build()
            ),
            tags: vec!["hivefix".to_string(), "diagnostic".to_string()],
            examples: vec![
                r#"!@ call hivefix_analyze"#.to_string(),
                r#"!@ call hivefix_analyze {"log_path": "/var/log/boot.log"}"#.to_string(),
            ],
            timeout: 30,
        }).ok();
    }
}

lazy_static! {
    /// Global tool registry
    pub static ref TOOL_REGISTRY: ToolRegistry = ToolRegistry::new();
}

/// Get the global tool registry
pub fn get_tool_registry() -> &'static ToolRegistry {
    &TOOL_REGISTRY
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tool_registration() {
        let registry = ToolRegistry::new();
        
        let tool = Tool {
            id: "test_tool".to_string(),
            name: "Test Tool".to_string(),
            description: "A test tool".to_string(),
            command: "echo test".to_string(),
            requires_privilege: false,
            requires_confirmation: false,
            schema: None,
            tags: vec!["test".to_string()],
            examples: vec![],
            timeout: 5,
        };
        
        assert!(registry.register(tool).is_ok());
        assert!(registry.get("test_tool").is_some());
    }
    
    #[test]
    fn test_dangerous_command_detection() {
        let registry = ToolRegistry::new();
        
        // Should fail - dangerous command without privilege
        let dangerous_tool = Tool {
            id: "dangerous".to_string(),
            name: "Dangerous".to_string(),
            description: "Dangerous tool".to_string(),
            command: "rm -rf /".to_string(),
            requires_privilege: false, // This should fail
            requires_confirmation: false,
            schema: None,
            tags: vec![],
            examples: vec![],
            timeout: 5,
        };
        
        assert!(registry.register(dangerous_tool).is_err());
    }
}