//! Constraint definitions for field validation

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Constraints that can be applied to schema fields
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Constraint {
    /// Minimum string length
    MinLength(usize),
    
    /// Maximum string length
    MaxLength(usize),
    
    /// Minimum numeric value
    Min(i64),
    
    /// Maximum numeric value
    Max(i64),
    
    /// Regex pattern match
    Pattern(String),
    
    /// Value must be in this list
    OneOf(Vec<Value>),
    
    /// Custom constraint with description
    Custom(String, String), // (name, description)
}

impl Constraint {
    /// Validate a value against this constraint
    pub fn validate(&self, value: &Value) -> Result<(), String> {
        match self {
            Constraint::MinLength(min) => {
                if let Value::String(s) = value {
                    if s.len() < *min {
                        return Err(format!("length must be at least {}", min));
                    }
                }
                Ok(())
            }
            
            Constraint::MaxLength(max) => {
                if let Value::String(s) = value {
                    if s.len() > *max {
                        return Err(format!("length must be at most {}", max));
                    }
                }
                Ok(())
            }
            
            Constraint::Min(min) => {
                if let Value::Number(n) = value {
                    if let Some(v) = n.as_i64() {
                        if v < *min {
                            return Err(format!("value must be at least {}", min));
                        }
                    }
                }
                Ok(())
            }
            
            Constraint::Max(max) => {
                if let Value::Number(n) = value {
                    if let Some(v) = n.as_i64() {
                        if v > *max {
                            return Err(format!("value must be at most {}", max));
                        }
                    }
                }
                Ok(())
            }
            
            Constraint::Pattern(pattern) => {
                if let Value::String(s) = value {
                    // Simple pattern matching for now
                    // In production, use regex crate
                    if !s.contains(pattern) {
                        return Err(format!("must match pattern: {}", pattern));
                    }
                }
                Ok(())
            }
            
            Constraint::OneOf(options) => {
                if !options.contains(value) {
                    return Err(format!("must be one of: {:?}", options));
                }
                Ok(())
            }
            
            Constraint::Custom(name, desc) => {
                // Custom constraints would be implemented by the user
                Err(format!("custom constraint '{}' failed: {}", name, desc))
            }
        }
    }
}

/// Common constraint builders
pub fn min(value: i64) -> Constraint {
    Constraint::Min(value)
}

pub fn max(value: i64) -> Constraint {
    Constraint::Max(value)
}

pub fn min_length(len: usize) -> Constraint {
    Constraint::MinLength(len)
}

pub fn max_length(len: usize) -> Constraint {
    Constraint::MaxLength(len)
}

pub fn pattern(pat: &str) -> Constraint {
    Constraint::Pattern(pat.to_string())
}

pub fn one_of(values: Vec<Value>) -> Constraint {
    Constraint::OneOf(values)
}