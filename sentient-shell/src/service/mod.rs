// Service Manager for SentientOS (sentd)
// Lightweight process manager with AI-first design

pub mod manager;
pub mod manifest;
pub mod process;
pub mod health;
pub mod api;
pub mod dependency;
pub mod shutdown;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub status: ServiceStatus,
    pub pid: Option<u32>,
    pub started_at: Option<SystemTime>,
    pub restart_count: u32,
    pub last_exit_code: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ServiceStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
    Restarting,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RestartPolicy {
    Never,
    OnFailure,
    Always,
    UnlessStopped,
}

impl std::fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceStatus::Stopped => write!(f, "⚫ stopped"),
            ServiceStatus::Starting => write!(f, "🟡 starting"),
            ServiceStatus::Running => write!(f, "🟢 running"),
            ServiceStatus::Stopping => write!(f, "🟡 stopping"),
            ServiceStatus::Failed => write!(f, "🔴 failed"),
            ServiceStatus::Restarting => write!(f, "🟡 restarting"),
        }
    }
}

// Service configuration from TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub service: ServiceDefinition,
    #[serde(default)]
    pub environment: Vec<(String, String)>,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDefinition {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default = "default_autostart")]
    pub autostart: bool,
    #[serde(default = "default_restart_policy")]
    pub restart: RestartPolicy,
    #[serde(default = "default_restart_delay")]
    pub restart_delay_ms: u64,
    #[serde(default)]
    pub working_directory: Option<String>,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub health_check: Option<HealthCheckConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub command: String,
    pub interval_ms: u64,
    pub timeout_ms: u64,
    pub retries: u32,
}

fn default_autostart() -> bool {
    false
}

fn default_restart_policy() -> RestartPolicy {
    RestartPolicy::OnFailure
}

fn default_restart_delay() -> u64 {
    5000 // 5 seconds
}