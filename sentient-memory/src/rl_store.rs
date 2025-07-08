// RL Memory Store - Replay buffers and policy storage
// Phase 10: Native RL Integration

use anyhow::{Result, Context};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::fs;
use uuid::Uuid;
use bincode;
use flate2::{Compression, write::GzEncoder, read::GzDecoder};
use std::io::{Write, Read};

/// Experience for replay buffer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub state: Vec<f32>,
    pub action: Vec<f32>,
    pub reward: f32,
    pub next_state: Vec<f32>,
    pub done: bool,
    pub metadata: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

/// Trajectory - sequence of experiences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trajectory {
    pub id: Uuid,
    pub experiences: Vec<Experience>,
    pub total_reward: f32,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Policy checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyCheckpoint {
    pub id: Uuid,
    pub model_type: String,
    pub parameters: Vec<u8>,  // Serialized model parameters
    pub metadata: PolicyMetadata,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyMetadata {
    pub episode: usize,
    pub total_steps: usize,
    pub average_reward: f32,
    pub best_reward: f32,
    pub training_time_hours: f32,
    pub hyperparameters: serde_json::Value,
}

/// Replay buffer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayConfig {
    pub max_size: usize,
    pub batch_size: usize,
    pub prioritized: bool,
    pub alpha: f32,  // Priority exponent
    pub beta: f32,   // Importance sampling weight
    pub beta_increment: f32,
    pub epsilon: f32,  // Small value to ensure non-zero priorities
}

impl Default for ReplayConfig {
    fn default() -> Self {
        Self {
            max_size: 100_000,
            batch_size: 32,
            prioritized: true,
            alpha: 0.6,
            beta: 0.4,
            beta_increment: 0.001,
            epsilon: 1e-6,
        }
    }
}

/// Priority information for prioritized replay
#[derive(Debug, Clone)]
struct PriorityInfo {
    priority: f32,
    weight: f32,
    index: usize,
}

/// Replay buffer for experience replay
pub struct ReplayBuffer {
    config: ReplayConfig,
    buffer: Arc<RwLock<VecDeque<Experience>>>,
    priorities: Arc<RwLock<Vec<f32>>>,
    total_priority: Arc<RwLock<f32>>,
    min_priority: Arc<RwLock<f32>>,
    max_priority: Arc<RwLock<f32>>,
}

impl ReplayBuffer {
    pub fn new(config: ReplayConfig) -> Self {
        Self {
            config,
            buffer: Arc::new(RwLock::new(VecDeque::with_capacity(config.max_size))),
            priorities: Arc::new(RwLock::new(Vec::with_capacity(config.max_size))),
            total_priority: Arc::new(RwLock::new(0.0)),
            min_priority: Arc::new(RwLock::new(f32::MAX)),
            max_priority: Arc::new(RwLock::new(1.0)),
        }
    }
    
    /// Add experience to buffer
    pub async fn add(&self, experience: Experience) -> Result<()> {
        let mut buffer = self.buffer.write().await;
        let mut priorities = self.priorities.write().await;
        
        // If buffer is full, remove oldest
        if buffer.len() >= self.config.max_size {
            buffer.pop_front();
            if self.config.prioritized {
                let removed_priority = priorities.remove(0);
                let mut total = self.total_priority.write().await;
                *total -= removed_priority;
            }
        }
        
        // Add new experience
        buffer.push_back(experience);
        
        // Set priority for new experience
        if self.config.prioritized {
            let max_priority = *self.max_priority.read().await;
            priorities.push(max_priority);
            
            let mut total = self.total_priority.write().await;
            *total += max_priority;
        }
        
        Ok(())
    }
    
