//! Secure tool executor with sandboxing and privilege management

use super::registry::{Tool, get_tool_registry};
use crate::schema::Schema;
use crate::schema::validate::JsonValidator;
use anyhow::{Result, bail, Context};
use serde_json::Value;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use std::io::{BufReader, BufRead};
use std::sync::mpsc;
use std::thread;

/// Execution mode for tools
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionMode {
    /// Normal execution with validation
    Safe,
    /// Requires elevated privileges
    Privileged,
    /// Run in background
    Background,
    /// Run in sandbox environment
    Sandboxed,
}

/// Tool execution result
#[derive(Debug)]
pub struct ExecutionResult {
    /// Exit code (0 for success)
    pub exit_code: i32,
    
    /// Standard output
    pub stdout: String,
    
    /// Standard error
    pub stderr: String,
    
    /// Execution time in milliseconds
    pub duration_ms: u64,
    
    /// Whether execution was interrupted
    pub interrupted: bool,
}

/// Tool executor
pub struct ToolExecutor {
    /// Sandbox directory for isolated execution
    sandbox_dir: Option<String>,
    
    /// Whether to allow privileged operations
    allow_privileged: bool,
    
    /// Whether to require confirmation for dangerous operations
    require_confirmation: bool,
}

impl ToolExecutor {
    /// Create new executor
    pub fn new() -> Self {
        Self {
            sandbox_dir: Some("/tmp/sentient-sandbox".to_string()),
            allow_privileged: false,
            require_confirmation: true,
        }
    }
    
    /// Enable privileged mode
    pub fn with_privileges(mut self) -> Self {
        self.allow_privileged = true;
        self
    }
    
    /// Disable confirmation prompts
    pub fn without_confirmation(mut self) -> Self {
        self.require_confirmation = false;
        self
    }
    
    /// Execute a tool by ID with arguments
    pub fn execute(
        &self,
        tool_id: &str,
        args: Option<Value>,
        mode: ExecutionMode,
    ) -> Result<ExecutionResult> {
        // Get tool from registry
        let registry = get_tool_registry();
        let tool = registry.get(tool_id)
            .ok_or_else(|| anyhow::anyhow!("Tool '{}' not found", tool_id))?;
        
        // Validate execution permissions
        self.validate_permissions(&tool, &mode)?;
        
        // Validate arguments against schema
        if let Some(schema) = &tool.schema {
            if let Some(args_value) = &args {
                let validator = JsonValidator::new(schema.clone());
                validator.validate_value(args_value)
                    .context("Invalid arguments for tool")?;
            } else {
                // Check if schema has required fields
                let has_required = schema.fields.iter().any(|f| f.required);
                if has_required {
                    bail!("Tool requires arguments but none provided");
                }
            }
        }
        
        // Request confirmation if needed
        if self.should_confirm(&tool, &mode) {
            if !self.request_confirmation(&tool, &args)? {
                bail!("Tool execution cancelled by user");
            }
        }
        
        // Build command
        let mut command = self.build_command(&tool, args)?;
        
        // Apply execution mode
        match mode {
            ExecutionMode::Safe => {},
            ExecutionMode::Privileged => {
                // Already validated permissions
            },
            ExecutionMode::Background => {
                // Will be handled by process spawn
            },
            ExecutionMode::Sandboxed => {
                self.apply_sandbox(&mut command)?;
            },
        }
        
        // Execute with timeout
        self.execute_with_timeout(command, tool.timeout, mode == ExecutionMode::Background)
    }
    
    /// Validate execution permissions
    fn validate_permissions(&self, tool: &Tool, mode: &ExecutionMode) -> Result<()> {
        // Check privilege requirements
        if tool.requires_privilege && !self.allow_privileged {
            bail!("Tool '{}' requires elevated privileges", tool.id);
        }
        
        // Validate mode compatibility
        if *mode == ExecutionMode::Privileged && !tool.requires_privilege {
            bail!("Tool '{}' does not support privileged execution", tool.id);
        }
        
        Ok(())
    }
    
