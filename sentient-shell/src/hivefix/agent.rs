use super::*;
use super::audit::{AuditLogger, AuditEventType};
use super::prompts::PromptTemplates;
use super::rollback::RollbackManager;
use super::sandbox_security::validate_patch_safety;
use crate::ai::AiClient;
use anyhow::{Result, Context};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::path::PathBuf;
use log::{info, warn, error, debug};

pub struct HiveFixAgent {
    config: HiveFixConfig,
    ai_client: AiClient,
    status: Arc<Mutex<HiveFixStatus>>,
    pub error_history: Arc<Mutex<Vec<ErrorEvent>>>,
    fix_candidates: Arc<Mutex<HashMap<String, FixCandidate>>>,
    running: Arc<Mutex<bool>>,
    audit_logger: Arc<AuditLogger>,
    rollback_manager: Arc<RollbackManager>,
}

impl HiveFixAgent {
    pub fn new(config: HiveFixConfig) -> Self {
        let ai_client = AiClient::new(
            config.ollama_url.clone(),
            String::new(), // SD not needed for hivefix
        );

        let audit_logger = Arc::new(
            AuditLogger::new(PathBuf::from("/var/log/hivefix.log"))
                .expect("Failed to create audit logger")
        );
        
        let rollback_manager = Arc::new(
            RollbackManager::new(PathBuf::from("/var/lib/hivefix/snapshots"))
                .expect("Failed to create rollback manager")
        );

        Self {
            config,
            ai_client,
            status: Arc::new(Mutex::new(HiveFixStatus::Idle)),
            error_history: Arc::new(Mutex::new(Vec::new())),
            fix_candidates: Arc::new(Mutex::new(HashMap::new())),
            running: Arc::new(Mutex::new(false)),
            audit_logger,
            rollback_manager,
        }
    }

    pub fn start(&mut self) -> Result<()> {
        let mut running = self.running.lock().unwrap();
        if *running {
            return Ok(());
        }
        *running = true;
        drop(running);

        let status = self.status.clone();
        let error_history = self.error_history.clone();
        let fix_candidates = self.fix_candidates.clone();
        let running = self.running.clone();
        let config = self.config.clone();
        let mut ai_client = self.ai_client.clone();

        thread::spawn(move || {
            info!("HiveFix agent started");
            
            while *running.lock().unwrap() {
                // Update status
                *status.lock().unwrap() = HiveFixStatus::Scanning;
                
                // Scan for errors
                match scan_system_logs(&config.log_paths) {
                    Ok(new_errors) => {
                        if !new_errors.is_empty() {
                            info!("Found {} new errors", new_errors.len());
                            
                            // Add to history
                            let mut history = error_history.lock().unwrap();
                            for error in new_errors {
                                let error_id = error.id.clone();
                                history.push(error.clone());
                                
                                // Analyze error
                                *status.lock().unwrap() = HiveFixStatus::Analyzing(error_id.clone());
                                
                                match analyze_error(&mut ai_client, &error) {
                                    Ok(fix) => {
                                        info!("Generated fix candidate for error {}", error_id);
                                        fix_candidates.lock().unwrap().insert(error_id.clone(), fix);
                                        
                                        // Test in sandbox if enabled
                                        if config.auto_fix {
                                            *status.lock().unwrap() = HiveFixStatus::Testing(error_id.clone());
                                            // Sandbox testing would go here
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Failed to analyze error {}: {}", error_id, e);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to scan logs: {}", e);
                    }
                }
                
                *status.lock().unwrap() = HiveFixStatus::Idle;
                thread::sleep(Duration::from_secs(30));
            }
            
            info!("HiveFix agent stopped");
        });

        Ok(())
    }

    pub fn stop(&mut self) {
        let mut running = self.running.lock().unwrap();
        *running = false;
    }

    pub fn get_status(&self) -> HiveFixStatus {
        self.status.lock().unwrap().clone()
    }

    pub fn get_error_history(&self) -> Vec<ErrorEvent> {
        self.error_history.lock().unwrap().clone()
    }

    pub fn get_fix_candidates(&self) -> HashMap<String, FixCandidate> {
        self.fix_candidates.lock().unwrap().clone()
    }

    pub fn apply_fix(&mut self, fix_id: &str) -> Result<()> {
        let candidates = self.fix_candidates.lock().unwrap();
        let fix = candidates.get(fix_id)
            .ok_or_else(|| anyhow::anyhow!("Fix candidate not found"))?;
        
        *self.status.lock().unwrap() = HiveFixStatus::Applying(fix_id.to_string());
        
        // Apply the patch
        match patch::apply_patch(&fix.patch) {
            Ok(_) => {
                info!("Successfully applied fix {}", fix_id);
                Ok(())
            }
            Err(e) => {
                error!("Failed to apply fix {}: {}", fix_id, e);
                Err(e)
            }
        }
    }
}

fn scan_system_logs(log_paths: &[String]) -> Result<Vec<ErrorEvent>> {
    let mut errors = Vec::new();
    
    // For now, simulate finding errors
    // In production, this would parse actual log files
    debug!("Scanning log paths: {:?}", log_paths);
    
    // Check for shell command errors in memory
    if let Some(error) = check_last_shell_error() {
        errors.push(error);
    }
    
    Ok(errors)
}

fn check_last_shell_error() -> Option<ErrorEvent> {
    // This would integrate with the shell's error tracking
    // For now, return None
    None
}

fn analyze_error(ai_client: &mut AiClient, error: &ErrorEvent) -> Result<FixCandidate> {
    // Use prompt templates for better results
    let prompt = PromptTemplates::get_prompt(error);
    
    // Validate patch safety before proposing
    let response = ai_client.generate_text(&prompt)
        .context("Failed to get AI response")?;
    
    // Validate the proposed patch
    if let Err(e) = validate_patch_safety(&response) {
        warn!("Proposed patch failed safety validation: {}", e);
        return Err(anyhow::anyhow!("Unsafe patch rejected"));
    }
    
    // Parse AI response into fix candidate
    let description = response.lines()
        .find(|line| line.starts_with("FIX:") || line.starts_with("CAUSE:"))
        .map(|line| line.trim_start_matches("FIX:").trim_start_matches("CAUSE:").trim())
        .unwrap_or("AI-generated fix")
        .to_string();
    
    Ok(FixCandidate {
        id: format!("fix_{}", error.id),
        error_id: error.id.clone(),
        description,
        patch: response,
        confidence: 0.75,
        tested: false,
        test_result: None,
    })
}