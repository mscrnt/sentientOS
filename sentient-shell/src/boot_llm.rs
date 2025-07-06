//! Boot LLM integration for offline/recovery mode

use anyhow::{Result, bail};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use lazy_static::lazy_static;

/// Boot model configuration
#[derive(Debug, Clone)]
pub struct BootLLMConfig {
    pub model_path: String,
    pub enabled: bool,
    pub timeout: Duration,
    pub max_tokens: usize,
}

impl Default for BootLLMConfig {
    fn default() -> Self {
        Self {
            model_path: "/boot/phi.Q8_0.gguf".to_string(),
            enabled: true,
            timeout: Duration::from_secs(10),
            max_tokens: 512,
        }
    }
}

/// Boot LLM client
pub struct BootLLM {
    config: BootLLMConfig,
    pub available: bool,
}

impl BootLLM {
    pub fn new() -> Self {
        let config = BootLLMConfig::default();
        let available = Self::check_availability(&config);
        
        if available {
            log::info!("🧠 Boot LLM available: {}", config.model_path);
        } else {
            log::warn!("⚠️  Boot LLM not available, using fallback responses");
        }
        
        Self { config, available }
    }
    
    /// Check if boot model is available
    fn check_availability(config: &BootLLMConfig) -> bool {
        // Check if we can access kernel boot model
        if std::path::Path::new(&config.model_path).exists() {
            return true;
        }
        
        // Check if kernel API is available
        if std::path::Path::new("/sys/kernel/llm/boot").exists() {
            return true;
        }
        
        false
    }
    
    /// Evaluate prompt with boot model
    pub fn evaluate(&self, prompt: &str) -> Result<String> {
        if !self.available {
            return self.fallback_response(prompt);
        }
        
        let start = Instant::now();
        
        // Try kernel API first
        if let Ok(response) = self.kernel_api_evaluate(prompt) {
            return Ok(response);
        }
        
        // Try direct model evaluation (future implementation)
        // For now, use fallback
        self.fallback_response(prompt)
    }
    
    /// Call kernel LLM API
    fn kernel_api_evaluate(&self, prompt: &str) -> Result<String> {
        // In real implementation, this would use syscall or /sys interface
        // For now, simulate the call
        
        if prompt.contains("!@") || prompt.contains("!~") {
            // Validated/sandboxed mode
            Ok(format!("Boot LLM: Processing safe command: {}", prompt.trim_start_matches(|c| c == '!' || c == '@' || c == '~')))
        } else {
            bail!("Kernel API not available")
        }
    }
    
    /// Provide fallback responses when model not available
    fn fallback_response(&self, prompt: &str) -> Result<String> {
        let prompt_lower = prompt.to_lowercase();
        
        let response = match prompt_lower.as_str() {
            s if s.contains("help") => {
                "Boot LLM Fallback Mode\n\
                Available commands:\n\
                - status: Show system status\n\
                - safe-mode: Enter safe mode\n\
                - recovery: Start recovery\n\
                - validate <cmd>: Validate command safety"
            }
            s if s.contains("status") => {
                "System Status: Boot mode\n\
                Model: Offline/Fallback\n\
                Available: Limited command set\n\
                Network: Not required"
            }
            s if s.contains("validate") => {
                if prompt.contains("rm") || prompt.contains("format") {
                    "⚠️  DANGEROUS: This command could damage the system"
                } else {
                    "✓ Command appears safe to execute"
                }
            }
            s if s.contains("safe-mode") => {
                "Entering safe mode...\n\
                - Network disabled\n\
                - Limited command set\n\
                - Boot LLM only"
            }
            _ => "Boot LLM: Command not recognized. Type 'help' for available commands."
        };
        
        Ok(response.to_string())
    }
    
    /// Check if main LLM is online
    pub fn is_main_llm_online(&self) -> bool {
        // Check if Ollama or main AI service is reachable
        if let Ok(response) = reqwest::blocking::Client::new()
            .get("http://localhost:11434/api/tags")
            .timeout(Duration::from_secs(2))
            .send() 
        {
            response.status().is_success()
        } else {
            false
        }
    }
}

lazy_static! {
    pub static ref BOOT_LLM: Mutex<BootLLM> = Mutex::new(BootLLM::new());
}

/// Get boot LLM response with automatic fallback
pub fn get_boot_llm_response(prompt: &str) -> Result<String> {
    BOOT_LLM.lock().unwrap().evaluate(prompt)
}

/// Check if we should use boot LLM
pub fn should_use_boot_llm() -> bool {
    let boot_llm = BOOT_LLM.lock().unwrap();
    !boot_llm.is_main_llm_online() || std::env::var("SENTIENT_BOOT_MODE").is_ok()
}