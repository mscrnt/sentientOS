// Centralized logging system for SentientOS
// Provides structured logging with filtering and persistence

use anyhow::{Result, Context};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Log entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub source: String,
    pub category: LogCategory,
    pub message: String,
    pub metadata: LogMetadata,
}

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
    Critical,
}

/// Log category for filtering
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogCategory {
    Goal,
    System,
    Network,
    Service,
    Activity,
    Error,
    Other(String),
}

/// Log metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LogMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goal_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reward: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
}

/// Log filter options
#[derive(Debug, Clone, Default)]
pub struct LogFilter {
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub level: Option<LogLevel>,
    pub category: Option<LogCategory>,
    pub source: Option<String>,
    pub goal_id: Option<String>,
    pub tool: Option<String>,
    pub failed_only: bool,
    pub limit: Option<usize>,
}

/// Trait for log storage backends
pub trait LogStore: Send + Sync {
    /// Write a log entry
    fn write(&mut self, entry: &LogEntry) -> Result<()>;
    
    /// Read logs with filter
    fn read(&self, filter: &LogFilter) -> Result<Vec<LogEntry>>;
    
    /// Flush any buffered data
    fn flush(&mut self) -> Result<()>;
    
    /// Clear old logs
    fn rotate(&mut self, keep_days: u32) -> Result<usize>;
}

/// In-memory log store with size limits
pub struct MemoryLogStore {
    entries: Arc<Mutex<VecDeque<LogEntry>>>,
    max_entries: usize,
}

impl MemoryLogStore {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(max_entries))),
            max_entries,
        }
    }
}

impl LogStore for MemoryLogStore {
    fn write(&mut self, entry: &LogEntry) -> Result<()> {
        let mut entries = self.entries.lock().unwrap();
        
        if entries.len() >= self.max_entries {
            entries.pop_front();
        }
        
        entries.push_back(entry.clone());
        Ok(())
    }
    
    fn read(&self, filter: &LogFilter) -> Result<Vec<LogEntry>> {
        let entries = self.entries.lock().unwrap();
        
        let filtered: Vec<LogEntry> = entries
            .iter()
            .filter(|entry| {
                // Apply filters
                if let Some(since) = filter.since {
                    if entry.timestamp < since {
                        return false;
                    }
                }
                
                if let Some(until) = filter.until {
                    if entry.timestamp > until {
                        return false;
                    }
                }
                
                if let Some(level) = filter.level {
                    if entry.level as u8 < level as u8 {
                        return false;
                    }
                }
                
                if let Some(ref category) = filter.category {
                    if &entry.category != category {
                        return false;
                    }
                }
                
                if let Some(ref source) = filter.source {
                    if !entry.source.contains(source) {
                        return false;
                    }
                }
                
                if let Some(ref goal_id) = filter.goal_id {
                    if entry.metadata.goal_id.as_ref() != Some(goal_id) {
                        return false;
                    }
                }
                
                if let Some(ref tool) = filter.tool {
                    if entry.metadata.tool.as_ref() != Some(tool) {
                        return false;
                    }
                }
                
                if filter.failed_only {
                    if entry.metadata.success != Some(false) {
                        return false;
                    }
                }
                
                true
            })
            .cloned()
            .collect();
        
        // Apply limit
        let result = if let Some(limit) = filter.limit {
            filtered.into_iter().rev().take(limit).rev().collect()
        } else {
            filtered
        };
        
        Ok(result)
    }
    
    fn flush(&mut self) -> Result<()> {
        Ok(()) // No-op for memory store
    }
    
    fn rotate(&mut self, _keep_days: u32) -> Result<usize> {
        let mut entries = self.entries.lock().unwrap();
        let count = entries.len();
        entries.clear();
        Ok(count)
    }
}

/// File-based log store with JSON lines format
pub struct FileLogStore {
    base_path: PathBuf,
    current_file: Option<File>,
    current_date: Option<String>,
}

impl FileLogStore {
    pub fn new<P: AsRef<Path>>(base_path: P) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        std::fs::create_dir_all(&base_path)?;
        
        Ok(Self {
            base_path,
            current_file: None,
            current_date: None,
        })
    }
    
    fn get_log_file(&mut self) -> Result<&mut File> {
        let today = Utc::now().format("%Y%m%d").to_string();
        
        if self.current_date.as_ref() != Some(&today) {
            // Open new file for today
            let filename = format!("sentient_log_{}.jsonl", today);
            let path = self.base_path.join(filename);
            
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?;
            
            self.current_file = Some(file);
            self.current_date = Some(today);
        }
        
        self.current_file
            .as_mut()
            .context("No log file open")
    }
}

impl LogStore for FileLogStore {
    fn write(&mut self, entry: &LogEntry) -> Result<()> {
        let file = self.get_log_file()?;
        let json = serde_json::to_string(entry)?;
        writeln!(file, "{}", json)?;
        Ok(())
    }
    
