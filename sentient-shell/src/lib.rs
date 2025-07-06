pub mod ai;
pub mod ai_router;
pub mod commands;
pub mod hivefix;
#[cfg(feature = "local-inference")]
pub mod inference;
pub mod package;
#[cfg(feature = "serial")]
pub mod serial;
pub mod service;
pub mod validated_exec;
pub mod schema;

// Re-export ShellState from main module
pub use crate::shell_state::ShellState;

pub mod shell_state;

pub const SHELL_VERSION: &str = "1.0.0";
pub const BANNER: &str = r#"
╔═══════════════════════════════════════════╗
║      SentientShell v1.0 – AI-Native CLI   ║
║    The Intelligent Interface to SentientOS ║
╚═══════════════════════════════════════════╝
"#;
