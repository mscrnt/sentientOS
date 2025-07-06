use super::*;
use anyhow::{Result, Context, bail};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub fix_id: String,
    pub timestamp: std::time::SystemTime,
    pub files: HashMap<PathBuf, FileSnapshot>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSnapshot {
    pub path: PathBuf,
    pub content: Vec<u8>,
    pub permissions: Option<u32>,
    pub hash: String,
}

pub struct RollbackManager {
    snapshot_dir: PathBuf,
    max_snapshots: usize,
}

impl RollbackManager {
    pub fn new(snapshot_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&snapshot_dir)?;
        
        Ok(Self {
            snapshot_dir,
            max_snapshots: 50,
        })
    }
    
    /// Create a snapshot before applying a fix
    pub fn create_snapshot(&self, fix_id: &str, files: &[PathBuf]) -> Result<String> {
        let snapshot_id = format!("snap_{}_{}", fix_id, chrono::Utc::now().timestamp());
        let mut file_snapshots = HashMap::new();
        
        for file_path in files {
            if file_path.exists() {
                let content = fs::read(file_path)
                    .context(format!("Failed to read file: {}", file_path.display()))?;
                
                let hash = calculate_file_hash(&content);
                
                #[cfg(unix)]
                let permissions = {
                    use std::os::unix::fs::PermissionsExt;
                    Some(fs::metadata(file_path)?.permissions().mode())
                };
                
                #[cfg(not(unix))]
                let permissions = None;
                
                file_snapshots.insert(
                    file_path.clone(),
                    FileSnapshot {
                        path: file_path.clone(),
                        content,
                        permissions,
                        hash,
                    }
                );
            }
        }
        
        let snapshot = Snapshot {
            id: snapshot_id.clone(),
            fix_id: fix_id.to_string(),
            timestamp: std::time::SystemTime::now(),
            files: file_snapshots,
            metadata: serde_json::json!({
                "created_by": "hivefix",
                "version": "1.0.0",
            }),
        };
        
        // Save snapshot
        self.save_snapshot(&snapshot)?;
        
        // Clean up old snapshots
        self.cleanup_old_snapshots()?;
        
        Ok(snapshot_id)
    }
    
    /// Rollback to a specific snapshot
    pub fn rollback(&self, snapshot_id: &str) -> Result<()> {
        let snapshot = self.load_snapshot(snapshot_id)?;
        
        // Verify files haven't been modified by external sources
        for (path, file_snap) in &snapshot.files {
            if path.exists() {
                let current_content = fs::read(path)?;
                let current_hash = calculate_file_hash(&current_content);
                
                // Warn if file was modified externally
                if current_hash != file_snap.hash {
                    log::warn!("File {} was modified externally since snapshot", path.display());
                }
            }
        }
        
        // Restore files
        for (path, file_snap) in snapshot.files {
            // Create backup of current state
            if path.exists() {
                let backup_path = path.with_extension("rollback.bak");
                fs::copy(&path, &backup_path)?;
            }
            
            // Restore content
            fs::write(&path, &file_snap.content)
                .context(format!("Failed to restore file: {}", path.display()))?;
            
            // Restore permissions
            #[cfg(unix)]
            if let Some(mode) = file_snap.permissions {
                use std::os::unix::fs::PermissionsExt;
                let permissions = fs::Permissions::from_mode(mode);
                fs::set_permissions(&path, permissions)?;
            }
        }
        
        Ok(())
    }
    
    /// List available snapshots
    pub fn list_snapshots(&self) -> Result<Vec<SnapshotInfo>> {
        let mut snapshots = Vec::new();
        
        for entry in fs::read_dir(&self.snapshot_dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|s| s.to_str()) == Some("snapshot") {
                if let Ok(snapshot) = self.load_snapshot_info(&entry.path()) {
                    snapshots.push(snapshot);
                }
            }
        }
        
        // Sort by timestamp (newest first)
        snapshots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        Ok(snapshots)
    }
    
    fn save_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        let snapshot_path = self.snapshot_dir.join(format!("{}.snapshot", snapshot.id));
        
        let data = serde_json::to_string_pretty(snapshot)?;
        fs::write(&snapshot_path, data)?;
        
        Ok(())
    }
    
    fn load_snapshot(&self, snapshot_id: &str) -> Result<Snapshot> {
        let snapshot_path = self.snapshot_dir.join(format!("{}.snapshot", snapshot_id));
        
        let data = fs::read_to_string(&snapshot_path)
            .context("Failed to read snapshot file")?;
        
        let snapshot: Snapshot = serde_json::from_str(&data)
            .context("Failed to parse snapshot")?;
        
        Ok(snapshot)
    }
    
    fn load_snapshot_info(&self, path: &Path) -> Result<SnapshotInfo> {
        let data = fs::read_to_string(path)?;
        let snapshot: Snapshot = serde_json::from_str(&data)?;
        
        Ok(SnapshotInfo {
            id: snapshot.id,
            fix_id: snapshot.fix_id,
            timestamp: snapshot.timestamp,
            file_count: snapshot.files.len(),
        })
    }
    
    fn cleanup_old_snapshots(&self) -> Result<()> {
        let snapshots = self.list_snapshots()?;
        
        if snapshots.len() > self.max_snapshots {
            // Remove oldest snapshots
            for snapshot in snapshots.iter().skip(self.max_snapshots) {
                let path = self.snapshot_dir.join(format!("{}.snapshot", snapshot.id));
                fs::remove_file(path).ok();
            }
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SnapshotInfo {
    pub id: String,
    pub fix_id: String,
    pub timestamp: std::time::SystemTime,
    pub file_count: usize,
}

fn calculate_file_hash(content: &[u8]) -> String {
    use sha2::{Sha256, Digest};
    
    let mut hasher = Sha256::new();
    hasher.update(content);
    
    format!("{:x}", hasher.finalize())
}