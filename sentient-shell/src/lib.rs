pub mod ai;
pub mod ai_router;
pub mod commands_functions;
pub mod commands {
    pub mod rag_tool;
    pub mod rl_trace;
    pub mod rl_infer;
    pub mod rl_retrain;
    pub mod sentient_goal;
}
pub mod hivefix;
#[cfg(feature = "local-inference")]
pub mod inference;
pub mod package;
#[cfg(feature = "serial")]
pub mod serial;
pub mod service;
pub mod services;
pub mod validated_exec;
pub mod web_ui;
pub mod schema;
pub mod boot_llm;
pub mod rag;

// Re-export ShellState from main module
pub use crate::shell_state::ShellState;

pub mod shell_state;

// Tool use framework
pub mod tools;

// LLM function parsing
pub mod llm;

// Shell integration
pub mod shell;

// RAG-Tool fusion framework
pub mod rag_tool_fusion;

// Python bindings for RL
pub mod bindings;

// Test modules
#[cfg(test)]
pub mod tests;

pub const SHELL_VERSION: &str = "1.0.0";
pub const BANNER: &str = r#"
╔═══════════════════════════════════════════╗
║      SentientShell v1.0 – AI-Native CLI   ║
║    The Intelligent Interface to SentientOS ║
╚═══════════════════════════════════════════╝
"#;
