//! CLI interface for RAG system

use super::*;
use anyhow::{Result, Context};
use std::sync::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref RAG_SYSTEM: Mutex<Option<RAGSystem>> = Mutex::new(None);
}

/// Initialize RAG system
pub fn init_rag() -> Result<String> {
    let mut rag_lock = RAG_SYSTEM.lock().unwrap();
    
    if rag_lock.is_some() {
        return Ok("RAG system already initialized".to_string());
    }
    
    log::info!("Initializing RAG system...");
    
    let rag = RAGSystem::new()?;
    *rag_lock = Some(rag);
    
    Ok("RAG system initialized successfully".to_string())
}

/// Handle RAG commands
pub fn handle_rag_command(args: &[&str]) -> Result<String> {
    if args.is_empty() {
        return Ok(rag_help());
    }
    
    match args[0] {
        "init" => init_rag(),
        "index" => {
            if args.len() < 2 {
                return Ok("Usage: rag index <docs|logs|memory|all>".to_string());
            }
            index_command(&args[1..])
        }
        "query" | "ask" => {
            if args.len() < 2 {
                return Ok("Usage: rag query <question>".to_string());
            }
            let query = args[1..].join(" ");
            query_command(&query)
        }
        "stats" => stats_command(),
        "help" => Ok(rag_help()),
        _ => Ok(format!("Unknown RAG command: {}. Try 'rag help'", args[0])),
    }
}

fn rag_help() -> String {
    r#"RAG (Retrieval-Augmented Generation) Commands:
  init              Initialize RAG system
  index <type>      Index documents (docs, logs, memory, all)
  query <question>  Ask a question using RAG
  stats             Show index statistics
  help              Show this help message

Examples:
  rag init
  rag index docs
  rag query "How does HiveFix recovery work?"
  
Prefixes:
  !@ rag <query>    Validated RAG query (safe mode)"#.to_string()
}

fn index_command(args: &[&str]) -> Result<String> {
    let mut rag_lock = RAG_SYSTEM.lock().unwrap();
    
    let rag = match rag_lock.as_mut() {
        Some(rag) => rag,
        None => return Ok("RAG system not initialized. Run 'rag init' first.".to_string()),
    };
    
    let mut total_indexed = 0;
    
    match args[0] {
        "docs" => {
            // Index system documentation
            let paths = vec![
                "/mnt/d/Projects/SentientOS/docs",
                "/mnt/d/Projects/SentientOS/sentient-shell/docs",
                "/mnt/d/Projects/SentientOS/README.md",
            ];
            
            for path in paths {
                if std::path::Path::new(path).exists() {
                    log::info!("Indexing documentation from: {}", path);
                    if std::path::Path::new(path).is_dir() {
                        total_indexed += rag.index_directory(path, DocumentType::SystemManual)?;
                    } else {
                        // Index single file
                        if let Ok(content) = std::fs::read_to_string(path) {
                            let doc = Document {
                                id: path.to_string(),
                                content,
                                metadata: DocumentMetadata {
                                    source: path.to_string(),
                                    doc_type: DocumentType::SystemManual,
                                    timestamp: None,
                                    tags: vec!["documentation".to_string()],
                                },
                            };
                            rag.index_document(doc)?;
                            total_indexed += 1;
                        }
                    }
                }
            }
        }
        "logs" => {
            // Index system logs
            let log_paths = vec![
                "/var/log/sentient",
                "/mnt/d/Projects/SentientOS/logs",
            ];
            
            for path in log_paths {
                if std::path::Path::new(path).exists() {
                    total_indexed += rag.index_directory(path, DocumentType::CrashLog)?;
                }
            }
        }
        "memory" => {
            // Index agent memory
            let memory_path = "/opt/sentient/rag/agent_memory";
            if std::path::Path::new(memory_path).exists() {
                total_indexed += rag.index_directory(memory_path, DocumentType::AgentMemory)?;
            }
        }
        "all" => {
            // Index everything
            total_indexed += index_command(&["docs"])?
                .split(' ')
                .nth(1)
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            
            total_indexed += index_command(&["logs"])?
                .split(' ')
                .nth(1)
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            
            total_indexed += index_command(&["memory"])?
                .split(' ')
                .nth(1)
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
        }
        _ => return Ok(format!("Unknown index type: {}", args[0])),
    }
    
    // Save index
    let index_path = std::path::PathBuf::from("/opt/sentient/rag/index");
    rag.index.save(&index_path)?;
    
    Ok(format!("Indexed {} documents successfully", total_indexed))
}

fn query_command(query: &str) -> Result<String> {
    let mut rag_lock = RAG_SYSTEM.lock().unwrap();
    
    let rag = match rag_lock.as_mut() {
        Some(rag) => rag,
        None => return Ok("RAG system not initialized. Run 'rag init' first.".to_string()),
    };
    
    log::info!("Processing RAG query: {}", query);
    
    let result = rag.query(query)?;
    
    // Format the response
    let mut output = String::new();
    
    output.push_str(&format!("Query: {}\n\n", result.query));
    output.push_str(&format!("Answer:\n{}\n\n", result.answer));
    
    if !result.sources.is_empty() {
        output.push_str("Sources:\n");
        for (i, source) in result.sources.iter().take(3).enumerate() {
            output.push_str(&format!(
                "[{}] {} (score: {:.2})\n",
                i + 1,
                source.metadata.source,
                source.score
            ));
        }
    }
    
    output.push_str(&format!("\nConfidence: {:.0}%", result.confidence * 100.0));
    
    Ok(output)
}

fn stats_command() -> Result<String> {
    let rag_lock = RAG_SYSTEM.lock().unwrap();
    
    let rag = match rag_lock.as_ref() {
        Some(rag) => rag,
        None => return Ok("RAG system not initialized. Run 'rag init' first.".to_string()),
    };
    
    let stats = rag.index.stats();
    
    Ok(format!(
        "RAG System Statistics:\n\
        Indexed vectors: {}\n\
        Vector dimension: {}\n\
        Memory usage: {} MB\n\
        Config: {:?}",
        stats.num_vectors,
        stats.dimension,
        stats.memory_usage_mb,
        rag.config
    ))
}

/// Direct RAG query for prefix commands
pub fn rag_query_direct(query: &str) -> Result<String> {
    // Ensure RAG is initialized
    init_rag()?;
    
    // Process query
    query_command(query)
}