    /// Check if confirmation is needed
    fn should_confirm(&self, tool: &Tool, mode: &ExecutionMode) -> bool {
        if !self.require_confirmation {
            return false;
        }
        
        tool.requires_confirmation || 
        tool.requires_privilege ||
        *mode == ExecutionMode::Privileged
    }
    
    /// Request user confirmation
    fn request_confirmation(&self, tool: &Tool, args: &Option<Value>) -> Result<bool> {
        println!("\n⚠️  Confirmation Required");
        println!("Tool: {} ({})", tool.name, tool.id);
        println!("Description: {}", tool.description);
        
        if let Some(args_value) = args {
            println!("Arguments: {}", serde_json::to_string_pretty(args_value)?);
        }
        
        if tool.requires_privilege {
            println!("⚡ This tool requires elevated privileges");
        }
        
        print!("\nProceed? [y/N]: ");
        use std::io::{self, Write};
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        Ok(input.trim().to_lowercase() == "y")
    }
    
    /// Build command from tool and arguments
    fn build_command(&self, tool: &Tool, args: Option<Value>) -> Result<Command> {
        let mut cmd = Command::new("sh");
        cmd.arg("-c");
        
        // Build command string
        let mut cmd_str = tool.command.clone();
        
        // Substitute arguments if provided
        if let Some(args_value) = args {
            cmd_str = self.substitute_args(&cmd_str, &args_value)?;
        }
        
        cmd.arg(cmd_str);
        
        // Set up pipes
        cmd.stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        Ok(cmd)
    }
    
    /// Substitute arguments in command string
    fn substitute_args(&self, command: &str, args: &Value) -> Result<String> {
        let mut result = command.to_string();
        
        // Handle different argument patterns
        if let Value::Object(map) = args {
            // Replace {key} patterns
            for (key, value) in map {
                let pattern = format!("{{{}}}", key);
                let replacement = match value {
                    Value::String(s) => shell_escape(s),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => serde_json::to_string(value)?,
                };
                result = result.replace(&pattern, &replacement);
            }
            
            // Handle special cases
            if command.contains("kill") && map.contains_key("pid") {
                if let Some(Value::Bool(true)) = map.get("force") {
                    result = format!("{} -9 {}", 
                        result.split_whitespace().next().unwrap_or("kill"),
                        map["pid"]);
                } else {
                    result = format!("{} {}", 
                        result.split_whitespace().next().unwrap_or("kill"),
                        map["pid"]);
                }
            }
        }
        
        Ok(result)
    }
    
    /// Apply sandbox restrictions
    fn apply_sandbox(&self, command: &mut Command) -> Result<()> {
        if let Some(sandbox_dir) = &self.sandbox_dir {
            // Create sandbox directory if needed
            std::fs::create_dir_all(sandbox_dir)?;
            
            // Use firejail if available, otherwise basic chroot
            if which::which("firejail").is_ok() {
                let mut firejail = Command::new("firejail");
                firejail.arg("--quiet")
                       .arg("--private")
                       .arg(format!("--private-tmp={}", sandbox_dir))
                       .arg("--net=none")
                       .arg("--no-sound");
                
                // Get original command args
                let orig_args: Vec<String> = command.get_args()
                    .map(|s| s.to_string_lossy().to_string())
                    .collect();
                
                // Replace command with firejail wrapper
                *command = firejail;
                command.arg("sh").arg("-c").arg(orig_args.join(" "));
            } else {
                // Basic isolation using environment
                command.env("HOME", sandbox_dir)
                       .env("TMPDIR", sandbox_dir)
                       .current_dir(sandbox_dir);
            }
        }
        
        Ok(())
    }
    
