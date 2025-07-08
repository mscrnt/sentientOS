// SentientOS Native Services
// These services provide core AI and goal processing functionality

pub mod activity_loop;
pub mod goal_processor;
pub mod llm_observer;
pub mod reflective_analyzer;

use anyhow::Result;
use async_trait::async_trait;

/// Trait for all SentientOS services
#[async_trait]
pub trait SentientService: Send + Sync {
    /// Service name
    fn name(&self) -> &str;
    
    /// Initialize the service
    async fn init(&mut self) -> Result<()>;
    
    /// Run the service main loop
    async fn run(&mut self) -> Result<()>;
    
    /// Shutdown the service gracefully
    async fn shutdown(&mut self) -> Result<()>;
}

/// Service runner that manages service lifecycle
pub struct ServiceRunner;

impl ServiceRunner {
    pub async fn run_service(name: &str) -> Result<()> {
        match name {
            "activity-loop" => {
                let mut service = activity_loop::ActivityLoopService::new();
                service.init().await?;
                service.run().await
            }
            "goal-processor" => {
                let mut service = goal_processor::GoalProcessorService::new();
                service.init().await?;
                service.run().await
            }
            "llm-observer" => {
                let mut service = llm_observer::LlmObserverService::new();
                service.init().await?;
                service.run().await
            }
            "reflective-analyzer" => {
                let mut service = reflective_analyzer::ReflectiveAnalyzerService::new();
                service.init().await?;
                service.run().await
            }
            _ => Err(anyhow::anyhow!("Unknown service: {}", name))
        }
    }
}