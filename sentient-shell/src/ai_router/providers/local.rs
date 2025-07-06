// Local inference provider (stub for now)
use crate::ai_router::*;
use anyhow::Result;

pub struct LocalProvider {
    model_path: String,
}

impl LocalProvider {
    pub fn new(model_path: String) -> Self {
        Self { model_path }
    }
}

impl ModelProvider for LocalProvider {
    fn name(&self) -> &str {
        "local"
    }

    fn is_available(&self) -> Result<bool> {
        // Check if local inference is compiled in
        #[cfg(feature = "local-inference")]
        {
            Ok(std::path::Path::new(&self.model_path).exists())
        }
        #[cfg(not(feature = "local-inference"))]
        {
            Ok(false)
        }
    }

    fn infer(&self, _request: &InferenceRequest) -> Result<InferenceResponse> {
        anyhow::bail!("Local inference not implemented yet")
    }

    fn list_models(&self) -> Result<Vec<ModelEndpoint>> {
        // Would scan local model directory
        Ok(vec![])
    }
}