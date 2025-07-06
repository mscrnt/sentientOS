// HiveFix - Self-healing AI agent for SentientOS
// Monitors system logs, detects errors, and proposes fixes using local LLM

pub mod agent;
pub mod sandbox;
pub mod sandbox_security;
pub mod patch;
pub mod communicator;
pub mod api;
pub mod audit;
pub mod error_injector;
pub mod rollback;
pub mod prompts;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub id: String,
    pub timestamp: SystemTime,
    pub source: ErrorSource,
    pub message: String,
    pub stack_trace: Option<String>,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorSource {
    Shell,
    Package(String),
    Kernel,
    System,
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixCandidate {
    pub id: String,
    pub error_id: String,
    pub description: String,
    pub patch: String,
    pub confidence: f32,
    pub tested: bool,
    pub test_result: Option<TestResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub success: bool,
    pub output: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HiveFixStatus {
    Idle,
    Scanning,
    Analyzing(String),
    Testing(String),
    WaitingApproval(String),
    Applying(String),
}

#[derive(Clone)]

// Global configuration
pub struct HiveFixConfig {
    pub enabled: bool,
    pub auto_fix: bool,
    pub sandbox_timeout_ms: u64,
    pub ollama_url: String,
    pub hive_server_url: Option<String>,
    pub log_paths: Vec<String>,
}

impl Default for HiveFixConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_fix: false,
            sandbox_timeout_ms: 30000,
            ollama_url: "http://192.168.69.197:11434".to_string(),
            hive_server_url: None,
            log_paths: vec![
                "/var/log/sentient/".to_string(),
                "/tmp/sentient-errors.log".to_string(),
            ],
        }
    }
}