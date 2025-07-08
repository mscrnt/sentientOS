use super::*;
use anyhow::{Result, Context};
use std::fs;
use std::path::{Path, PathBuf};

pub struct ManifestLoader {
    config_dir: PathBuf,
}

impl ManifestLoader {
    pub fn new() -> Self {
        let config_dir = if cfg!(target_os = "uefi") {
            PathBuf::from("/etc/sentient/services")
        } else {
            // For testing on regular OS
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("sentient")
                .join("services")
        };

        Self { config_dir }
    }

    pub fn ensure_config_dir(&self) -> Result<()> {
        if !self.config_dir.exists() {
            fs::create_dir_all(&self.config_dir)
                .context("Failed to create service config directory")?;
        }
        Ok(())
    }

    pub fn load_service(&self, name: &str) -> Result<ServiceConfig> {
        let manifest_path = self.config_dir.join(format!("{}.toml", name));
        
        if !manifest_path.exists() {
            anyhow::bail!("Service manifest not found: {}", name);
        }

        let content = fs::read_to_string(&manifest_path)
            .context("Failed to read service manifest")?;

        let config: ServiceConfig = toml::from_str(&content)
            .context("Failed to parse service manifest")?;

        // Validate the configuration
        self.validate_config(&config)?;

        Ok(config)
    }

    pub fn list_services(&self) -> Result<Vec<String>> {
        let mut services = Vec::new();

        if self.config_dir.exists() {
            for entry in fs::read_dir(&self.config_dir)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        services.push(name.to_string());
                    }
                }
            }
        }

        services.sort();
        Ok(services)
    }

    pub fn save_service(&self, config: &ServiceConfig) -> Result<()> {
        self.ensure_config_dir()?;

        let manifest_path = self.config_dir.join(format!("{}.toml", config.service.name));
        let content = toml::to_string_pretty(config)
            .context("Failed to serialize service config")?;

        fs::write(&manifest_path, content)
            .context("Failed to write service manifest")?;

        Ok(())
    }

    pub fn remove_service(&self, name: &str) -> Result<()> {
        let manifest_path = self.config_dir.join(format!("{}.toml", name));
        
        if manifest_path.exists() {
            fs::remove_file(&manifest_path)
                .context("Failed to remove service manifest")?;
        }

        Ok(())
    }

    fn validate_config(&self, config: &ServiceConfig) -> Result<()> {
        // Validate service name
        if config.service.name.is_empty() {
            anyhow::bail!("Service name cannot be empty");
        }

        // Validate command
        if config.service.command.is_empty() {
            anyhow::bail!("Service command cannot be empty");
        }

        // Check if command exists (basic validation)
        let command_path = Path::new(&config.service.command);
        if command_path.is_absolute() && !command_path.exists() {
            log::warn!("Service command not found: {}", config.service.command);
        }

        // Validate dependencies
        for dep in &config.dependencies {
            if dep == &config.service.name {
                anyhow::bail!("Service cannot depend on itself");
            }
        }

        Ok(())
    }
}

// Default service manifests for core services
impl ManifestLoader {
    pub fn create_default_services(&self) -> Result<()> {
        self.ensure_config_dir()?;

        // Create HiveFix service manifest if it doesn't exist
        let hivefix_path = self.config_dir.join("hivefix.toml");
        if !hivefix_path.exists() {
            let hivefix_config = ServiceConfig {
            service: ServiceDefinition {
                name: "hivefix".to_string(),
                command: "./target/debug/sentient-shell".to_string(),
                args: vec!["--run-package".to_string(), "hivefix".to_string(), "enable".to_string()],
                autostart: false,
                restart: RestartPolicy::Always,
                restart_delay_ms: 10000,
                working_directory: None,
                user: None,
                health_check: Some(HealthCheckConfig {
                    command: "hivefix status".to_string(),
                    interval_ms: 30000,
                    timeout_ms: 5000,
                    retries: 3,
                }),
            },
            environment: vec![
                ("HIVEFIX_LOG_LEVEL".to_string(), "info".to_string()),
            ],
            dependencies: vec![],
            };

            self.save_service(&hivefix_config)?;
        }

        // Create AI router service manifest if it doesn't exist
        let ai_router_path = self.config_dir.join("ai-router.toml");
        if !ai_router_path.exists() {
            let ai_router_config = ServiceConfig {
            service: ServiceDefinition {
                name: "ai-router".to_string(),
                command: "ai-router".to_string(),
                args: vec![],
                autostart: false,
                restart: RestartPolicy::OnFailure,
                restart_delay_ms: 5000,
                working_directory: None,
                user: None,
                health_check: None,
            },
            environment: vec![
                ("OLLAMA_URL".to_string(), "http://192.168.69.197:11434".to_string()),
                ("SD_URL".to_string(), "http://192.168.69.197:7860".to_string()),
            ],
            dependencies: vec![],
            };

            self.save_service(&ai_router_config)?;
        }
        
        // Create activity loop service manifest
        let activity_loop_path = self.config_dir.join("activity-loop.toml");
        if !activity_loop_path.exists() {
            let activity_loop_config = ServiceConfig {
                service: ServiceDefinition {
                    name: "activity-loop".to_string(),
                    command: "sentient-shell".to_string(),
                    args: vec!["service".to_string(), "run".to_string(), "activity-loop".to_string()],
                    autostart: true,
                    restart: RestartPolicy::Always,
                    restart_delay_ms: 5000,
                    working_directory: None,
                    user: None,
                    health_check: Some(HealthCheckConfig {
                        command: "pgrep -f activity-loop".to_string(),
                        interval_ms: 30000,
                        timeout_ms: 5000,
                        retries: 3,
                    }),
                },
                environment: vec![
                    ("GOAL_INTERVAL_MS".to_string(), "5000".to_string()),
                    ("HEARTBEAT_INTERVAL_MS".to_string(), "60000".to_string()),
                    ("RUST_LOG".to_string(), "info".to_string()),
                ],
                dependencies: vec![],
            };

            self.save_service(&activity_loop_config)?;
        }
        
        // Create LLM observer service manifest
        let llm_observer_path = self.config_dir.join("llm-observer.toml");
        if !llm_observer_path.exists() {
            let llm_observer_config = ServiceConfig {
                service: ServiceDefinition {
                    name: "llm-observer".to_string(),
                    command: "sentient-shell".to_string(),
                    args: vec!["service".to_string(), "run".to_string(), "llm-observer".to_string()],
                    autostart: true,
                    restart: RestartPolicy::OnFailure,
                    restart_delay_ms: 10000,
                    working_directory: None,
                    user: None,
                    health_check: None,
                },
                environment: vec![
                    ("OLLAMA_URL".to_string(), "http://192.168.69.197:11434".to_string()),
                    ("LLM_INTERVAL_MS".to_string(), "30000".to_string()),
                ],
                dependencies: vec!["activity-loop".to_string()],
            };

            self.save_service(&llm_observer_config)?;
        }

        Ok(())
    }
}