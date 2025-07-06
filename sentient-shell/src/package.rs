use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};

pub mod core;

#[derive(Debug, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: String,
    pub installed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageRegistry {
    packages: HashMap<String, Package>,
    install_path: PathBuf,
}

impl PackageRegistry {
    pub fn new() -> Self {
        let install_path = if cfg!(target_os = "uefi") {
            PathBuf::from("/shellpkg")
        } else {
            // For testing on regular OS
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("sentient-shell")
                .join("packages")
        };

        Self {
            packages: HashMap::new(),
            install_path,
        }
    }

    pub fn init(&mut self) -> Result<()> {
        // Create package directory if it doesn't exist
        if !cfg!(target_os = "uefi") {
            fs::create_dir_all(&self.install_path)
                .context("Failed to create package directory")?;
        }

        // Register built-in packages
        self.register_builtin_packages();
        
        Ok(())
    }

    fn register_builtin_packages(&mut self) {
        // Core Utils packages
        self.packages.insert("calc".to_string(), Package {
            name: "calc".to_string(),
            version: "1.0.0".to_string(),
            description: "Basic calculator for mathematical operations".to_string(),
            installed: false,
        });
        
        self.packages.insert("todo".to_string(), Package {
            name: "todo".to_string(),
            version: "1.0.0".to_string(),
            description: "AI-powered task manager with priority suggestions".to_string(),
            installed: false,
        });
        
        self.packages.insert("timer".to_string(), Package {
            name: "timer".to_string(),
            version: "1.0.0".to_string(),
            description: "Simple timer and stopwatch utility".to_string(),
            installed: false,
        });
        
        self.packages.insert("scratch".to_string(), Package {
            name: "scratch".to_string(),
            version: "1.0.0".to_string(),
            description: "Quick buffer for text/code snippets".to_string(),
            installed: false,
        });

        // System packages
        self.packages.insert("neofetch".to_string(), Package {
            name: "neofetch".to_string(),
            version: "1.0.0".to_string(),
            description: "Display system information in a pretty way".to_string(),
            installed: false,
        });
        
        self.packages.insert("df".to_string(), Package {
            name: "df".to_string(),
            version: "1.0.0".to_string(),
            description: "Display filesystem usage information".to_string(),
            installed: false,
        });
        
        self.packages.insert("top".to_string(), Package {
            name: "top".to_string(),
            version: "1.0.0".to_string(),
            description: "Display running processes and resource usage".to_string(),
            installed: false,
        });
        
        self.packages.insert("ps".to_string(), Package {
            name: "ps".to_string(),
            version: "1.0.0".to_string(),
            description: "List running processes".to_string(),
            installed: false,
        });

        // Knowledge packages
        self.packages.insert("ask".to_string(), Package {
            name: "ask".to_string(),
            version: "1.0.0".to_string(),
            description: "Natural language interface to Ollama AI".to_string(),
            installed: false,
        });

        // Creative packages
        self.packages.insert("joke".to_string(), Package {
            name: "joke".to_string(),
            version: "1.0.0".to_string(),
            description: "Get a random joke from the AI".to_string(),
            installed: false,
        });

        // HiveFix agent
        self.packages.insert("hivefix".to_string(), Package {
            name: "hivefix".to_string(),
            version: "1.0.0".to_string(),
            description: "Self-healing AI agent that monitors and fixes system errors".to_string(),
            installed: false,
        });
    }

    pub fn list_available(&self) -> Vec<&Package> {
        self.packages.values().filter(|p| !p.installed).collect()
    }

    pub fn list_installed(&self) -> Vec<&Package> {
        self.packages.values().filter(|p| p.installed).collect()
    }