    /// Sample batch from buffer
    pub async fn sample(&self, batch_size: Option<usize>) -> Result<Vec<(Experience, f32, usize)>> {
        let size = batch_size.unwrap_or(self.config.batch_size);
        let buffer = self.buffer.read().await;
        
        if buffer.len() < size {
            return Err(anyhow::anyhow!("Not enough experiences in buffer"));
        }
        
        let mut batch = Vec::with_capacity(size);
        
        if self.config.prioritized {
            // Prioritized sampling
            let priorities = self.priorities.read().await;
            let total_priority = *self.total_priority.read().await;
            
            // Calculate segment size
            let segment_size = total_priority / size as f32;
            
            for i in 0..size {
                // Sample from segment
                let segment_start = i as f32 * segment_size;
                let segment_end = (i + 1) as f32 * segment_size;
                let sample_point = segment_start + rand::random::<f32>() * (segment_end - segment_start);
                
                // Find experience index
                let mut cumsum = 0.0;
                let mut idx = 0;
                for (j, &priority) in priorities.iter().enumerate() {
                    cumsum += priority;
                    if cumsum >= sample_point {
                        idx = j;
                        break;
                    }
                }
                
                // Calculate importance sampling weight
                let prob = priorities[idx] / total_priority;
                let weight = (buffer.len() as f32 * prob).powf(-self.config.beta);
                
                batch.push((buffer[idx].clone(), weight, idx));
            }
            
            // Normalize weights
            let max_weight = batch.iter().map(|(_, w, _)| *w).fold(0.0f32, f32::max);
            for (_, weight, _) in &mut batch {
                *weight /= max_weight;
            }
        } else {
            // Uniform sampling
            use rand::seq::SliceRandom;
            let indices: Vec<usize> = (0..buffer.len()).collect();
            let sampled_indices = indices.choose_multiple(&mut rand::thread_rng(), size);
            
            for &idx in sampled_indices {
                batch.push((buffer[idx].clone(), 1.0, idx));
            }
        }
        
        Ok(batch)
    }
    
    /// Update priorities for sampled experiences
    pub async fn update_priorities(&self, indices: &[usize], td_errors: &[f32]) -> Result<()> {
        if !self.config.prioritized {
            return Ok(());
        }
        
        let mut priorities = self.priorities.write().await;
        let mut total_priority = self.total_priority.write().await;
        let mut min_priority = self.min_priority.write().await;
        let mut max_priority = self.max_priority.write().await;
        
        for (&idx, &td_error) in indices.iter().zip(td_errors) {
            let new_priority = (td_error.abs() + self.config.epsilon).powf(self.config.alpha);
            
            // Update total
            *total_priority = *total_priority - priorities[idx] + new_priority;
            
            // Update priority
            priorities[idx] = new_priority;
            
            // Update min/max
            *min_priority = min_priority.min(new_priority);
            *max_priority = max_priority.max(new_priority);
        }
        
        Ok(())
    }
    
    /// Update beta for importance sampling
    pub fn update_beta(&mut self, increment: Option<f32>) {
        let inc = increment.unwrap_or(self.config.beta_increment);
        self.config.beta = (self.config.beta + inc).min(1.0);
    }
    
    /// Get current buffer size
    pub async fn len(&self) -> usize {
        self.buffer.read().await.len()
    }
    
    /// Clear buffer
    pub async fn clear(&self) {
        self.buffer.write().await.clear();
        self.priorities.write().await.clear();
        *self.total_priority.write().await = 0.0;
        *self.min_priority.write().await = f32::MAX;
        *self.max_priority.write().await = 1.0;
    }
    
    /// Save buffer to disk
    pub async fn save(&self, path: &Path) -> Result<()> {
        let buffer = self.buffer.read().await;
        let data = bincode::serialize(&buffer.clone().into_iter().collect::<Vec<_>>())?;
        
        // Compress data
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&data)?;
        let compressed = encoder.finish()?;
        
        fs::write(path, compressed).await?;
        log::info!("Saved replay buffer with {} experiences to {:?}", buffer.len(), path);
        
        Ok(())
    }
    
    /// Load buffer from disk
    pub async fn load(&self, path: &Path) -> Result<()> {
        let compressed = fs::read(path).await?;
        
        // Decompress data
        let mut decoder = GzDecoder::new(&compressed[..]);
        let mut data = Vec::new();
        decoder.read_to_end(&mut data)?;
        
        let experiences: Vec<Experience> = bincode::deserialize(&data)?;
        
        // Clear current buffer
        self.clear().await;
        
        // Add loaded experiences
        for exp in experiences {
            self.add(exp).await?;
        }
        
        log::info!("Loaded {} experiences from {:?}", self.len().await, path);
        Ok(())
    }
}

