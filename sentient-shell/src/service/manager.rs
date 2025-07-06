use super::*;
use super::manifest::ManifestLoader;
use super::process::ProcessManager;
use super::health::HealthMonitor;
use super::dependency::DependencyResolver;
use super::shutdown::GracefulShutdown;
use anyhow::{Result, Context};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use log::{info, warn, error};

pub struct ServiceManager {
    manifest_loader: Arc<ManifestLoader>,
    process_manager: Arc<ProcessManager>,
    pub health_monitor: Arc<HealthMonitor>,
    logs: Arc<Mutex<HashMap<String, Vec<String>>>>,
}

impl ServiceManager {
    pub fn new() -> Self {
        let manager = Self {
            manifest_loader: Arc::new(ManifestLoader::new()),
            process_manager: Arc::new(ProcessManager::new()),
            health_monitor: Arc::new(HealthMonitor::new()),
            logs: Arc::new(Mutex::new(HashMap::new())),
        };

        // Start health monitor
        manager.health_monitor.start();

        manager
    }

    pub fn init(&self) -> Result<()> {
        info!("🚀 [SENTD] Initializing service manager");

        // Create default service manifests if they don't exist
        self.manifest_loader.create_default_services()?;

        // Load and start autostart services
        self.autostart_services()?;

        Ok(())
    }

    fn autostart_services(&self) -> Result<()> {
        // Load all service configurations
        let service_names = self.manifest_loader.list_services()?;
        let mut configs = HashMap::new();
        let mut autostart_services = Vec::new();

        for service_name in service_names {
            match self.manifest_loader.load_service(&service_name) {
                Ok(config) => {
                    if config.service.autostart {
                        autostart_services.push(service_name.clone());
                    }
                    configs.insert(service_name, config);
                }
                Err(e) => {
                    error!("🔴 [SENTD] Failed to load service {}: {}", service_name, e);
                }
            }
        }

        // Calculate startup order
        match DependencyResolver::calculate_startup_order(&configs) {
            Ok(startup_order) => {
                // Start services in dependency order
                for service_name in startup_order {
                    if autostart_services.contains(&service_name) {
                        info!("🚀 [SENTD] Autostarting service: {}", service_name);
                        
                        if let Err(e) = self.start_service(&service_name) {
                            error!("🔴 [SENTD] Failed to autostart {}: {}", service_name, e);
                        }
                    }
                }
            }
            Err(e) => {
                error!("🔴 [SENTD] Failed to calculate startup order: {}", e);
                // Fall back to starting without order
                for service_name in autostart_services {
                    if let Err(e) = self.start_service(&service_name) {
                        error!("🔴 [SENTD] Failed to autostart {}: {}", service_name, e);
                    }
                }
            }
        }

        Ok(())
    }

    fn check_dependencies(&self, config: &ServiceConfig) -> bool {
        let services = self.process_manager.list_services()
            .into_iter()
            .map(|s| (s.name.clone(), s))
            .collect::<HashMap<_, _>>();
        
        match DependencyResolver::check_dependencies(&services, config) {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to check dependencies: {}", e);
                false
            }
        }
    }

    pub fn start_service(&self, name: &str) -> Result<()> {
        let config = self.manifest_loader.load_service(name)?;

        // Add health check if configured
        if let Some(health_config) = &config.service.health_check {
            self.health_monitor.add_check(name.to_string(), health_config.clone());
        }

        // Start the service
        self.process_manager.start_service(config)?;

        // Log the start event
        self.add_log(name, format!("Service started"));

        Ok(())
    }

    pub fn stop_service(&self, name: &str) -> Result<()> {
        // Remove health check
        self.health_monitor.remove_check(name);

        // Get service info for graceful shutdown
        if let Some(info) = self.process_manager.get_service_info(name) {
            if let Some(pid) = info.pid {
                // Use graceful shutdown
                let shutdown = GracefulShutdown::default();
                shutdown.shutdown_service(pid, name)?;
            }
        }

        // Stop the service (cleanup internal state)
        self.process_manager.stop_service(name)?;

        // Log the stop event
        self.add_log(name, format!("Service stopped"));

        Ok(())
    }

    pub fn restart_service(&self, name: &str) -> Result<()> {
        self.add_log(name, format!("Service restart requested"));
        self.process_manager.restart_service(name)?;
        Ok(())
    }

    pub fn get_service_status(&self, name: &str) -> Result<ServiceInfo> {
        self.process_manager.get_service_info(name)
            .ok_or_else(|| anyhow::anyhow!("Service {} not found", name))
    }

    pub fn list_services(&self) -> Result<Vec<ServiceInfo>> {
        let running_services = self.process_manager.list_services();
        let all_services = self.manifest_loader.list_services()?;

        // Combine running and configured services
        let mut services: HashMap<String, ServiceInfo> = running_services
            .into_iter()
            .map(|s| (s.name.clone(), s))
            .collect();

        // Add configured but not running services
        for name in all_services {
            if !services.contains_key(&name) {
                services.insert(name.clone(), ServiceInfo {
                    name,
                    status: ServiceStatus::Stopped,
                    pid: None,
                    started_at: None,
                    restart_count: 0,
                    last_exit_code: None,
                });
            }
        }

        let mut service_list: Vec<_> = services.into_values().collect();
        service_list.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(service_list)
    }

    pub fn get_service_logs(&self, name: &str, lines: usize) -> Vec<String> {
        let logs = self.logs.lock().unwrap();
        logs.get(name)
            .map(|log_vec| {
                log_vec.iter()
                    .rev()
                    .take(lines)
                    .rev()
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    fn add_log(&self, service: &str, message: String) {
        let mut logs = self.logs.lock().unwrap();
        let log_entry = format!("[{}] {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), message);
        
        logs.entry(service.to_string())
            .or_insert_with(Vec::new)
            .push(log_entry);

        // Keep only last 1000 lines per service
        if let Some(log_vec) = logs.get_mut(service) {
            if log_vec.len() > 1000 {
                log_vec.drain(0..log_vec.len() - 1000);
            }
        }
    }
}

// Global service manager instance
lazy_static::lazy_static! {
    static ref SERVICE_MANAGER: ServiceManager = ServiceManager::new();
}

pub fn get_service_manager() -> &'static ServiceManager {
    &SERVICE_MANAGER
}