//! Vector index for storing and searching embeddings

use super::DocumentMetadata;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::fs;

/// Vector dimension for embeddings
const VECTOR_DIM: usize = 768; // Adjust based on model

/// Simple in-memory vector index
#[derive(Debug, Serialize, Deserialize)]
pub struct VectorIndex {
    vectors: HashMap<String, Vector>,
    metadata: HashMap<String, DocumentMetadata>,
    dimension: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vector {
    pub data: Vec<f32>,
}

impl VectorIndex {
    /// Create new empty index
    pub fn new(dimension: usize) -> Self {
        Self {
            vectors: HashMap::new(),
            metadata: HashMap::new(),
            dimension,
        }
    }
    
    /// Load index from disk or create new
    pub fn load_or_create(path: &Path) -> Result<Self> {
        let index_file = path.join("index.json");
        
        if index_file.exists() {
            log::info!("Loading existing index from {:?}", index_file);
            let data = fs::read_to_string(&index_file)
                .context("Failed to read index file")?;
            let index: Self = serde_json::from_str(&data)
                .context("Failed to parse index")?;
            Ok(index)
        } else {
            log::info!("Creating new index at {:?}", path);
            fs::create_dir_all(path)?;
            Ok(Self::new(VECTOR_DIM))
        }
    }
    
    /// Save index to disk
    pub fn save(&self, path: &Path) -> Result<()> {
        let index_file = path.join("index.json");
        let data = serde_json::to_string_pretty(self)
            .context("Failed to serialize index")?;
        fs::write(&index_file, data)
            .context("Failed to write index file")?;
        Ok(())
    }
    
    /// Add a vector to the index
    pub fn add(&mut self, id: String, embedding: Vec<f32>, metadata: DocumentMetadata) -> Result<()> {
        if embedding.len() != self.dimension {
            anyhow::bail!("Embedding dimension mismatch: expected {}, got {}", 
                self.dimension, embedding.len());
        }
        
        self.vectors.insert(id.clone(), Vector { data: embedding });
        self.metadata.insert(id, metadata);
        Ok(())
    }
    
    /// Search for nearest neighbors
    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<(String, f32, DocumentMetadata)>> {
        if query.len() != self.dimension {
            anyhow::bail!("Query dimension mismatch");
        }
        
        // Calculate distances to all vectors
        let mut distances: Vec<(String, f32)> = self.vectors
            .iter()
            .map(|(id, vector)| {
                let distance = cosine_similarity(query, &vector.data);
                (id.clone(), distance)
            })
            .collect();
        
        // Sort by distance (descending for cosine similarity)
        distances.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Return top-k results with metadata
        let results = distances
            .into_iter()
            .take(k)
            .filter_map(|(id, score)| {
                self.metadata.get(&id).map(|meta| {
                    (id, score, meta.clone())
                })
            })
            .collect();
        
        Ok(results)
    }
    
    /// Get index statistics
    pub fn stats(&self) -> IndexStats {
        IndexStats {
            num_vectors: self.vectors.len(),
            dimension: self.dimension,
            memory_usage_mb: self.estimate_memory_usage() / 1_000_000,
        }
    }
    
    fn estimate_memory_usage(&self) -> usize {
        // Rough estimate: vectors + metadata
        let vector_size = self.vectors.len() * self.dimension * 4; // f32 = 4 bytes
        let metadata_size = self.metadata.len() * 200; // Rough estimate
        vector_size + metadata_size
    }
}

/// Index statistics
#[derive(Debug)]
pub struct IndexStats {
    pub num_vectors: usize,
    pub dimension: usize,
    pub memory_usage_mb: usize,
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert_eq!(cosine_similarity(&a, &b), 1.0);
        
        let c = vec![0.0, 1.0, 0.0];
        assert_eq!(cosine_similarity(&a, &c), 0.0);
    }
    
    #[test]
    fn test_vector_index() {
        let mut index = VectorIndex::new(3);
        
        let meta = DocumentMetadata {
            source: "test.txt".to_string(),
            doc_type: super::super::DocumentType::UserNote,
            timestamp: None,
            tags: vec![],
        };
        
        index.add("doc1".to_string(), vec![1.0, 0.0, 0.0], meta.clone()).unwrap();
        index.add("doc2".to_string(), vec![0.0, 1.0, 0.0], meta.clone()).unwrap();
        
        let results = index.search(&[1.0, 0.0, 0.0], 2).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "doc1");
        assert_eq!(results[0].1, 1.0);
    }
}