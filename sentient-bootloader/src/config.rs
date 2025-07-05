use crate::boot_info::RuntimeType;
use alloc::string::String;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    pub model_path: String,
    pub runtime: RuntimeType,
    pub batch_size: u32,
    pub context_length: u32,
    pub temperature: f32,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            model_path: String::from("\\neuro_model.gguf"),
            runtime: RuntimeType::Hybrid,
            batch_size: 1,
            context_length: 4096,
            temperature: 0.7,
        }
    }
}
