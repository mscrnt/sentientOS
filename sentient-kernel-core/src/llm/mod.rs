//! LLM subsystem for kernel-level AI integration

pub mod bootmod;
pub mod inference;
pub mod routing;

pub use bootmod::{boot_llm, boot_llm_eval, is_boot_model_available};

use crate::error::Result;
use alloc::string::String;

/// LLM service trait for different model providers
pub trait LLMService: Send + Sync {
    /// Evaluate a prompt and return response
    fn evaluate(&mut self, prompt: &str) -> Result<String>;
    
    /// Check if service is available
    fn is_available(&self) -> bool;
    
    /// Get service name
    fn name(&self) -> &'static str;
}

/// Initialize LLM subsystem
pub fn init() -> Result<()> {
    log::info!("Initializing LLM subsystem...");
    
    // Force lazy initialization of boot model
    let _ = bootmod::boot_llm();
    
    if is_boot_model_available() {
        log::info!("✓ Boot model loaded successfully");
    } else {
        log::warn!("⚠ Boot model not available, using fallback");
    }
    
    Ok(())
}