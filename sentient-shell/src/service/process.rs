use super::*;
use anyhow::{Result, Context};
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use log::{info, warn, error};

pub struct ProcessManager {
    processes: Arc<Mutex<HashMap<String, ManagedProcess>>>,
    configs: Arc<Mutex<HashMap<String, ServiceConfig>>>,
}

struct ManagedProcess {
    name: String,
    process: Option<Child>,
    status: ServiceStatus,
    started_at: Option<SystemTime>,
    restart_count: u32,
    last_exit_code: Option<i32>,
    restart_policy: RestartPolicy,
    restart_delay_ms: u64,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            configs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn start_service(&self, config: ServiceConfig) -> Result<()> {
        let name = config.service.name.clone();
        
        // Check if already running
        {
            let processes = self.processes.lock().unwrap();
            if let Some(proc) = processes.get(&name) {
                if matches!(proc.status, ServiceStatus::Running | ServiceStatus::Starting) {
                    anyhow::bail!("Service {} is already running", name);
                }
            }
        }

        // Store config
        self.configs.lock().unwrap().insert(name.clone(), config.clone());

        // Start the process
        self.spawn_process(config)?;

        Ok(())
    }

    fn spawn_process(&self, config: ServiceConfig) -> Result<()> {
        let name = config.service.name.clone();
        
        info!("🟢 [SERV] Starting service: {}", name);

        // Build command
        let mut cmd = Command::new(&config.service.command);
        cmd.args(&config.service.args);

        // Set working directory
        if let Some(dir) = &config.service.working_directory {
            cmd.current_dir(dir);
        }

        // Set environment variables
        for (key, value) in &config.environment {
            cmd.env(key, value);
        }

        // Configure stdio
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Spawn process
        match cmd.spawn() {
            Ok(child) => {
                let pid = child.id();
                
                let mut processes = self.processes.lock().unwrap();
                processes.insert(name.clone(), ManagedProcess {
                    name: name.clone(),
                    process: Some(child),
                    status: ServiceStatus::Running,
                    started_at: Some(SystemTime::now()),
                    restart_count: 0,
                    last_exit_code: None,
                    restart_policy: config.service.restart,
                    restart_delay_ms: config.service.restart_delay_ms,
                });

                info!("🟢 [SERV] {} started (PID: {})", name, pid);

                // Start monitoring thread
                self.start_monitor_thread(name);

                Ok(())
            }
            Err(e) => {
                error!("🔴 [SERV] Failed to start {}: {}", name, e);
                
                let mut processes = self.processes.lock().unwrap();
                processes.insert(name.clone(), ManagedProcess {
                    name: name.clone(),
                    process: None,
                    status: ServiceStatus::Failed,
                    started_at: None,
                    restart_count: 0,
                    last_exit_code: None,
                    restart_policy: config.service.restart,
                    restart_delay_ms: config.service.restart_delay_ms,
                });

                Err(e.into())
            }
        }
    }

    fn start_monitor_thread(&self, name: String) {
        let processes = self.processes.clone();
        let configs = self.configs.clone();
        let manager = self.clone();

        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(1));

                let should_restart = {
                    let mut procs = processes.lock().unwrap();
                    
                    if let Some(managed) = procs.get_mut(&name) {
                        if let Some(mut child) = managed.process.take() {
                            match child.try_wait() {
                                Ok(Some(status)) => {
                                    // Process exited
                                    let exit_code = status.code();
                                    info!("🟡 [SERV] {} exited with code: {:?}", name, exit_code);
                                    
                                    managed.status = ServiceStatus::Stopped;
                                    managed.last_exit_code = exit_code;
                                    
                                    // Check restart policy
                                    match managed.restart_policy {
                                        RestartPolicy::Never => false,
                                        RestartPolicy::Always => true,
                                        RestartPolicy::OnFailure => {
                                            exit_code.map(|c| c != 0).unwrap_or(true)
                                        }
                                        RestartPolicy::UnlessStopped => {
                                            // Only restart if not manually stopped
                                            managed.status != ServiceStatus::Stopped
                                        }
                                    }
                                }
                                Ok(None) => {
                                    // Still running
                                    managed.process = Some(child);
                                    false
                                }
                                Err(e) => {
                                    error!("🔴 [SERV] Error checking {} status: {}", name, e);
                                    managed.status = ServiceStatus::Failed;
                                    false
                                }
                            }
                        } else {
                            false
                        }
                    } else {
                        return; // Service removed, exit monitor
                    }
                };

