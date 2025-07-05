pub mod ai;
pub mod commands;
#[cfg(feature = "local-inference")]
pub mod inference;
#[cfg(feature = "serial")]
pub mod serial;

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
