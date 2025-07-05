pub mod ai;
pub mod commands;
#[cfg(feature = "serial")]
pub mod serial;
#[cfg(feature = "local-inference")]
pub mod inference;

pub use commands::ShellState;

pub const BANNER: &str = r#"
╔═══════════════════════════════════════════╗
║      SentientShell v1.0 – AI-Native CLI   ║
║    The Intelligent Interface to SentientOS ║
╚═══════════════════════════════════════════╝
"#;