    pub fn install(&mut self, package_name: &str) -> Result<String> {
        let package = self.packages.get_mut(package_name)
            .ok_or_else(|| anyhow::anyhow!("Package '{}' not found", package_name))?;

        if package.installed {
            return Ok(format!("Package '{}' is already installed", package_name));
        }

        // Create package stub file
        let package_file = self.install_path.join(format!("{}.meta", package_name));
        
        if !cfg!(target_os = "uefi") {
            // Only write files when not in UEFI environment
            let meta = serde_json::to_string_pretty(&package)?;
            fs::write(&package_file, meta)
                .context("Failed to write package metadata")?;
        }

        package.installed = true;
        
        Ok(format!("Successfully installed package '{}'", package_name))
    }

    pub fn uninstall(&mut self, package_name: &str) -> Result<String> {
        let package = self.packages.get_mut(package_name)
            .ok_or_else(|| anyhow::anyhow!("Package '{}' not found", package_name))?;

        if !package.installed {
            return Ok(format!("Package '{}' is not installed", package_name));
        }

        // Remove package stub file
        let package_file = self.install_path.join(format!("{}.meta", package_name));
        
        if !cfg!(target_os = "uefi") && package_file.exists() {
            fs::remove_file(&package_file)
                .context("Failed to remove package metadata")?;
        }

        package.installed = false;
        
        Ok(format!("Successfully uninstalled package '{}'", package_name))
    }

    pub fn is_installed(&self, package_name: &str) -> bool {
        self.packages.get(package_name)
            .map(|p| p.installed)
            .unwrap_or(false)
    }

    pub fn search(&self, query: &str) -> Vec<&Package> {
        let query_lower = query.to_lowercase();
        self.packages.values()
            .filter(|p| {
                p.name.to_lowercase().contains(&query_lower) ||
                p.description.to_lowercase().contains(&query_lower)
            })
            .collect()
    }
}

// Built-in package implementations
pub mod builtin {
    use anyhow::Result;

    pub fn calc(expression: &str) -> Result<String> {
        // Simple calculator implementation
        let expr = expression.trim();
        
        // Basic arithmetic parser
        if let Some((left, right)) = expr.split_once('+') {
            let l: f64 = left.trim().parse()?;
            let r: f64 = right.trim().parse()?;
            return Ok(format!("{}", l + r));
        }
        
        if let Some((left, right)) = expr.split_once('-') {
            let l: f64 = left.trim().parse()?;
            let r: f64 = right.trim().parse()?;
            return Ok(format!("{}", l - r));
        }
        
        if let Some((left, right)) = expr.split_once('*') {
            let l: f64 = left.trim().parse()?;
            let r: f64 = right.trim().parse()?;
            return Ok(format!("{}", l * r));
        }
        
        if let Some((left, right)) = expr.split_once('/') {
            let l: f64 = left.trim().parse()?;
            let r: f64 = right.trim().parse()?;
            if r == 0.0 {
                return Err(anyhow::anyhow!("Division by zero"));
            }
            return Ok(format!("{}", l / r));
        }
        
        Err(anyhow::anyhow!("Invalid expression. Use format: number operator number"))
    }

    pub fn neofetch() -> String {
        let os_info = format!(
            r#"
       _____            _   _            _   ____   _____ 
      / ____|          | | (_)          | | / __ \ / ____|
     | (___   ___ _ __ | |_ _  ___ _ __ | || |  | | (___  
      \___ \ / _ \ '_ \| __| |/ _ \ '_ \| || |  | |\___ \ 
      ____) |  __/ | | | |_| |  __/ | | | || |__| |____) |
     |_____/ \___|_| |_|\__|_|\___|_| |_|_| \____/|_____/ 
                                                           
     OS: SentientOS v0.1.0
     Kernel: AI-First Microkernel
     Shell: SentientShell v1.0
     AI Model: deepseek-v2:16b
     Memory: Dynamic AI-managed
     CPU: Quantum-ready
"#
        );
        os_info
    }

    pub async fn joke(ai_client: &mut crate::ai::AiClient) -> Result<String> {
        // Request a joke from the AI
        let prompt = "Tell me a short, funny programming joke. Keep it under 50 words.";
        ai_client.generate_text(prompt)
    }
}