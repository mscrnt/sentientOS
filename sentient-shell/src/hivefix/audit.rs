use super::*;
use anyhow::{Result, Context};
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Write, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub event_id: String,
    pub details: serde_json::Value,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    ErrorDetected,
    PromptSent,
    PatchProposed,
    PatchTested,
    PatchApplied,
    PatchRejected,
    SyncPerformed,
    ConfigChanged,
}

pub struct AuditLogger {
    log_path: PathBuf,
    writer: Mutex<Option<BufWriter<File>>>,
}

impl AuditLogger {
    pub fn new(log_path: PathBuf) -> Result<Self> {
        // Create log directory if needed
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        Ok(Self {
            log_path,
            writer: Mutex::new(None),
        })
    }
    
    pub fn init(&self) -> Result<()> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .context("Failed to open audit log")?;
        
        let mut writer = self.writer.lock().unwrap();
        *writer = Some(BufWriter::new(file));
        
        Ok(())
    }
    
    pub fn log_event(&self, event_type: AuditEventType, details: serde_json::Value) -> Result<String> {
        let event_id = generate_event_id();
        let hash = calculate_hash(&event_type, &details);
        
        let entry = AuditEntry {
            timestamp: Utc::now(),
            event_type,
            event_id: event_id.clone(),
            details,
            hash,
        };
        
        self.write_entry(&entry)?;
        
        Ok(event_id)
    }
    
    fn write_entry(&self, entry: &AuditEntry) -> Result<()> {
        let mut writer = self.writer.lock().unwrap();
        
        if let Some(w) = writer.as_mut() {
            let json = serde_json::to_string(entry)?;
            writeln!(w, "{}", json)?;
            w.flush()?;
        }
        
        Ok(())
    }
    
    pub fn log_error_detected(&self, error: &ErrorEvent) -> Result<()> {
        let details = serde_json::json!({
            "error_id": error.id,
            "source": format!("{:?}", error.source),
            "message": error.message,
            "has_stack_trace": error.stack_trace.is_some(),
        });
        
        self.log_event(AuditEventType::ErrorDetected, details)?;
        Ok(())
    }
    
    pub fn log_prompt_sent(&self, prompt: &str, model: &str) -> Result<()> {
        let details = serde_json::json!({
            "model": model,
            "prompt_length": prompt.len(),
            "prompt_hash": calculate_string_hash(prompt),
        });
        
        self.log_event(AuditEventType::PromptSent, details)?;
        Ok(())
    }
    
    pub fn log_patch_proposed(&self, fix: &FixCandidate) -> Result<()> {
        let details = serde_json::json!({
            "fix_id": fix.id,
            "error_id": fix.error_id,
            "description": fix.description,
            "confidence": fix.confidence,
            "patch_hash": calculate_string_hash(&fix.patch),
        });
        
        self.log_event(AuditEventType::PatchProposed, details)?;
        Ok(())
    }
    
    pub fn log_patch_tested(&self, fix_id: &str, result: &TestResult) -> Result<()> {
        let details = serde_json::json!({
            "fix_id": fix_id,
            "success": result.success,
            "duration_ms": result.duration_ms,
            "output_length": result.output.len(),
        });
        
        self.log_event(AuditEventType::PatchTested, details)?;
        Ok(())
    }
    
    pub fn log_patch_applied(&self, fix_id: &str, success: bool) -> Result<()> {
        let event_type = if success {
            AuditEventType::PatchApplied
        } else {
            AuditEventType::PatchRejected
        };
        
        let details = serde_json::json!({
            "fix_id": fix_id,
            "success": success,
            "timestamp": Utc::now().to_rfc3339(),
        });
        
        self.log_event(event_type, details)?;
        Ok(())
    }
}

/// Generate a unique event ID
fn generate_event_id() -> String {
    use std::time::SystemTime;
    
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    
    format!("evt_{:x}", timestamp)
}

/// Calculate hash for audit integrity
fn calculate_hash(event_type: &AuditEventType, details: &serde_json::Value) -> String {
    use sha2::{Sha256, Digest};
    
    let mut hasher = Sha256::new();
    hasher.update(format!("{:?}", event_type).as_bytes());
    hasher.update(details.to_string().as_bytes());
    
    format!("{:x}", hasher.finalize())
}

fn calculate_string_hash(s: &str) -> String {
    use sha2::{Sha256, Digest};
    
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    
    format!("{:x}", hasher.finalize())
}

/// Trace the full lifecycle of a fix
pub fn trace_fix(audit_log_path: &Path, fix_id: &str) -> Result<Vec<AuditEntry>> {
    use std::io::{BufRead, BufReader};
    
    let file = File::open(audit_log_path)?;
    let reader = BufReader::new(file);
    
    let mut entries = Vec::new();
    
    for line in reader.lines() {
        let line = line?;
        if let Ok(entry) = serde_json::from_str::<AuditEntry>(&line) {
            // Check if this entry is related to the fix
            if entry.details.get("fix_id")
                .and_then(|v| v.as_str())
                .map(|id| id == fix_id)
                .unwrap_or(false) 
            {
                entries.push(entry);
            }
        }
    }
    
    Ok(entries)
}