use super::*;
use anyhow::{Result, Context};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Patch {
    pub file_path: Option<String>,
    pub old_content: Option<String>,
    pub new_content: String,
    pub patch_type: PatchType,
}

#[derive(Debug, Clone)]
pub enum PatchType {
    FilePatch,      // Modify existing file
    ConfigPatch,    // Update configuration
    CodePatch,      // Fix code issues
    StatePatch,     // Fix corrupted state
}

pub fn generate_patch(error: &ErrorEvent, ai_suggestion: &str) -> Result<Patch> {
    // Parse AI suggestion to determine patch type and content
    let patch_type = detect_patch_type(error);
    
    // Extract file path if mentioned
    let file_path = extract_file_path(ai_suggestion);
    
    Ok(Patch {
        file_path,
        old_content: None,
        new_content: ai_suggestion.to_string(),
        patch_type,
    })
}

pub fn apply_patch(patch_content: &str) -> Result<()> {
    // Parse patch content
    // For now, this is a simplified implementation
    
    if patch_content.contains("FILE:") {
        // File-based patch
        let lines: Vec<&str> = patch_content.lines().collect();
        for line in lines {
            if let Some(file_path) = line.strip_prefix("FILE:") {
                // In production, apply actual file changes
                log::info!("Would patch file: {}", file_path.trim());
            }
        }
    }
    
    Ok(())
}

fn detect_patch_type(error: &ErrorEvent) -> PatchType {
    match &error.source {
        ErrorSource::Shell => PatchType::CodePatch,
        ErrorSource::Package(_) => PatchType::CodePatch,
        ErrorSource::Kernel => PatchType::ConfigPatch,
        ErrorSource::System => PatchType::ConfigPatch,
        ErrorSource::User => PatchType::StatePatch,
    }
}

fn extract_file_path(content: &str) -> Option<String> {
    // Look for file paths in the content
    for line in content.lines() {
        if line.contains("/") && (line.contains(".rs") || line.contains(".toml")) {
            // Simple heuristic to find file paths
            let parts: Vec<&str> = line.split_whitespace().collect();
            for part in parts {
                if part.contains("/") && !part.starts_with("http") {
                    return Some(part.to_string());
                }
            }
        }
    }
    None
}

pub fn create_backup(file_path: &Path) -> Result<PathBuf> {
    let backup_path = file_path.with_extension("hivefix.bak");
    fs::copy(file_path, &backup_path)
        .context("Failed to create backup")?;
    Ok(backup_path)
}

pub fn restore_backup(backup_path: &Path) -> Result<()> {
    let original_path = backup_path.with_extension("");
    fs::copy(backup_path, original_path)
        .context("Failed to restore backup")?;
    fs::remove_file(backup_path)?;
    Ok(())
}