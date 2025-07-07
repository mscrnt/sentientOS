use anyhow::{Result, Context};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEntry {
    pub trace_id: String,
    pub timestamp: DateTime<Utc>,
    pub prompt: String,
    pub intent: String,
    pub model_used: String,
    pub tool_executed: Option<String>,
    pub rag_used: bool,
    pub conditions_evaluated: Vec<String>,
    pub success: bool,
    pub duration_ms: u64,
    pub reward: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTrace {
    pub entries: Vec<TraceEntry>,
}

pub struct TraceLogger {
    log_path: PathBuf,
    file_mutex: Arc<Mutex<()>>,
}

impl TraceLogger {
    pub async fn new(log_path: impl AsRef<Path>) -> Result<Self> {
        let log_path = log_path.as_ref().to_path_buf();
        
        // Ensure directory exists
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent)
                .await
                .context("Failed to create log directory")?;
        }
        
        Ok(Self {
            log_path,
            file_mutex: Arc::new(Mutex::new(())),
        })
    }
    
    pub async fn log(&self, entry: TraceEntry) -> Result<()> {
        let _lock = self.file_mutex.lock().await;
        
        let json_line = serde_json::to_string(&entry)
            .context("Failed to serialize trace entry")?;
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .await
            .context("Failed to open trace log file")?;
        
        file.write_all(json_line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;
        
        Ok(())
    }
    
    pub async fn update_reward(&self, trace_id: &str, reward: f64) -> Result<()> {
        let _lock = self.file_mutex.lock().await;
        
        // Read all entries
        let content = fs::read_to_string(&self.log_path)
            .await
            .context("Failed to read trace log")?;
        
        let mut entries: Vec<TraceEntry> = content
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_str(line))
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to parse trace entries")?;
        
        // Update the specific entry
        let mut updated = false;
        for entry in &mut entries {
            if entry.trace_id == trace_id {
                entry.reward = Some(reward);
                updated = true;
                break;
            }
        }
        
        if !updated {
            return Err(anyhow::anyhow!("Trace ID not found: {}", trace_id));
        }
        
        // Rewrite the file
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.log_path)
            .await
            .context("Failed to open trace log for writing")?;
        
        for entry in entries {
            let json_line = serde_json::to_string(&entry)?;
            file.write_all(json_line.as_bytes()).await?;
            file.write_all(b"\n").await?;
        }
        
        file.flush().await?;
        
        Ok(())
    }
    
    pub async fn load_traces(&self) -> Result<ExecutionTrace> {
        let content = fs::read_to_string(&self.log_path)
            .await
            .unwrap_or_default();
        
        let entries: Vec<TraceEntry> = content
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_str(line))
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to parse trace entries")?;
        
        Ok(ExecutionTrace { entries })
    }
    
    pub async fn get_summary(&self) -> Result<TraceSummary> {
        let traces = self.load_traces().await?;
        
        let total_executions = traces.entries.len();
        let successful_executions = traces.entries.iter().filter(|e| e.success).count();
        let rag_used_count = traces.entries.iter().filter(|e| e.rag_used).count();
        let tool_used_count = traces.entries.iter().filter(|e| e.tool_executed.is_some()).count();
        
        let mut model_usage: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut tool_usage: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut intent_distribution: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        
        let mut total_duration = 0u64;
        let mut rewarded_count = 0;
        let mut total_reward = 0.0;
        
        for entry in &traces.entries {
            *model_usage.entry(entry.model_used.clone()).or_insert(0) += 1;
            *intent_distribution.entry(entry.intent.clone()).or_insert(0) += 1;
            
            if let Some(tool) = &entry.tool_executed {
                *tool_usage.entry(tool.clone()).or_insert(0) += 1;
            }
            
            total_duration += entry.duration_ms;
            
            if let Some(reward) = entry.reward {
                rewarded_count += 1;
                total_reward += reward;
            }
        }
        
        let average_duration = if total_executions > 0 {
            total_duration as f64 / total_executions as f64
        } else {
            0.0
        };
        
        let average_reward = if rewarded_count > 0 {
            total_reward / rewarded_count as f64
        } else {
            0.0
        };
        
        Ok(TraceSummary {
            total_executions,
            successful_executions,
            success_rate: if total_executions > 0 {
                successful_executions as f64 / total_executions as f64
            } else {
                0.0
            },
            rag_used_count,
            tool_used_count,
            model_usage,
            tool_usage,
            intent_distribution,
            average_duration_ms: average_duration,
            average_reward,
            rewarded_count,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TraceSummary {
    pub total_executions: usize,
    pub successful_executions: usize,
    pub success_rate: f64,
    pub rag_used_count: usize,
    pub tool_used_count: usize,
    pub model_usage: std::collections::HashMap<String, usize>,
    pub tool_usage: std::collections::HashMap<String, usize>,
    pub intent_distribution: std::collections::HashMap<String, usize>,
    pub average_duration_ms: f64,
    pub average_reward: f64,
    pub rewarded_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_trace_logging() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test_trace.jsonl");
        
        let logger = TraceLogger::new(&log_path).await.unwrap();
        
        let entry = TraceEntry {
            trace_id: "test-123".to_string(),
            timestamp: Utc::now(),
            prompt: "Test prompt".to_string(),
            intent: "PureQuery".to_string(),
            model_used: "phi2_local".to_string(),
            tool_executed: None,
            rag_used: true,
            conditions_evaluated: vec![],
            success: true,
            duration_ms: 150,
            reward: None,
        };
        
        logger.log(entry.clone()).await.unwrap();
        
        // Test loading
        let traces = logger.load_traces().await.unwrap();
        assert_eq!(traces.entries.len(), 1);
        assert_eq!(traces.entries[0].trace_id, "test-123");
        
        // Test reward update
        logger.update_reward("test-123", 0.8).await.unwrap();
        
        let traces = logger.load_traces().await.unwrap();
        assert_eq!(traces.entries[0].reward, Some(0.8));
    }
}