/// Storage for policy checkpoints
pub struct PolicyStorage {
    storage_dir: PathBuf,
    metadata_cache: Arc<RwLock<Vec<PolicyMetadata>>>,
}

impl PolicyStorage {
    pub fn new(storage_dir: PathBuf) -> Self {
        Self {
            storage_dir,
            metadata_cache: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Initialize storage directory
    pub async fn init(&self) -> Result<()> {
        fs::create_dir_all(&self.storage_dir).await?;
        self.refresh_cache().await?;
        Ok(())
    }
    
    /// Save policy checkpoint
    pub async fn save_checkpoint(&self, checkpoint: PolicyCheckpoint) -> Result<Uuid> {
        let checkpoint_dir = self.storage_dir.join(checkpoint.id.to_string());
        fs::create_dir_all(&checkpoint_dir).await?;
        
        // Save metadata
        let metadata_path = checkpoint_dir.join("metadata.json");
        let metadata_json = serde_json::to_string_pretty(&checkpoint)?;
        fs::write(&metadata_path, metadata_json).await?;
        
        // Save model parameters (compressed)
        let model_path = checkpoint_dir.join("model.bin.gz");
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&checkpoint.parameters)?;
        let compressed = encoder.finish()?;
        fs::write(&model_path, compressed).await?;
        
        // Update cache
        self.metadata_cache.write().await.push(checkpoint.metadata.clone());
        
        log::info!("Saved policy checkpoint: {}", checkpoint.id);
        Ok(checkpoint.id)
    }
    
    /// Load policy checkpoint
    pub async fn load_checkpoint(&self, id: Uuid) -> Result<PolicyCheckpoint> {
        let checkpoint_dir = self.storage_dir.join(id.to_string());
        
        // Load metadata
        let metadata_path = checkpoint_dir.join("metadata.json");
        let metadata_json = fs::read_to_string(&metadata_path).await?;
        let mut checkpoint: PolicyCheckpoint = serde_json::from_str(&metadata_json)?;
        
        // Load model parameters
        let model_path = checkpoint_dir.join("model.bin.gz");
        let compressed = fs::read(&model_path).await?;
        let mut decoder = GzDecoder::new(&compressed[..]);
        let mut parameters = Vec::new();
        decoder.read_to_end(&mut parameters)?;
        checkpoint.parameters = parameters;
        
        Ok(checkpoint)
    }
    
    /// List all checkpoints
    pub async fn list_checkpoints(&self) -> Result<Vec<PolicyMetadata>> {
        Ok(self.metadata_cache.read().await.clone())
    }
    
    /// Get best checkpoint by reward
    pub async fn get_best_checkpoint(&self) -> Result<Option<Uuid>> {
        let cache = self.metadata_cache.read().await;
        
        if cache.is_empty() {
            return Ok(None);
        }
        
        let best = cache.iter()
            .max_by(|a, b| a.best_reward.partial_cmp(&b.best_reward).unwrap())
            .unwrap();
        
        // Find checkpoint ID
        let mut entries = fs::read_dir(&self.storage_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                let metadata_path = entry.path().join("metadata.json");
                if metadata_path.exists() {
                    let metadata_json = fs::read_to_string(&metadata_path).await?;
                    let checkpoint: PolicyCheckpoint = serde_json::from_str(&metadata_json)?;
                    if checkpoint.metadata.best_reward == best.best_reward {
                        return Ok(Some(checkpoint.id));
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// Delete old checkpoints
    pub async fn cleanup(&self, keep_count: usize) -> Result<usize> {
        let mut cache = self.metadata_cache.write().await;
        
        if cache.len() <= keep_count {
            return Ok(0);
        }
        
        // Sort by episode number
        cache.sort_by_key(|m| m.episode);
        
        // Keep only the latest checkpoints
        let to_remove = cache.len() - keep_count;
        let removed_metadata: Vec<_> = cache.drain(..to_remove).collect();
        
        // Delete checkpoint directories
        let mut deleted = 0;
        let mut entries = fs::read_dir(&self.storage_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                let metadata_path = entry.path().join("metadata.json");
                if metadata_path.exists() {
                    let metadata_json = fs::read_to_string(&metadata_path).await?;
                    let checkpoint: PolicyCheckpoint = serde_json::from_str(&metadata_json)?;
                    
                    // Check if this checkpoint should be removed
                    if removed_metadata.iter().any(|m| m.episode == checkpoint.metadata.episode) {
                        fs::remove_dir_all(entry.path()).await?;
                        deleted += 1;
                    }
                }
            }
        }
        
        log::info!("Cleaned up {} old checkpoints", deleted);
        Ok(deleted)
    }
    
    /// Refresh metadata cache
    async fn refresh_cache(&self) -> Result<()> {
        let mut cache = Vec::new();
        
        let mut entries = fs::read_dir(&self.storage_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                let metadata_path = entry.path().join("metadata.json");
                if metadata_path.exists() {
                    let metadata_json = fs::read_to_string(&metadata_path).await?;
                    let checkpoint: PolicyCheckpoint = serde_json::from_str(&metadata_json)?;
                    cache.push(checkpoint.metadata);
                }
            }
        }
        
        *self.metadata_cache.write().await = cache;
        Ok(())
    }
}

/// Main RL memory store
pub struct RLMemoryStore {
    replay_buffers: Arc<Mutex<std::collections::HashMap<String, ReplayBuffer>>>,
    policy_storage: PolicyStorage,
    trajectories: Arc<RwLock<VecDeque<Trajectory>>>,
    max_trajectories: usize,
}

impl RLMemoryStore {
    pub fn new(storage_dir: PathBuf) -> Self {
        Self {
            replay_buffers: Arc::new(Mutex::new(std::collections::HashMap::new())),
            policy_storage: PolicyStorage::new(storage_dir.join("policies")),
            trajectories: Arc::new(RwLock::new(VecDeque::new())),
            max_trajectories: 1000,
        }
    }
    
    /// Initialize store
    pub async fn init(&self) -> Result<()> {
        self.policy_storage.init().await?;
        Ok(())
    }
    
    /// Get or create replay buffer
    pub async fn get_replay_buffer(&self, name: &str, config: Option<ReplayConfig>) -> ReplayBuffer {
        let mut buffers = self.replay_buffers.lock().await;
        
        if !buffers.contains_key(name) {
            let config = config.unwrap_or_default();
            let buffer = ReplayBuffer::new(config);
            buffers.insert(name.to_string(), buffer);
        }
        
        // Return a clone (Arc makes this cheap)
        let buffer = buffers.get(name).unwrap();
        ReplayBuffer {
            config: buffer.config.clone(),
            buffer: buffer.buffer.clone(),
            priorities: buffer.priorities.clone(),
            total_priority: buffer.total_priority.clone(),
            min_priority: buffer.min_priority.clone(),
            max_priority: buffer.max_priority.clone(),
        }
    }
    
    /// Add trajectory
    pub async fn add_trajectory(&self, trajectory: Trajectory) -> Result<()> {
        let mut trajectories = self.trajectories.write().await;
        
        if trajectories.len() >= self.max_trajectories {
            trajectories.pop_front();
        }
        
        trajectories.push_back(trajectory);
        Ok(())
    }
    
    /// Get recent trajectories
    pub async fn get_trajectories(&self, count: usize) -> Vec<Trajectory> {
        let trajectories = self.trajectories.read().await;
        trajectories.iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }
    
    /// Get policy storage
    pub fn policy_storage(&self) -> &PolicyStorage {
        &self.policy_storage
    }
    
    /// Save all buffers
    pub async fn save_all(&self, backup_dir: &Path) -> Result<()> {
        fs::create_dir_all(backup_dir).await?;
        
        let buffers = self.replay_buffers.lock().await;
        for (name, buffer) in buffers.iter() {
            let buffer_path = backup_dir.join(format!("{}_replay.bin.gz", name));
            buffer.save(&buffer_path).await?;
        }
        
        // Save trajectories
        let trajectories_path = backup_dir.join("trajectories.json.gz");
        let trajectories = self.trajectories.read().await;
        let data = serde_json::to_vec(&trajectories.clone().into_iter().collect::<Vec<_>>())?;
        
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&data)?;
        let compressed = encoder.finish()?;
        fs::write(&trajectories_path, compressed).await?;
        
        log::info!("Saved all memory stores to {:?}", backup_dir);
        Ok(())
    }
    
    /// Load all buffers
    pub async fn load_all(&self, backup_dir: &Path) -> Result<()> {
        // Load replay buffers
        let mut entries = fs::read_dir(backup_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(name) = path.file_stem() {
                if let Some(name_str) = name.to_str() {
                    if name_str.ends_with("_replay") {
                        let buffer_name = name_str.trim_end_matches("_replay");
                        let buffer = self.get_replay_buffer(buffer_name, None).await;
                        buffer.load(&path).await?;
                    }
                }
            }
        }
        
        // Load trajectories
        let trajectories_path = backup_dir.join("trajectories.json.gz");
        if trajectories_path.exists() {
            let compressed = fs::read(&trajectories_path).await?;
            let mut decoder = GzDecoder::new(&compressed[..]);
            let mut data = Vec::new();
            decoder.read_to_end(&mut data)?;
            
            let loaded_trajectories: Vec<Trajectory> = serde_json::from_slice(&data)?;
            let mut trajectories = self.trajectories.write().await;
            trajectories.clear();
            trajectories.extend(loaded_trajectories);
        }
        
        log::info!("Loaded all memory stores from {:?}", backup_dir);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_replay_buffer() {
        let config = ReplayConfig {
            max_size: 10,
            batch_size: 3,
            prioritized: false,
            ..Default::default()
        };
        
        let buffer = ReplayBuffer::new(config);
        
        // Add experiences
        for i in 0..5 {
            let exp = Experience {
                state: vec![i as f32],
                action: vec![i as f32],
                reward: i as f32,
                next_state: vec![(i + 1) as f32],
                done: false,
                metadata: None,
                timestamp: Utc::now(),
            };
            buffer.add(exp).await.unwrap();
        }
        
        assert_eq!(buffer.len().await, 5);
        
        // Sample batch
        let batch = buffer.sample(Some(3)).await.unwrap();
        assert_eq!(batch.len(), 3);
    }
    
    #[tokio::test]
    async fn test_prioritized_replay() {
        let config = ReplayConfig {
            max_size: 10,
            batch_size: 3,
            prioritized: true,
            ..Default::default()
        };
        
        let buffer = ReplayBuffer::new(config);
        
        // Add experiences
        for i in 0..5 {
            let exp = Experience {
                state: vec![i as f32],
                action: vec![i as f32],
                reward: i as f32,
                next_state: vec![(i + 1) as f32],
                done: false,
                metadata: None,
                timestamp: Utc::now(),
            };
            buffer.add(exp).await.unwrap();
        }
        
        // Sample and update priorities
        let batch = buffer.sample(Some(3)).await.unwrap();
        let indices: Vec<_> = batch.iter().map(|(_, _, idx)| *idx).collect();
        let td_errors = vec![1.0, 2.0, 0.5];
        
        buffer.update_priorities(&indices, &td_errors).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_policy_storage() {
        let temp_dir = std::env::temp_dir().join("test_policy_storage");
        let storage = PolicyStorage::new(temp_dir.clone());
        storage.init().await.unwrap();
        
        // Create checkpoint
        let checkpoint = PolicyCheckpoint {
            id: Uuid::new_v4(),
            model_type: "dqn".to_string(),
            parameters: vec![1, 2, 3, 4, 5],
            metadata: PolicyMetadata {
                episode: 100,
                total_steps: 10000,
                average_reward: 0.75,
                best_reward: 0.95,
                training_time_hours: 2.5,
                hyperparameters: serde_json::json!({
                    "learning_rate": 0.001,
                    "batch_size": 32
                }),
            },
            created_at: Utc::now(),
        };
        
        // Save checkpoint
        let id = storage.save_checkpoint(checkpoint.clone()).await.unwrap();
        
        // Load checkpoint
        let loaded = storage.load_checkpoint(id).await.unwrap();
        assert_eq!(loaded.metadata.episode, checkpoint.metadata.episode);
        assert_eq!(loaded.parameters, checkpoint.parameters);
        
        // List checkpoints
        let checkpoints = storage.list_checkpoints().await.unwrap();
        assert_eq!(checkpoints.len(), 1);
        
        // Cleanup
        std::fs::remove_dir_all(temp_dir).ok();
    }
}