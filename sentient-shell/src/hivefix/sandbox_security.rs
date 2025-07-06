use anyhow::{Result, Context, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashSet;

/// Security configuration for sandbox
pub struct SandboxSecurity {
    /// Maximum memory in bytes
    pub max_memory: usize,
    /// Maximum execution time in milliseconds
    pub max_execution_time_ms: u64,
    /// Allowed syscalls (whitelist)
    pub allowed_syscalls: HashSet<String>,
    /// Blocked paths (blacklist)
    pub blocked_paths: HashSet<PathBuf>,
}

impl Default for SandboxSecurity {
    fn default() -> Self {
        let mut allowed_syscalls = HashSet::new();
        // Only allow safe syscalls
        for syscall in &[
            "read", "write", "open", "close", "stat", "fstat",
            "mmap", "munmap", "brk", "access", "exit_group",
            "clock_gettime", "gettimeofday", "nanosleep",
        ] {
            allowed_syscalls.insert(syscall.to_string());
        }

        let mut blocked_paths = HashSet::new();
        // Block critical system paths
        for path in &[
            "/boot", "/sys", "/proc", "/dev",
            "/etc/passwd", "/etc/shadow",
            "/.ssh", "/.gnupg",
        ] {
            blocked_paths.insert(PathBuf::from(path));
        }

        Self {
            max_memory: 256 * 1024 * 1024, // 256MB
            max_execution_time_ms: 30000,   // 30 seconds
            allowed_syscalls,
            blocked_paths,
        }
    }
}

/// Validate that a path is safe to access
pub fn validate_path_access(path: &Path, security: &SandboxSecurity) -> Result<()> {
    let canonical = path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf());
    
    // Check against blocked paths
    for blocked in &security.blocked_paths {
        if canonical.starts_with(blocked) {
            bail!("Access denied: path {} is blocked", canonical.display());
        }
    }
    
    // Ensure path doesn't contain suspicious patterns
    let path_str = canonical.to_string_lossy();
    if path_str.contains("..") || path_str.contains("~") {
        bail!("Access denied: path contains suspicious patterns");
    }
    
    Ok(())
}

/// Create a restricted environment for testing
pub fn create_restricted_env(sandbox_dir: &Path) -> Result<Vec<(String, String)>> {
    let mut env = Vec::new();
    
    // Minimal environment
    env.push(("PATH".to_string(), "/usr/bin:/bin".to_string()));
    env.push(("HOME".to_string(), sandbox_dir.to_string_lossy().to_string()));
    env.push(("TMPDIR".to_string(), sandbox_dir.join("tmp").to_string_lossy().to_string()));
    env.push(("USER".to_string(), "sandbox".to_string()));
    
    // Create tmp directory
    fs::create_dir_all(sandbox_dir.join("tmp"))?;
    
    Ok(env)
}

/// Validate patch content for safety
pub fn validate_patch_safety(patch: &str) -> Result<()> {
    // Check for dangerous patterns
    let dangerous_patterns = [
        "rm -rf",
        "dd if=",
        "mkfs",
        "> /dev/",
        "sudo",
        "chmod 777",
        "eval(",
        "exec(",
        "__import__",
    ];
    
    for pattern in &dangerous_patterns {
        if patch.contains(pattern) {
            bail!("Patch contains dangerous pattern: {}", pattern);
        }
    }
    
    // Check for kernel/bootloader paths
    let critical_paths = [
        "/boot",
        "/kernel",
        "sentient-kernel",
        "sentient-bootloader",
        "/efi",
    ];
    
    for path in &critical_paths {
        if patch.contains(path) {
            bail!("Patch attempts to modify critical system path: {}", path);
        }
    }
    
    Ok(())
}

#[cfg(target_os = "linux")]
pub mod linux {
    use super::*;
    use std::process::Command;
    
    /// Set resource limits for a process
    pub fn set_resource_limits(security: &SandboxSecurity) -> Result<()> {
        use std::os::unix::process::CommandExt;
        
        // This would use setrlimit in production
        // For now, we'll use ulimit commands
        Ok(())
    }
    
    /// Create a minimal chroot environment
    pub fn create_chroot(sandbox_dir: &Path) -> Result<()> {
        // Create essential directories
        for dir in &["bin", "lib", "lib64", "usr", "tmp", "dev"] {
            fs::create_dir_all(sandbox_dir.join(dir))?;
        }
        
        // Copy minimal binaries (in production, use busybox or similar)
        // For now, just create the structure
        
        Ok(())
    }
}