    fn read(&self, filter: &LogFilter) -> Result<Vec<LogEntry>> {
        let mut results = Vec::new();
        
        // Read all log files in directory
        for entry in std::fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                let file = File::open(&path)?;
                let reader = BufReader::new(file);
                
                for line in reader.lines() {
                    let line = line?;
                    if let Ok(log_entry) = serde_json::from_str::<LogEntry>(&line) {
                        // Apply filters (same logic as MemoryLogStore)
                        if Self::matches_filter(&log_entry, filter) {
                            results.push(log_entry);
                        }
                    }
                }
            }
        }
        
        // Sort by timestamp
        results.sort_by_key(|e| e.timestamp);
        
        // Apply limit
        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }
        
        Ok(results)
    }
    
    fn flush(&mut self) -> Result<()> {
        if let Some(ref mut file) = self.current_file {
            file.flush()?;
        }
        Ok(())
    }
    
    fn rotate(&mut self, keep_days: u32) -> Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::days(keep_days as i64);
        let mut removed = 0;
        
        for entry in std::fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    let modified: DateTime<Utc> = modified.into();
                    if modified < cutoff {
                        std::fs::remove_file(path)?;
                        removed += 1;
                    }
                }
            }
        }
        
        Ok(removed)
    }
}

impl FileLogStore {
    fn matches_filter(entry: &LogEntry, filter: &LogFilter) -> bool {
        if let Some(since) = filter.since {
            if entry.timestamp < since {
                return false;
            }
        }
        
        if let Some(until) = filter.until {
            if entry.timestamp > until {
                return false;
            }
        }
        
        if let Some(level) = filter.level {
            if entry.level as u8 < level as u8 {
                return false;
            }
        }
        
        if let Some(ref category) = filter.category {
            if &entry.category != category {
                return false;
            }
        }
        
        if let Some(ref source) = filter.source {
            if !entry.source.contains(source) {
                return false;
            }
        }
        
        if let Some(ref goal_id) = filter.goal_id {
            if entry.metadata.goal_id.as_ref() != Some(goal_id) {
                return false;
            }
        }
        
        if let Some(ref tool) = filter.tool {
            if entry.metadata.tool.as_ref() != Some(tool) {
                return false;
            }
        }
        
        if filter.failed_only {
            if entry.metadata.success != Some(false) {
                return false;
            }
        }
        
        true
    }
}

/// Combined log store that writes to multiple backends
pub struct MultiLogStore {
    stores: Vec<Box<dyn LogStore>>,
}

impl MultiLogStore {
    pub fn new() -> Self {
        Self {
            stores: Vec::new(),
        }
    }
    
    pub fn add_store(&mut self, store: Box<dyn LogStore>) {
        self.stores.push(store);
    }
}

impl LogStore for MultiLogStore {
    fn write(&mut self, entry: &LogEntry) -> Result<()> {
        for store in &mut self.stores {
            store.write(entry)?;
        }
        Ok(())
    }
    
    fn read(&self, filter: &LogFilter) -> Result<Vec<LogEntry>> {
        // Read from first store only (typically memory for speed)
        if let Some(store) = self.stores.first() {
            store.read(filter)
        } else {
            Ok(Vec::new())
        }
    }
    
    fn flush(&mut self) -> Result<()> {
        for store in &mut self.stores {
            store.flush()?;
        }
        Ok(())
    }
    
    fn rotate(&mut self, keep_days: u32) -> Result<usize> {
        let mut total = 0;
        for store in &mut self.stores {
            total += store.rotate(keep_days)?;
        }
        Ok(total)
    }
}

/// Global logger instance
static LOGGER: Mutex<Option<Box<dyn LogStore>>> = Mutex::new(None);

/// Initialize the global logger
pub fn init_logger(store: Box<dyn LogStore>) {
    let mut logger = LOGGER.lock().unwrap();
    *logger = Some(store);
}

/// Log a message
pub fn log(entry: LogEntry) -> Result<()> {
    let mut logger = LOGGER.lock().unwrap();
    if let Some(ref mut store) = *logger {
        store.write(&entry)?;
    }
    Ok(())
}

/// Read logs with filter
pub fn read_logs(filter: &LogFilter) -> Result<Vec<LogEntry>> {
    let logger = LOGGER.lock().unwrap();
    if let Some(ref store) = *logger {
        store.read(filter)
    } else {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_log_store() {
        let mut store = MemoryLogStore::new(10);
        
        let entry = LogEntry {
            timestamp: Utc::now(),
            level: LogLevel::Info,
            source: "test".to_string(),
            category: LogCategory::System,
            message: "Test message".to_string(),
            metadata: LogMetadata::default(),
        };
        
        store.write(&entry).unwrap();
        
        let filter = LogFilter::default();
        let logs = store.read(&filter).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].message, "Test message");
    }
    
    #[test]
    fn test_log_filter() {
        let mut store = MemoryLogStore::new(100);
        
        // Add various log entries
        for i in 0..10 {
            let entry = LogEntry {
                timestamp: Utc::now(),
                level: if i % 2 == 0 { LogLevel::Info } else { LogLevel::Error },
                source: format!("test{}", i % 3),
                category: LogCategory::System,
                message: format!("Message {}", i),
                metadata: LogMetadata {
                    success: Some(i % 3 != 0),
                    ..Default::default()
                },
            };
            store.write(&entry).unwrap();
        }
        
        // Test level filter
        let filter = LogFilter {
            level: Some(LogLevel::Error),
            ..Default::default()
        };
        let logs = store.read(&filter).unwrap();
        assert_eq!(logs.len(), 5);
        
        // Test failed only filter
        let filter = LogFilter {
            failed_only: true,
            ..Default::default()
        };
        let logs = store.read(&filter).unwrap();
        assert_eq!(logs.len(), 3);
    }
}