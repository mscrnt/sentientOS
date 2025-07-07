use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::time::{Duration, Instant};
use uuid::Uuid;
use std::collections::HashMap;

use crate::ai_router::intelligent_router::IntelligentRouter;
use crate::rag::RAGConfig;
use super::types::{RagResponse, ToolExecution, RagSystem, ToolRegistry, ExecutionMode};
use super::condition_matcher::{ConditionMatcher, ToolCondition};
use super::trace_logger::{TraceLogger, ExecutionTrace, TraceEntry};
use crate::bindings::rl_policy::{SimplePythonRL, extract_state_from_prompt, RLDecision};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HybridIntent {
    PureQuery,           // Only needs RAG
    PureAction,          // Only needs tool execution
    QueryThenAction,     // RAG first, then tool based on result
    ActionThenQuery,     // Tool first, then RAG to explain
    ConditionalAction,   // Tool execution depends on RAG result
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPipeline {
    pub prompt: String,
    pub intent: HybridIntent,
    pub rag_response: Option<RagResponse>,
    pub tool_execution: Option<ToolExecution>,
    pub conditions_evaluated: Vec<String>,
    pub final_response: String,
    pub trace_id: String,
    pub duration_ms: u64,
}

pub struct RagToolRouter {
    router: IntelligentRouter,
    rag_system: RagSystem,
    tool_registry: ToolRegistry,
    condition_matcher: ConditionMatcher,
    trace_logger: TraceLogger,
    rl_policy: Option<SimplePythonRL>,
}

impl RagToolRouter {
    pub async fn new(
        router_config_path: &Path,
        rag_config: RAGConfig,
        tool_registry_path: &Path,
        conditions_path: &Path,
    ) -> Result<Self> {
        let router = IntelligentRouter::from_config(router_config_path)
            .await
            .context("Failed to initialize router")?;
        
        let rag_system = RagSystem::new(rag_config)
            .await
            .context("Failed to initialize RAG system")?;
        
        let tool_registry = ToolRegistry::load(tool_registry_path)
            .await
            .context("Failed to load tool registry")?;
        
        let condition_matcher = ConditionMatcher::load(conditions_path)
            .await
            .context("Failed to load condition matcher")?;
        
        let trace_logger = TraceLogger::new("logs/rl_trace.jsonl")
            .await
            .context("Failed to initialize trace logger")?;
        
        // Initialize RL policy (optional)
        let mut python_rl = SimplePythonRL::new();
        let rl_policy = match python_rl.initialize() {
            Ok(_) => {
                log::info!("✅ RL policy initialized successfully");
                Some(python_rl)
            }
            Err(e) => {
                log::warn!("⚠️ RL policy initialization failed: {}. Using heuristics.", e);
                None
            }
        };
        
        Ok(Self {
            router,
            rag_system,
            tool_registry,
            condition_matcher,
            trace_logger,
            rl_policy,
        })
    }
    
    pub async fn execute(&mut self, prompt: &str, explain: bool) -> Result<ExecutionPipeline> {
        let start = Instant::now();
        let trace_id = uuid::Uuid::new_v4().to_string();
        
        // Detect hybrid intent
        let intent = self.detect_hybrid_intent(prompt).await?;
        
        // Try RL-based routing
        let rl_decision = self.route_with_rl(prompt, &intent).await;
        
        if explain {
            println!("🔍 Detected intent: {:?}", intent);
            if let Some(ref decision) = rl_decision {
                println!("🤖 RL routing: model={}, rag={}, tool={:?}", 
                    decision.model, decision.use_rag, decision.tool);
            }
        }
        
        let mut pipeline = ExecutionPipeline {
            prompt: prompt.to_string(),
            intent: intent.clone(),
            rag_response: None,
            tool_execution: None,
            conditions_evaluated: Vec::new(),
            final_response: String::new(),
            trace_id: trace_id.clone(),
            duration_ms: 0,
        };
        
        // Execute based on intent
        match intent {
            HybridIntent::PureQuery => {
                let response = self.execute_pure_query(prompt, explain).await?;
                pipeline.rag_response = Some(response.clone());
                pipeline.final_response = response.answer;
            }
            
            HybridIntent::PureAction => {
                let execution = self.execute_pure_action(prompt, explain).await?;
                pipeline.tool_execution = Some(execution.clone());
                pipeline.final_response = format!("Tool executed: {}", execution.tool_name);
            }
            
            HybridIntent::QueryThenAction => {
                let (rag_resp, tool_exec, conditions) = self.execute_query_then_action(prompt, explain).await?;
                pipeline.rag_response = Some(rag_resp.clone());
                pipeline.tool_execution = tool_exec.clone();
                pipeline.conditions_evaluated = conditions;
                pipeline.final_response = self.format_hybrid_response(&rag_resp, &tool_exec);
            }
            
            HybridIntent::ActionThenQuery => {
                let (tool_exec, rag_resp) = self.execute_action_then_query(prompt, explain).await?;
                pipeline.tool_execution = Some(tool_exec.clone());
                pipeline.rag_response = Some(rag_resp.clone());
                pipeline.final_response = self.format_hybrid_response(&rag_resp, &Some(tool_exec));
            }
            
            HybridIntent::ConditionalAction => {
                let (rag_resp, tool_exec, conditions) = self.execute_conditional_action(prompt, explain).await?;
                pipeline.rag_response = Some(rag_resp.clone());
                pipeline.tool_execution = tool_exec.clone();
                pipeline.conditions_evaluated = conditions;
                pipeline.final_response = self.format_conditional_response(&rag_resp, &tool_exec, &pipeline.conditions_evaluated);
            }
        }
        
        pipeline.duration_ms = start.elapsed().as_millis() as u64;
        
        // Log execution trace
        let trace = self.create_trace_entry(&pipeline);
        self.trace_logger.log(trace).await?;
        
        Ok(pipeline)
    }
    
    async fn detect_hybrid_intent(&self, prompt: &str) -> Result<HybridIntent> {
        // Use the router to detect intent
        let route_result = self.router.route(prompt).await?;
        
        // Analyze prompt for action keywords
        let action_keywords = ["run", "execute", "check", "monitor", "clean", "fix", "show"];
        let query_keywords = ["what", "how", "why", "explain", "describe", "when"];
        let conditional_keywords = ["if", "when", "should", "could"];
        
        let prompt_lower = prompt.to_lowercase();
        let has_action = action_keywords.iter().any(|k| prompt_lower.contains(k));
        let has_query = query_keywords.iter().any(|k| prompt_lower.contains(k));
        let has_conditional = conditional_keywords.iter().any(|k| prompt_lower.contains(k));
        
        // Determine hybrid intent
        match (has_query, has_action, has_conditional) {
            (true, false, _) => Ok(HybridIntent::PureQuery),
            (false, true, false) => Ok(HybridIntent::PureAction),
            (true, true, false) => {
                // Determine order based on position
                let first_query_pos = query_keywords.iter()
                    .filter_map(|k| prompt_lower.find(k))
                    .min()
                    .unwrap_or(usize::MAX);
                let first_action_pos = action_keywords.iter()
                    .filter_map(|k| prompt_lower.find(k))
                    .min()
                    .unwrap_or(usize::MAX);
                
                if first_query_pos < first_action_pos {
                    Ok(HybridIntent::QueryThenAction)
                } else {
                    Ok(HybridIntent::ActionThenQuery)
                }
            }
            (_, _, true) => Ok(HybridIntent::ConditionalAction),
            _ => Ok(HybridIntent::PureQuery), // Default to query
        }
    }
    
    /// Use RL policy to make routing decision
    async fn route_with_rl(&self, prompt: &str, intent: &HybridIntent) -> Option<RLDecision> {
        if let Some(ref rl_policy) = self.rl_policy {
            match rl_policy.infer(prompt, &format!("{:?}", intent)) {
                Ok(decision) => {
                    log::info!("🤖 RL Decision: model={}, rag={}, tool={:?}, confidence={}",
                        decision.model, decision.use_rag, decision.tool, decision.confidence);
                    Some(decision)
                }
                Err(e) => {
                    log::warn!("RL inference failed: {}", e);
                    None
                }
            }
        } else {
            None
        }
    }
    
    async fn execute_pure_query(&mut self, prompt: &str, explain: bool) -> Result<RagResponse> {
        if explain {
            println!("📚 Executing RAG query...");
        }
        
        self.rag_system.query(prompt).await
    }
    
    async fn execute_pure_action(&mut self, prompt: &str, explain: bool) -> Result<ToolExecution> {
        if explain {
            println!("🔧 Executing tool action...");
        }
        
        // Extract tool name and arguments from prompt
        let (tool_name, args) = self.parse_tool_command(prompt)?;
        
        // Execute tool
        self.tool_registry.execute(
            &tool_name,
            args,
            ExecutionMode::Standard,
            None, // No confirmation for now
        ).await
    }
    
    async fn execute_query_then_action(
        &mut self,
        prompt: &str,
        explain: bool,
    ) -> Result<(RagResponse, Option<ToolExecution>, Vec<String>)> {
        // First, execute RAG query
        let rag_response = self.execute_pure_query(prompt, explain).await?;
        
        if explain {
            println!("📊 RAG Response: {}", rag_response.answer);
            println!("🔍 Checking conditions for tool execution...");
        }
        
        // Check conditions based on RAG response
        let conditions = self.condition_matcher.evaluate(&rag_response.answer).await?;
        let condition_names: Vec<String> = conditions.iter().map(|c| c.name.clone()).collect();
        
        // Execute tool if conditions are met
        let tool_execution = if let Some(condition) = conditions.first() {
            if explain {
                println!("✅ Condition '{}' matched, executing tool '{}'", condition.name, condition.tool);
            }
            
            let execution = self.tool_registry.execute(
                &condition.tool,
                condition.args.clone(),
                ExecutionMode::Standard,
                condition.confirm,
            ).await?;
            
            Some(execution)
        } else {
            if explain {
                println!("❌ No conditions matched, skipping tool execution");
            }
            None
        };
        
        Ok((rag_response, tool_execution, condition_names))
    }
    
    async fn execute_action_then_query(
        &mut self,
        prompt: &str,
        explain: bool,
    ) -> Result<(ToolExecution, RagResponse)> {
        // First, execute tool
        let tool_execution = self.execute_pure_action(prompt, explain).await?;
        
        // Then, use RAG to explain the result
        let explanation_prompt = format!(
            "Explain the result of running '{}' tool: {}",
            tool_execution.tool_name,
            tool_execution.output
        );
        
        if explain {
            println!("📚 Using RAG to explain tool result...");
        }
        
        let rag_response = self.rag_system.query(&explanation_prompt).await?;
        
        Ok((tool_execution, rag_response))
    }
    
    async fn execute_conditional_action(
        &mut self,
        prompt: &str,
        explain: bool,
    ) -> Result<(RagResponse, Option<ToolExecution>, Vec<String>)> {
        // Similar to query_then_action but with more sophisticated condition evaluation
        self.execute_query_then_action(prompt, explain).await
    }
    
    fn parse_tool_command(&self, prompt: &str) -> Result<(String, serde_json::Value)> {
        // Simple parser - in production, use more sophisticated NLP
        let words: Vec<&str> = prompt.split_whitespace().collect();
        
        // Look for tool keywords
        let tool_map = [
            ("disk", "disk_info"),
            ("memory", "memory_usage"),
            ("process", "process_list"),
            ("network", "network_status"),
            ("service", "service_manager"),
        ];
        
        for (keyword, tool_name) in &tool_map {
            if prompt.to_lowercase().contains(keyword) {
                return Ok((tool_name.to_string(), serde_json::json!({})));
            }
        }
        
        Err(anyhow::anyhow!("Could not parse tool command from prompt"))
    }
    
    fn format_hybrid_response(
        &self,
        rag_response: &RagResponse,
        tool_execution: &Option<ToolExecution>,
    ) -> String {
        let mut response = format!("📚 Knowledge: {}\n", rag_response.answer);
        
        if let Some(exec) = tool_execution {
            response.push_str(&format!("\n🔧 Action Result: {}\n", exec.output));
        }
        
        response
    }
    
    fn format_conditional_response(
        &self,
        rag_response: &RagResponse,
        tool_execution: &Option<ToolExecution>,
        conditions: &[String],
    ) -> String {
        let mut response = self.format_hybrid_response(rag_response, tool_execution);
        
        if !conditions.is_empty() {
            response.push_str(&format!("\n✅ Conditions Met: {}", conditions.join(", ")));
        }
        
        response
    }
    
    fn create_trace_entry(&self, pipeline: &ExecutionPipeline) -> TraceEntry {
        TraceEntry {
            trace_id: pipeline.trace_id.clone(),
            timestamp: chrono::Utc::now(),
            prompt: pipeline.prompt.clone(),
            intent: format!("{:?}", pipeline.intent),
            model_used: self.router.get_last_model_used().unwrap_or_default(),
            tool_executed: pipeline.tool_execution.as_ref().map(|e| e.tool_name.clone()),
            rag_used: pipeline.rag_response.is_some(),
            conditions_evaluated: pipeline.conditions_evaluated.clone(),
            success: pipeline.tool_execution.as_ref().map(|e| e.exit_code == 0).unwrap_or(true),
            duration_ms: pipeline.duration_ms,
            reward: None, // Will be set by feedback system
        }
    }
}