    /// Execute command with timeout
    fn execute_with_timeout(
        &self,
        mut command: Command,
        timeout_secs: u64,
        background: bool,
    ) -> Result<ExecutionResult> {
        let start = std::time::Instant::now();
        
        if background {
            // Spawn in background
            command.spawn()
                .context("Failed to spawn background process")?;
            
            return Ok(ExecutionResult {
                exit_code: 0,
                stdout: "Process started in background".to_string(),
                stderr: String::new(),
                duration_ms: start.elapsed().as_millis() as u64,
                interrupted: false,
            });
        }
        
        // Spawn process
        let mut child = command.spawn()
            .context("Failed to spawn process")?;
        
        // Set up channels for output collection
        let (tx_out, rx_out) = mpsc::channel();
        let (tx_err, rx_err) = mpsc::channel();
        
        // Collect stdout
        if let Some(stdout) = child.stdout.take() {
            let tx = tx_out.clone();
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        let _ = tx.send(line);
                    }
                }
            });
        }
        
        // Collect stderr
        if let Some(stderr) = child.stderr.take() {
            let tx = tx_err.clone();
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        let _ = tx.send(line);
                    }
                }
            });
        }
        
        // Wait with timeout using a thread
        let timeout = Duration::from_secs(timeout_secs);
        
        let (tx_status, rx_status) = mpsc::channel();
        let child_id = child.id();
        
        // Spawn thread to wait for child
        thread::spawn(move || {
            let status = child.wait();
            let _ = tx_status.send(status);
        });
        
        // Wait for either completion or timeout
        let exit_status = match rx_status.recv_timeout(timeout) {
            Ok(Ok(status)) => status,
            Ok(Err(e)) => bail!("Failed to wait for process: {}", e),
            Err(_) => {
                // Timeout - kill process
                log::warn!("Process exceeded timeout of {}s, terminating", timeout_secs);
                
                // Kill the process using its ID
                #[cfg(unix)]
                {
                    use std::os::unix::process::CommandExt;
                    unsafe {
                        libc::kill(child_id as i32, libc::SIGTERM);
                        thread::sleep(Duration::from_millis(100));
                        libc::kill(child_id as i32, libc::SIGKILL);
                    }
                }
                
                #[cfg(not(unix))]
                {
                    // On non-Unix, we can't easily kill by PID
                    log::error!("Cannot kill process on non-Unix platform");
                }
                
                // Create a failed exit status
                // We'll handle this by returning early with a custom result
                return Ok(ExecutionResult {
                    exit_code: -1,
                    stdout: String::new(),
                    stderr: "Process terminated due to timeout".to_string(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    interrupted: true,
                })
            }
        };
        
        // Collect output
        drop(tx_out);
        drop(tx_err);
        
        let stdout: Vec<String> = rx_out.iter().collect();
        let stderr: Vec<String> = rx_err.iter().collect();
        
        Ok(ExecutionResult {
            exit_code: exit_status.code().unwrap_or(-1),
            stdout: stdout.join("\n"),
            stderr: stderr.join("\n"),
            duration_ms: start.elapsed().as_millis() as u64,
            interrupted: false,
        })
    }
}

/// Shell escape a string for safe command execution
fn shell_escape(s: &str) -> String {
    // Simple escaping - in production use a proper shell escaping library
    if s.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.') {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\"'\"'"))
    }
}

/// Execute a tool with default settings
pub fn execute_tool(tool_id: &str, args: Option<Value>) -> Result<ExecutionResult> {
    let executor = ToolExecutor::new();
    executor.execute(tool_id, args, ExecutionMode::Safe)
}

/// Execute a tool with specific mode
pub fn execute_tool_with_mode(
    tool_id: &str,
    args: Option<Value>,
    mode: ExecutionMode,
) -> Result<ExecutionResult> {
    let mut executor = ToolExecutor::new();
    
    if mode == ExecutionMode::Privileged {
        executor = executor.with_privileges();
    }
    
    executor.execute(tool_id, args, mode)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_shell_escape() {
        assert_eq!(shell_escape("hello"), "hello");
        assert_eq!(shell_escape("hello world"), "'hello world'");
        assert_eq!(shell_escape("it's"), "'it'\"'\"'s'");
    }
    
    #[test]
    fn test_execution_modes() {
        assert_ne!(ExecutionMode::Safe, ExecutionMode::Privileged);
        assert_ne!(ExecutionMode::Background, ExecutionMode::Sandboxed);
    }
}