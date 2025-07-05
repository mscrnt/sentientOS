#[cfg(feature = "local-inference")]
use anyhow::{Result, Context};
#[cfg(feature = "local-inference")]
use tract_onnx::prelude::*;

#[cfg(feature = "local-inference")]
pub struct LocalInference {
    model: Option<SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>>,
}

#[cfg(feature = "local-inference")]
impl LocalInference {
    pub fn new() -> Result<Self> {
        // Try to load a model if available
        let model_path = std::env::var("SENTIENT_LOCAL_MODEL")
            .unwrap_or_else(|_| "models/tiny_llm.onnx".to_string());
            
        let model = if std::path::Path::new(&model_path).exists() {
            log::info!("Loading local model from {}", model_path);
            match Self::load_model(&model_path) {
                Ok(m) => Some(m),
                Err(e) => {
                    log::warn!("Failed to load local model: {}", e);
                    None
                }
            }
        } else {
            log::info!("No local model found at {}", model_path);
            None
        };
        
        Ok(Self { model })
    }
    
    fn load_model(path: &str) -> Result<SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>> {
        let model = tract_onnx::onnx()
            .model_for_path(path)
            .context("Failed to load ONNX model")?
            .into_optimized()
            .context("Failed to optimize model")?
            .into_runnable()
            .context("Failed to make model runnable")?;
            
        Ok(model)
    }
    
    pub fn infer(&mut self, prompt: &str) -> Result<String> {
        if self.model.is_none() {
            anyhow::bail!("No local model loaded");
        }
        
        // This is a simplified example - real inference would:
        // 1. Tokenize the prompt
        // 2. Convert to tensor
        // 3. Run inference
        // 4. Decode output tokens
        
        // For demo purposes, return a fixed response
        Ok(format!(
            "Local inference response for '{}': This is a demo response from the local ONNX model. \
            In a real implementation, this would tokenize your input and generate a proper response.",
            prompt
        ))
    }
    
    pub fn load_test_model() -> Result<()> {
        // This demonstrates loading a simple MNIST model for testing
        log::info!("Loading test MNIST model...");
        
        let model_bytes = include_bytes!("../models/mnist.onnx");
        let model = tract_onnx::onnx()
            .model_for_read(&mut &model_bytes[..])
            .context("Failed to load MNIST model")?;
            
        // Print model info
        log::info!("Model inputs: {:?}", model.inputs);
        log::info!("Model outputs: {:?}", model.outputs);
        
        // Create test input (28x28 image)
        let input = tract_ndarray::Array4::<f32>::zeros((1, 1, 28, 28));
        
        let model = model
            .into_optimized()?
            .into_runnable()?;
            
        // Run inference
        let output = model.run(tvec!(input.into()))?;
        
        // Get predictions
        let predictions = output[0]
            .to_array_view::<f32>()?
            .iter()
            .cloned()
            .collect::<Vec<_>>();
            
        log::info!("MNIST predictions: {:?}", predictions);
        
        Ok(())
    }
}

#[cfg(not(feature = "local-inference"))]
pub struct LocalInference;

#[cfg(not(feature = "local-inference"))]
impl LocalInference {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }
    
    pub fn infer(&mut self, _prompt: &str) -> Result<String> {
        anyhow::bail!("Local inference not enabled")
    }
}