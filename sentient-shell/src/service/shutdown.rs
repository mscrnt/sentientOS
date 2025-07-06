// Graceful shutdown implementation for service manager
use super::{ServiceInfo, ServiceConfig, ServiceStatus};
use super::dependency::DependencyResolver;
use anyhow::{Result, Context};
use std::time::{Duration, Instant};
use std::process::{Command, Child};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;

#[cfg(unix)]
use nix::sys::signal::{self, Signal};
#[cfg(unix)]
use nix::unistd::Pid;

pub struct GracefulShutdown {
    timeout: Duration,
    #[cfg(unix)]
    shutdown_sequence: Vec<Signal>,
}

impl Default for GracefulShutdown {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            #[cfg(unix)]
            shutdown_sequence: vec![
                Signal::SIGTERM,  // First try graceful termination
                Signal::SIGKILL,  // Force kill if needed
            ],
            #[cfg(not(unix))]
            shutdown_sequence: vec![],
        }
    }
}

impl GracefulShutdown {
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            timeout: Duration::from_secs(timeout_secs),
            ..Default::default()
        }
    }

    /// Shutdown a service with grace period
    #[cfg(unix)]
    pub fn shutdown_service(&self, pid: u32, service_name: &str) -> Result<()> {
        log::info!("🔻 [SENTD] Initiating graceful shutdown for {} (PID: {})", service_name, pid);
        
        let start_time = Instant::now();
        let pid = Pid::from_raw(pid as i32);
        
        // Try each signal in sequence
        for signal in &self.shutdown_sequence {
            match signal {
                Signal::SIGTERM => {
                    log::debug!("Sending SIGTERM to {}", service_name);
                    signal::kill(pid, *signal)?;
                    
                    // Wait for process to exit gracefully
                    if self.wait_for_exit(pid, self.timeout)? {
                        log::info!("✅ [SENTD] {} stopped gracefully", service_name);
                        return Ok(());
                    }
                    
                    log::warn!("⚠️  [SENTD] {} did not stop after SIGTERM", service_name);
                }
                Signal::SIGKILL => {
                    log::warn!("⚠️  [SENTD] Force killing {} with SIGKILL", service_name);
                    signal::kill(pid, *signal)?;
                    
                    // Give it a moment to die
                    if self.wait_for_exit(pid, Duration::from_secs(5))? {
                        log::info!("✅ [SENTD] {} force stopped", service_name);
                        return Ok(());
                    }
                }
                _ => {}
            }
        }
        
        anyhow::bail!("Failed to stop service {} after {:?}", service_name, start_time.elapsed())
    }

    #[cfg(not(unix))]
    pub fn shutdown_service(&self, pid: u32, service_name: &str) -> Result<()> {
        log::warn!("Graceful shutdown not implemented for non-Unix systems");
        Ok(())
    }

    /// Wait for a process to exit
    #[cfg(unix)]
    fn wait_for_exit(&self, pid: Pid, timeout: Duration) -> Result<bool> {
        let start = Instant::now();
        
        while start.elapsed() < timeout {
            // Check if process still exists
            match signal::kill(pid, Signal::SIGCONT) {
                Ok(_) => {
                    // Process still running
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(nix::errno::Errno::ESRCH) => {
                    // Process no longer exists
                    return Ok(true);
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Error checking process: {}", e));
                }
            }
        }
        
        Ok(false)
    }

    /// Shutdown all services in dependency order
    pub fn shutdown_all_services(
        services: &HashMap<String, ServiceInfo>,
        configs: &HashMap<String, ServiceConfig>,
        stop_signal: Arc<AtomicBool>,
    ) -> Result<()> {
        log::info!("🔻 [SENTD] Initiating system-wide service shutdown");
        
        // Calculate shutdown order (reverse of startup order)
        let mut shutdown_order = DependencyResolver::calculate_startup_order(configs)?;
        shutdown_order.reverse();
        
        for service_name in shutdown_order {
            if stop_signal.load(Ordering::Relaxed) {
                log::info!("🛑 [SENTD] Shutdown interrupted");
                break;
            }
            
            if let Some(info) = services.get(&service_name) {
                if info.status == ServiceStatus::Running {
                    if let Some(pid) = info.pid {
                        let shutdown = GracefulShutdown::default();
                        if let Err(e) = shutdown.shutdown_service(pid, &service_name) {
                            log::error!("🔴 [SENTD] Failed to stop {}: {}", service_name, e);
                        }
                    }
                }
            }
        }
        
        log::info!("✅ [SENTD] Service shutdown complete");
        Ok(())
    }
}

/// Service stop handler with pre/post hooks
pub struct ServiceStopHandler {
    pre_stop_hooks: Vec<Box<dyn Fn(&str) -> Result<()> + Send + Sync>>,
    post_stop_hooks: Vec<Box<dyn Fn(&str) -> Result<()> + Send + Sync>>,
}

impl ServiceStopHandler {
    pub fn new() -> Self {
        Self {
            pre_stop_hooks: Vec::new(),
            post_stop_hooks: Vec::new(),
        }
    }

    pub fn add_pre_stop_hook<F>(&mut self, hook: F) 
    where
        F: Fn(&str) -> Result<()> + Send + Sync + 'static
    {
        self.pre_stop_hooks.push(Box::new(hook));
    }

    pub fn add_post_stop_hook<F>(&mut self, hook: F)
    where
        F: Fn(&str) -> Result<()> + Send + Sync + 'static
    {
        self.post_stop_hooks.push(Box::new(hook));
    }

    pub fn stop_service(&self, service_name: &str, pid: u32) -> Result<()> {
        // Run pre-stop hooks
        for hook in &self.pre_stop_hooks {
            if let Err(e) = hook(service_name) {
                log::warn!("Pre-stop hook failed for {}: {}", service_name, e);
            }
        }

        // Perform graceful shutdown
        let shutdown = GracefulShutdown::default();
        shutdown.shutdown_service(pid, service_name)?;

        // Run post-stop hooks
        for hook in &self.post_stop_hooks {
            if let Err(e) = hook(service_name) {
                log::warn!("Post-stop hook failed for {}: {}", service_name, e);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graceful_shutdown_creation() {
        let shutdown = GracefulShutdown::new(60);
        assert_eq!(shutdown.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_service_stop_handler() {
        let mut handler = ServiceStopHandler::new();
        let pre_called = Arc::new(AtomicBool::new(false));
        let post_called = Arc::new(AtomicBool::new(false));
        
        let pre_clone = pre_called.clone();
        handler.add_pre_stop_hook(move |_| {
            pre_clone.store(true, Ordering::Relaxed);
            Ok(())
        });
        
        let post_clone = post_called.clone();
        handler.add_post_stop_hook(move |_| {
            post_clone.store(true, Ordering::Relaxed);
            Ok(())
        });
        
        // Would need a real process to test fully
        // This just verifies the structure compiles
    }
}