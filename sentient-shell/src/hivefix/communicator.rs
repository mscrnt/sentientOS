use super::*;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Serialize, Deserialize)]
pub struct HiveDelta {
    pub id: String,
    pub timestamp: SystemTime,
    pub machine_id: String,
    pub error_fingerprint: String,
    pub fix_description: String,
    pub patch_content: String,
    pub success_rate: f32,
    pub test_results: Vec<TestResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HiveSync {
    pub deltas: Vec<HiveDelta>,
    pub last_sync: SystemTime,
}

pub struct HiveCommunicator {
    server_url: Option<String>,
    machine_id: String,
}

impl HiveCommunicator {
    pub fn new(server_url: Option<String>) -> Self {
        // Generate unique machine ID (anonymized)
        let machine_id = generate_machine_id();
        
        Self {
            server_url,
            machine_id,
        }
    }

    pub async fn submit_fix(&self, fix: &FixCandidate, error: &ErrorEvent) -> Result<()> {
        if let Some(url) = &self.server_url {
            let delta = HiveDelta {
                id: fix.id.clone(),
                timestamp: SystemTime::now(),
                machine_id: self.machine_id.clone(),
                error_fingerprint: fingerprint_error(error),
                fix_description: fix.description.clone(),
                patch_content: sanitize_patch(&fix.patch),
                success_rate: fix.confidence,
                test_results: fix.test_result.as_ref().map(|r| vec![r.clone()]).unwrap_or_default(),
            };
            
            // Submit to hive server
            self.send_delta(url, &delta).await?;
        }
        
        Ok(())
    }

    pub async fn fetch_fixes(&self) -> Result<Vec<HiveDelta>> {
        if let Some(url) = &self.server_url {
            // Fetch relevant fixes from hive
            self.receive_deltas(url).await
        } else {
            Ok(Vec::new())
        }
    }

    async fn send_delta(&self, url: &str, delta: &HiveDelta) -> Result<()> {
        // In production, this would POST to the hive server
        log::info!("Would submit fix {} to hive server", delta.id);
        Ok(())
    }

    async fn receive_deltas(&self, url: &str) -> Result<Vec<HiveDelta>> {
        // In production, this would GET from the hive server
        log::info!("Would fetch fixes from hive server");
        Ok(Vec::new())
    }
}

fn generate_machine_id() -> String {
    // Generate anonymous machine ID
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    
    // Hash system info (in production, use actual system data)
    "sentientos-instance".hash(&mut hasher);
    SystemTime::now().hash(&mut hasher);
    
    format!("hive_{:x}", hasher.finish())
}

fn fingerprint_error(error: &ErrorEvent) -> String {
    // Create anonymized fingerprint of error
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    
    // Hash error components
    format!("{:?}", error.source).hash(&mut hasher);
    
    // Hash error message pattern (remove specific values)
    let pattern = anonymize_message(&error.message);
    pattern.hash(&mut hasher);
    
    format!("err_{:x}", hasher.finish())
}

fn anonymize_message(msg: &str) -> String {
    // Remove specific values, keep structure
    msg.split_whitespace()
        .map(|word| {
            if word.contains('/') || word.parse::<f64>().is_ok() {
                "[VALUE]"
            } else {
                word
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn sanitize_patch(patch: &str) -> String {
    // Remove sensitive information from patches
    patch.lines()
        .map(|line| {
            if line.contains("password") || line.contains("key") || line.contains("token") {
                "[REDACTED]"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}