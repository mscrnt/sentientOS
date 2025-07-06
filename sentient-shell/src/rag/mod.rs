//! Retrieval-Augmented Generation (RAG) system for offline Q&A

pub mod index;
pub mod embedder;
pub mod retriever;
pub mod reranker;
pub mod generator;
pub mod cli;

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// RAG system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RAGConfig {
    pub index_path: PathBuf,
    pub embedding_model: String,
    pub reranker_model: String,
    pub generator_model: String,
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub top_k: usize,
    pub rerank_top_k: usize,
}

impl Default for RAGConfig {
    fn default() -> Self {
        Self {
            index_path: PathBuf::from("/opt/sentient/rag/index"),
            embedding_model: "Qwen3-Embedding-8B".to_string(),
            reranker_model: "Qwen3-Reranker-8B".to_string(),
            generator_model: "phi".to_string(),
            chunk_size: 512,
            chunk_overlap: 50,
            top_k: 10,
            rerank_top_k: 3,
        }
    }
}

/// Document to be indexed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub metadata: DocumentMetadata,
}

/// Document metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub source: String,
    pub doc_type: DocumentType,
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
    pub tags: Vec<String>,
}

/// Types of documents in the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DocumentType {
    SystemManual,
    KernelAPI,
    BootSequence,
    CrashLog,
    HiveFixHistory,
    AgentMemory,
    UserNote,
    Configuration,
}

/// RAG query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RAGResult {
    pub query: String,
    pub answer: String,
    pub sources: Vec<RetrievedChunk>,
    pub confidence: f32,
}

/// Retrieved document chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedChunk {
    pub content: String,
    pub score: f32,
    pub document_id: String,
    pub metadata: DocumentMetadata,
}

/// Main RAG system
pub struct RAGSystem {
    config: RAGConfig,
    embedder: embedder::Embedder,
    index: index::VectorIndex,
    retriever: retriever::Retriever,
    reranker: reranker::Reranker,
    generator: generator::Generator,
}

impl RAGSystem {
    /// Create new RAG system with default config
    pub fn new() -> Result<Self> {
        let config = RAGConfig::default();
        Self::with_config(config)
    }
    
    /// Create RAG system with custom config
    pub fn with_config(config: RAGConfig) -> Result<Self> {
        let embedder = embedder::Embedder::new(&config.embedding_model)?;
        let index = index::VectorIndex::load_or_create(&config.index_path)?;
        let retriever = retriever::Retriever::new(config.top_k);
        let reranker = reranker::Reranker::new(&config.reranker_model)?;
        let generator = generator::Generator::new(&config.generator_model)?;
        
        Ok(Self {
            config,
            embedder,
            index,
            retriever,
            reranker,
            generator,
        })
    }
    
    /// Index a document
    pub fn index_document(&mut self, doc: Document) -> Result<()> {
        log::info!("Indexing document: {}", doc.id);
        
        // Split into chunks
        let chunks = self.chunk_document(&doc)?;
        
        // Embed each chunk
        for (i, chunk) in chunks.iter().enumerate() {
            let embedding = self.embedder.embed(chunk)?;
            let chunk_id = format!("{}_{}", doc.id, i);
            
            self.index.add(chunk_id, embedding, doc.metadata.clone())?;
        }
        
        Ok(())
    }
    
    /// Query the RAG system
    pub fn query(&mut self, query: &str) -> Result<RAGResult> {
        log::info!("RAG query: {}", query);
        
        // 1. Embed the query
        let query_embedding = self.embedder.embed(query)?;
        
        // 2. Retrieve top-k chunks
        let candidates = self.retriever.retrieve(&self.index, &query_embedding)?;
        
        // 3. Rerank candidates
        let reranked = self.reranker.rerank(query, candidates)?;
        
        // 4. Generate answer using top chunks
        let context = self.build_context(&reranked);
        let answer = self.generator.generate(query, &context)?;
        
        // 5. Calculate confidence
        let confidence = self.calculate_confidence(&reranked);
        
        Ok(RAGResult {
            query: query.to_string(),
            answer,
            sources: reranked,
            confidence,
        })
    }
    
    /// Chunk a document into smaller pieces
    fn chunk_document(&self, doc: &Document) -> Result<Vec<String>> {
        let mut chunks = Vec::new();
        let content = &doc.content;
        let chunk_size = self.config.chunk_size;
        let overlap = self.config.chunk_overlap;
        
        let mut start = 0;
        while start < content.len() {
            let end = (start + chunk_size).min(content.len());
            let chunk = content[start..end].to_string();
            chunks.push(chunk);
            
            if end >= content.len() {
                break;
            }
            
            start = end - overlap;
        }
        
        Ok(chunks)
    }
    
    /// Build context from retrieved chunks
    fn build_context(&self, chunks: &[RetrievedChunk]) -> String {
        chunks.iter()
            .take(self.config.rerank_top_k)
            .enumerate()
            .map(|(i, chunk)| {
                format!("[Source {}] {}", i + 1, chunk.content)
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
    
    /// Calculate confidence score
    fn calculate_confidence(&self, chunks: &[RetrievedChunk]) -> f32 {
        if chunks.is_empty() {
            return 0.0;
        }
        
        // Average of top scores, normalized
        let sum: f32 = chunks.iter()
            .take(self.config.rerank_top_k)
            .map(|c| c.score)
            .sum();
        
        sum / self.config.rerank_top_k as f32
    }
    
    /// Index all documents in a directory
    pub fn index_directory(&mut self, path: &str, doc_type: DocumentType) -> Result<usize> {
        log::info!("Indexing directory: {} as {:?}", path, doc_type);
        
        let mut count = 0;
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let doc = Document {
                        id: path.to_string_lossy().to_string(),
                        content,
                        metadata: DocumentMetadata {
                            source: path.to_string_lossy().to_string(),
                            doc_type: doc_type.clone(),
                            timestamp: None,
                            tags: vec![],
                        },
                    };
                    
                    self.index_document(doc)?;
                    count += 1;
                }
            }
        }
        
        Ok(count)
    }
}