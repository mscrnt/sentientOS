//! Document retrieval from vector index

use super::{RetrievedChunk, DocumentMetadata};
use super::index::VectorIndex;
use anyhow::Result;

/// Document retriever
pub struct Retriever {
    top_k: usize,
}

impl Retriever {
    /// Create new retriever
    pub fn new(top_k: usize) -> Self {
        Self { top_k }
    }
    
    /// Retrieve relevant chunks from index
    pub fn retrieve(
        &self,
        index: &VectorIndex,
        query_embedding: &[f32],
    ) -> Result<Vec<RetrievedChunk>> {
        let results = index.search(query_embedding, self.top_k)?;
        
        let chunks: Vec<RetrievedChunk> = results
            .into_iter()
            .map(|(doc_id, score, metadata)| {
                RetrievedChunk {
                    content: self.get_chunk_content(&doc_id, &metadata),
                    score,
                    document_id: doc_id,
                    metadata,
                }
            })
            .collect();
        
        Ok(chunks)
    }
    
    /// Get chunk content from document ID
    fn get_chunk_content(&self, doc_id: &str, metadata: &DocumentMetadata) -> String {
        // In a real implementation, this would fetch the actual chunk
        // For now, we'll use the source path to load content
        
        if let Ok(content) = std::fs::read_to_string(&metadata.source) {
            // Extract the chunk based on ID (format: docpath_chunknum)
            if let Some(chunk_num_str) = doc_id.split('_').last() {
                if let Ok(chunk_num) = chunk_num_str.parse::<usize>() {
                    // Simple chunking: 512 chars with 50 overlap
                    let chunk_size = 512;
                    let overlap = 50;
                    let start = chunk_num * (chunk_size - overlap);
                    let end = (start + chunk_size).min(content.len());
                    
                    if start < content.len() {
                        return content[start..end].to_string();
                    }
                }
            }
            
            // Fallback: return first chunk
            content.chars().take(512).collect()
        } else {
            format!("[Content from {} not available]", metadata.source)
        }
    }
    
    /// Filter results by metadata
    pub fn filter_by_type(
        &self,
        chunks: Vec<RetrievedChunk>,
        doc_type: super::DocumentType,
    ) -> Vec<RetrievedChunk> {
        chunks
            .into_iter()
            .filter(|chunk| chunk.metadata.doc_type == doc_type)
            .collect()
    }
    
    /// Filter results by score threshold
    pub fn filter_by_score(
        &self,
        chunks: Vec<RetrievedChunk>,
        min_score: f32,
    ) -> Vec<RetrievedChunk> {
        chunks
            .into_iter()
            .filter(|chunk| chunk.score >= min_score)
            .collect()
    }
}