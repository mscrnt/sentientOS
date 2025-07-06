use super::*;
use anyhow::{Result, Context};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use log::{info, warn, error};

pub struct HealthMonitor {
    checks: Arc<Mutex<HashMap<String, HealthCheck>>>,
    running: Arc<Mutex<bool>>,
}

struct HealthCheck {
    service_name: String,
    config: HealthCheckConfig,
    last_check: Option<Instant>,
    last_result: HealthStatus,
    consecutive_failures: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HealthStatus {
    Unknown,
    Healthy,
    Unhealthy,
    Failed,
}

impl HealthMonitor {
    pub fn new() -> Self {
        Self {
            checks: Arc::new(Mutex::new(HashMap::new())),
            running: Arc::new(Mutex::new(false)),
        }
    }

    pub fn start(&self) {
        let mut running = self.running.lock().unwrap();
        if *running {
            return;
        }
        *running = true;
        drop(running);

        let checks = self.checks.clone();
        let running = self.running.clone();

        thread::spawn(move || {
            info!("💗 [HEALTH] Health monitor started");

            while *running.lock().unwrap() {
                // Run health checks
                let checks_to_run: Vec<(String, HealthCheckConfig)> = {
                    let mut checks_map = checks.lock().unwrap();
                    checks_map.iter_mut()
                        .filter_map(|(name, check)| {
                            let should_run = check.last_check
                                .map(|last| last.elapsed().as_millis() >= check.config.interval_ms as u128)
                                .unwrap_or(true);

                            if should_run {
                                check.last_check = Some(Instant::now());
                                Some((name.clone(), check.config.clone()))
                            } else {
                                None
                            }
                        })
                        .collect()
                };

                for (name, config) in checks_to_run {
                    let result = run_health_check(&name, &config);
                    
                    let mut checks_map = checks.lock().unwrap();
                    if let Some(check) = checks_map.get_mut(&name) {
                        check.last_result = result;
                        
                        match result {
                            HealthStatus::Healthy => {
                                if check.consecutive_failures > 0 {
                                    info!("💚 [HEALTH] {} recovered", name);
                                }
                                check.consecutive_failures = 0;
                            }
                            HealthStatus::Unhealthy | HealthStatus::Failed => {
                                check.consecutive_failures += 1;
                                warn!("💛 [HEALTH] {} unhealthy (failures: {})", 
                                      name, check.consecutive_failures);
                                
                                if check.consecutive_failures >= config.retries {
                                    error!("💔 [HEALTH] {} exceeded failure threshold", name);
                                    // TODO: Trigger service restart or alert
                                }
                            }
                            _ => {}
                        }
                    }
                }

                thread::sleep(Duration::from_secs(1));
            }

            info!("💔 [HEALTH] Health monitor stopped");
        });
    }

    pub fn stop(&self) {
        *self.running.lock().unwrap() = false;
    }

    pub fn add_check(&self, service_name: String, config: HealthCheckConfig) {
        let mut checks = self.checks.lock().unwrap();
        checks.insert(service_name.clone(), HealthCheck {
            service_name,
            config,
            last_check: None,
            last_result: HealthStatus::Unknown,
            consecutive_failures: 0,
        });
    }

    pub fn remove_check(&self, service_name: &str) {
        let mut checks = self.checks.lock().unwrap();
        checks.remove(service_name);
    }

    pub fn get_status(&self, service_name: &str) -> Option<HealthStatus> {
        let checks = self.checks.lock().unwrap();
        checks.get(service_name).map(|c| c.last_result)
    }

    pub fn get_all_statuses(&self) -> HashMap<String, HealthStatus> {
        let checks = self.checks.lock().unwrap();
        checks.iter()
            .map(|(name, check)| (name.clone(), check.last_result))
            .collect()
    }
}

fn run_health_check(service_name: &str, config: &HealthCheckConfig) -> HealthStatus {
    // Parse command
    let parts: Vec<&str> = config.command.split_whitespace().collect();
    if parts.is_empty() {
        return HealthStatus::Failed;
    }

    let mut cmd = Command::new(parts[0]);
    if parts.len() > 1 {
        cmd.args(&parts[1..]);
    }

    // Set timeout
    let timeout = Duration::from_millis(config.timeout_ms);
    let start = Instant::now();

    // Execute health check
    match cmd
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(mut child) => {
            // Wait for completion with timeout
            loop {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        return if status.success() {
                            HealthStatus::Healthy
                        } else {
                            HealthStatus::Unhealthy
                        };
                    }
                    Ok(None) => {
                        if start.elapsed() > timeout {
                            let _ = child.kill();
                            return HealthStatus::Failed;
                        }
                        thread::sleep(Duration::from_millis(100));
                    }
                    Err(_) => return HealthStatus::Failed,
                }
            }
        }
        Err(e) => {
            error!("Failed to run health check for {}: {}", service_name, e);
            HealthStatus::Failed
        }
    }
}