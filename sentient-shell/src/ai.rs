use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct AiClient {
    ollama_url: String,
    sd_url: String,
    client: reqwest::blocking::Client,
}

#[derive(Debug)]
pub struct ImageInfo {
    pub hash: String,
    pub size: usize,
}

// Ollama API structures
#[derive(Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
    done: bool,
}

#[derive(Deserialize)]
struct OllamaModel {
    name: String,
}

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

// Stable Diffusion API structures
#[derive(Serialize)]
struct SDTxt2ImgRequest {
    prompt: String,
    negative_prompt: String,
    steps: u32,
    width: u32,
    height: u32,
}

#[derive(Deserialize)]
struct SDTxt2ImgResponse {
    images: Vec<String>,
}

#[derive(Deserialize)]
struct SDModel {
    title: String,
    model_name: String,
}

impl AiClient {
    pub fn new(ollama_url: String, sd_url: String) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());
            
        Self {
            ollama_url,
            sd_url,
            client,
        }
    }
    
    pub fn ollama_url(&self) -> &str {
        &self.ollama_url
    }
    
    pub fn sd_url(&self) -> &str {
        &self.sd_url
    }
    
    pub fn check_ollama_connection(&self) -> Result<bool> {
        let url = format!("{}/api/tags", self.ollama_url);
        match self.client.get(&url).send() {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
    
    pub fn check_sd_connection(&self) -> Result<bool> {
        let url = format!("{}/sdapi/v1/sd-models", self.sd_url);
        match self.client.get(&url).send() {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
    
    pub fn get_preferred_model(&self) -> Result<Option<String>> {
        let models = self.list_ollama_models()?;
        
        // Prefer deepseek-v2 if available
        if models.iter().any(|m| m.contains("deepseek-v2")) {
            return Ok(Some("deepseek-v2".to_string()));
        }
        
        // Otherwise use the first available model
        Ok(models.into_iter().next())
    }
    
    pub fn list_ollama_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/api/tags", self.ollama_url);
        let resp = self.client
            .get(&url)
            .send()
            .context("Failed to connect to Ollama")?;
            
        if !resp.status().is_success() {
            anyhow::bail!("Ollama API returned error: {}", resp.status());
        }
        
        let tags: OllamaTagsResponse = resp.json()
            .context("Failed to parse Ollama response")?;
            
        Ok(tags.models.into_iter().map(|m| m.name).collect())
    }
    
    pub fn list_sd_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/sdapi/v1/sd-models", self.sd_url);
        let resp = self.client
            .get(&url)
            .send()
            .context("Failed to connect to Stable Diffusion")?;
            
        if !resp.status().is_success() {
            anyhow::bail!("SD API returned error: {}", resp.status());
        }
        
        let models: Vec<SDModel> = resp.json()
            .context("Failed to parse SD response")?;
            
        Ok(models.into_iter().map(|m| m.title).collect())
    }
    
    pub fn generate_text(&mut self, prompt: &str) -> Result<String> {
        let model = self.get_preferred_model()?
            .ok_or_else(|| anyhow::anyhow!("No Ollama models available"))?;
            
        let url = format!("{}/api/generate", self.ollama_url);
        let request = OllamaGenerateRequest {
            model,
            prompt: prompt.to_string(),
            stream: false,
        };
        
        let resp = self.client
            .post(&url)
            .json(&request)
            .send()
            .context("Failed to send request to Ollama")?;
            
        if !resp.status().is_success() {
            anyhow::bail!("Ollama API returned error: {}", resp.status());
        }
        
        let response: OllamaGenerateResponse = resp.json()
            .context("Failed to parse Ollama response")?;
            
        Ok(response.response)
    }
    
    pub fn generate_image(&mut self, prompt: &str) -> Result<ImageInfo> {
        let url = format!("{}/sdapi/v1/txt2img", self.sd_url);
        let request = SDTxt2ImgRequest {
            prompt: prompt.to_string(),
            negative_prompt: String::new(),
            steps: 20,
            width: 512,
            height: 512,
        };
        
        let resp = self.client
            .post(&url)
            .json(&request)
            .send()
            .context("Failed to send request to Stable Diffusion")?;
            
        if !resp.status().is_success() {
            anyhow::bail!("SD API returned error: {}", resp.status());
        }
        
        let response: SDTxt2ImgResponse = resp.json()
            .context("Failed to parse SD response")?;
            
        if response.images.is_empty() {
            anyhow::bail!("No images generated");
        }
        
        // Get the first image
        let image_b64 = &response.images[0];
        use base64::Engine;
        let image_data = base64::engine::general_purpose::STANDARD
            .decode(image_b64)
            .context("Failed to decode image data")?;
            
        // Calculate SHA256 hash
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&image_data);
        let hash = format!("{:x}", hasher.finalize());
        
        Ok(ImageInfo {
            hash,
            size: image_data.len(),
        })
    }
}