                if should_restart {
                    // Get restart delay
                    let delay_ms = {
                        let procs = processes.lock().unwrap();
                        procs.get(&name).map(|p| p.restart_delay_ms).unwrap_or(5000)
                    };

                    info!("🟡 [SERV] Restarting {} in {}ms", name, delay_ms);
                    thread::sleep(Duration::from_millis(delay_ms));

                    // Increment restart count
                    {
                        let mut procs = processes.lock().unwrap();
                        if let Some(managed) = procs.get_mut(&name) {
                            managed.restart_count += 1;
                            managed.status = ServiceStatus::Restarting;
                        }
                    }

                    // Restart service
                    if let Some(config) = configs.lock().unwrap().get(&name).cloned() {
                        if let Err(e) = manager.spawn_process(config) {
                            error!("🔴 [SERV] Failed to restart {}: {}", name, e);
                        }
                    }
                }
            }
        });
    }

    pub fn stop_service(&self, name: &str) -> Result<()> {
        let mut processes = self.processes.lock().unwrap();
        
        if let Some(managed) = processes.get_mut(name) {
            if let Some(mut child) = managed.process.take() {
                info!("🟡 [SERV] Stopping service: {}", name);
                managed.status = ServiceStatus::Stopping;
                
                // Try graceful shutdown first
                #[cfg(unix)]
                {
                    use nix::sys::signal::{self, Signal};
                    use nix::unistd::Pid;
                    
                    let pid = Pid::from_raw(child.id() as i32);
                    let _ = signal::kill(pid, Signal::SIGTERM);
                }
                
                // Give it time to shutdown
                thread::sleep(Duration::from_secs(5));
                
                // Force kill if still running
                match child.try_wait() {
                    Ok(None) => {
                        warn!("🟡 [SERV] Force killing {}", name);
                        child.kill()?;
                        child.wait()?;
                    }
                    _ => {}
                }
                
                managed.status = ServiceStatus::Stopped;
                info!("⚫ [SERV] {} stopped", name);
            }
            
            Ok(())
        } else {
            anyhow::bail!("Service {} not found", name)
        }
    }

    pub fn restart_service(&self, name: &str) -> Result<()> {
        self.stop_service(name)?;
        
        // Get config and restart
        if let Some(config) = self.configs.lock().unwrap().get(name).cloned() {
            thread::sleep(Duration::from_secs(1));
            self.start_service(config)?;
        }
        
        Ok(())
    }

    pub fn get_service_info(&self, name: &str) -> Option<ServiceInfo> {
        let processes = self.processes.lock().unwrap();
        
        processes.get(name).map(|managed| ServiceInfo {
            name: managed.name.clone(),
            status: managed.status,
            pid: managed.process.as_ref().map(|p| p.id()),
            started_at: managed.started_at,
            restart_count: managed.restart_count,
            last_exit_code: managed.last_exit_code,
        })
    }

    pub fn list_services(&self) -> Vec<ServiceInfo> {
        let processes = self.processes.lock().unwrap();
        
        processes.values().map(|managed| ServiceInfo {
            name: managed.name.clone(),
            status: managed.status,
            pid: managed.process.as_ref().map(|p| p.id()),
            started_at: managed.started_at,
            restart_count: managed.restart_count,
            last_exit_code: managed.last_exit_code,
        }).collect()
    }
}

impl Clone for ProcessManager {
    fn clone(&self) -> Self {
        Self {
            processes: self.processes.clone(),
            configs: self.configs.clone(),
        }
    }
}