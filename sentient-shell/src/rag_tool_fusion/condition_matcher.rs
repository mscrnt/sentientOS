use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::collections::HashMap;
use regex::Regex;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCondition {
    pub name: String,
    pub description: String,
    pub pattern: ConditionPattern,
    pub tool: String,
    pub args: serde_json::Value,
    pub confirm: Option<bool>,
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ConditionPattern {
    Contains { keywords: Vec<String> },
    Regex { pattern: String },
    Numeric { field: String, operator: String, value: f64 },
    Combined { conditions: Vec<ConditionPattern>, operator: String }, // "and" or "or"
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConditionConfig {
    pub conditions: Vec<ToolCondition>,
}

pub struct ConditionMatcher {
    conditions: Vec<ToolCondition>,
    regex_cache: HashMap<String, Regex>,
}

impl ConditionMatcher {
    pub async fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .await
            .context("Failed to read conditions file")?;
        
        let config: ConditionConfig = if path.extension().map(|e| e == "yaml").unwrap_or(false) {
            serde_yaml::from_str(&content)?
        } else {
            serde_json::from_str(&content)?
        };
        
        let mut matcher = Self {
            conditions: config.conditions,
            regex_cache: HashMap::new(),
        };
        
        // Pre-compile regex patterns
        matcher.compile_regex_patterns()?;
        
        Ok(matcher)
    }
    
    pub async fn evaluate(&self, text: &str) -> Result<Vec<&ToolCondition>> {
        let mut matched_conditions = Vec::new();
        
        for condition in &self.conditions {
            if self.evaluate_pattern(&condition.pattern, text)? {
                matched_conditions.push(condition);
            }
        }
        
        // Sort by priority (higher priority first)
        matched_conditions.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        Ok(matched_conditions)
    }
    
    fn evaluate_pattern(&self, pattern: &ConditionPattern, text: &str) -> Result<bool> {
        match pattern {
            ConditionPattern::Contains { keywords } => {
                let text_lower = text.to_lowercase();
                Ok(keywords.iter().any(|k| text_lower.contains(&k.to_lowercase())))
            }
            
            ConditionPattern::Regex { pattern } => {
                if let Some(regex) = self.regex_cache.get(pattern) {
                    Ok(regex.is_match(text))
                } else {
                    Err(anyhow::anyhow!("Regex pattern not compiled: {}", pattern))
                }
            }
            
            ConditionPattern::Numeric { field, operator, value } => {
                // Extract numeric value from text based on field
                if let Some(extracted_value) = self.extract_numeric_value(text, field) {
                    match operator.as_str() {
                        ">" => Ok(extracted_value > *value),
                        ">=" => Ok(extracted_value >= *value),
                        "<" => Ok(extracted_value < *value),
                        "<=" => Ok(extracted_value <= *value),
                        "==" => Ok((extracted_value - *value).abs() < f64::EPSILON),
                        "!=" => Ok((extracted_value - *value).abs() >= f64::EPSILON),
                        _ => Err(anyhow::anyhow!("Unknown operator: {}", operator)),
                    }
                } else {
                    Ok(false)
                }
            }
            
            ConditionPattern::Combined { conditions, operator } => {
                let results: Result<Vec<bool>> = conditions
                    .iter()
                    .map(|c| self.evaluate_pattern(c, text))
                    .collect();
                
                let results = results?;
                
                match operator.as_str() {
                    "and" => Ok(results.iter().all(|&r| r)),
                    "or" => Ok(results.iter().any(|&r| r)),
                    _ => Err(anyhow::anyhow!("Unknown logical operator: {}", operator)),
                }
            }
        }
    }
    
    fn compile_regex_patterns(&mut self) -> Result<()> {
        for condition in &self.conditions {
            self.compile_pattern_regex(&condition.pattern)?;
        }
        Ok(())
    }
    
    fn compile_pattern_regex(&mut self, pattern: &ConditionPattern) -> Result<()> {
        match pattern {
            ConditionPattern::Regex { pattern: regex_str } => {
                if !self.regex_cache.contains_key(regex_str) {
                    let regex = Regex::new(regex_str)
                        .context(format!("Failed to compile regex: {}", regex_str))?;
                    self.regex_cache.insert(regex_str.clone(), regex);
                }
            }
            ConditionPattern::Combined { conditions, .. } => {
                for condition in conditions {
                    self.compile_pattern_regex(condition)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    fn extract_numeric_value(&self, text: &str, field: &str) -> Option<f64> {
        // Simple extraction based on common patterns
        let patterns = vec![
            (r"memory.*?(\d+(?:\.\d+)?)\s*%", "memory_percent"),
            (r"disk.*?(\d+(?:\.\d+)?)\s*%", "disk_percent"),
            (r"cpu.*?(\d+(?:\.\d+)?)\s*%", "cpu_percent"),
            (r"temperature.*?(\d+(?:\.\d+)?)", "temperature"),
            (r"(\d+(?:\.\d+)?)\s*GB\s*free", "free_gb"),
            (r"(\d+(?:\.\d+)?)\s*MB\s*free", "free_mb"),
        ];
        
        for (pattern_str, field_name) in patterns {
            if field == field_name {
                if let Ok(regex) = Regex::new(pattern_str) {
                    if let Some(caps) = regex.captures(text) {
                        if let Some(num_str) = caps.get(1) {
                            if let Ok(value) = num_str.as_str().parse::<f64>() {
                                return Some(value);
                            }
                        }
                    }
                }
            }
        }
        
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio;
    
    #[tokio::test]
    async fn test_condition_matching() {
        let config = ConditionConfig {
            conditions: vec![
                ToolCondition {
                    name: "high_memory".to_string(),
                    description: "Trigger when memory usage is high".to_string(),
                    pattern: ConditionPattern::Numeric {
                        field: "memory_percent".to_string(),
                        operator: ">".to_string(),
                        value: 90.0,
                    },
                    tool: "clean_cache".to_string(),
                    args: serde_json::json!({}),
                    confirm: Some(true),
                    priority: 10,
                },
                ToolCondition {
                    name: "disk_check".to_string(),
                    description: "Check disk when mentioned".to_string(),
                    pattern: ConditionPattern::Contains {
                        keywords: vec!["disk space".to_string(), "storage".to_string()],
                    },
                    tool: "disk_info".to_string(),
                    args: serde_json::json!({"verbose": true}),
                    confirm: None,
                    priority: 5,
                },
            ],
        };
        
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("conditions.json");
        tokio::fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap())
            .await
            .unwrap();
        
        let matcher = ConditionMatcher::load(&config_path).await.unwrap();
        
        // Test high memory condition
        let matches = matcher.evaluate("System memory usage is at 95%").await.unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "high_memory");
        
        // Test disk check condition
        let matches = matcher.evaluate("Check disk space availability").await.unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "disk_check");
        
        // Test no matches
        let matches = matcher.evaluate("Everything is running smoothly").await.unwrap();
        assert_eq!(matches.len(), 0);
    }
}