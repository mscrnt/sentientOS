//! Common type definitions for schema validation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Common validation patterns
pub mod patterns {
    /// Email regex pattern
    pub const EMAIL: &str = r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$";
    
    /// URL pattern
    pub const URL: &str = r"^https?://[^\s/$.?#].[^\s]*$";
    
    /// IPv4 pattern
    pub const IPV4: &str = r"^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$";
    
    /// Alphanumeric only
    pub const ALPHANUMERIC: &str = r"^[a-zA-Z0-9]+$";
    
    /// Shell-safe characters
    pub const SHELL_SAFE: &str = r"^[a-zA-Z0-9_\-\.]+$";
}

/// Command validation schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSchema {
    pub command: String,
    pub args_schema: super::Schema,
    pub requires_confirmation: bool,
    pub max_execution_time: Option<u64>,
}

/// AI prompt validation schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptSchema {
    pub max_length: usize,
    pub forbidden_patterns: Vec<String>,
    pub required_context: Vec<String>,
}

/// Service manifest schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceManifestSchema {
    pub name: String,
    pub version: String,
    pub command: String,
    pub environment: HashMap<String, String>,
    pub dependencies: Vec<String>,
}

/// Package manifest schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifestSchema {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub entry_point: String,
    pub permissions: Vec<String>,
}

/// Create standard schemas
pub fn create_command_arg_schema() -> super::Schema {
    use super::schema::{SchemaBuilder, FieldType};
    use super::constraints::*;
    
    SchemaBuilder::new("CommandArgs")
        .description("Standard command argument validation")
        .string_field("command")
            .min_length(1)
            .pattern(patterns::SHELL_SAFE)
            .and()
        .build()
}

pub fn create_ai_prompt_schema() -> super::Schema {
    use super::schema::{SchemaBuilder, FieldType};
    use super::constraints::*;
    
    SchemaBuilder::new("AIPrompt")
        .description("AI prompt validation schema")
        .string_field("prompt")
            .min_length(1)
            .max_length(4096)
            .and()
        .string_field("model")
            .pattern(patterns::ALPHANUMERIC)
            .and()
        .integer_field("max_tokens")
            .min(1)
            .max(8192)
            .and()
        .build()
}