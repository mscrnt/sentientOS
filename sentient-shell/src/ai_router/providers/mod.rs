// AI Provider implementations

pub mod ollama;
pub mod local;
pub mod boot;

use super::{ModelProvider, InferenceRequest, InferenceResponse, ModelEndpoint};