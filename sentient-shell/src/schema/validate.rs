//! Core validation trait and error types

use thiserror::Error;
use std::collections::HashMap;
use serde_json::Value;

/// Validation errors that can occur during schema validation
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Field `{field}` is missing")]
    MissingField { field: String },
    
    #[error("Field `{field}` has wrong type: expected {expected}, got {actual}")]
    WrongType {
        field: String,
        expected: String,
        actual: String,
    },
    
    #[error("Field `{field}` failed constraint: {constraint}")]
    ConstraintViolation {
        field: String,
        constraint: String,
    },
    
    #[error("Multiple validation errors: {0:?}")]
    Multiple(Vec<ValidationError>),
    
    #[error("Custom validation error: {0}")]
    Custom(String),
}

/// Result type for validation operations
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Core validation trait that all validated types must implement
pub trait Validate {
    /// Validate the instance according to its schema
    fn validate(&self) -> ValidationResult<()>;
    
    /// Get the schema definition for this type
    fn schema() -> super::Schema;
}

/// Validator for dynamic JSON values
pub struct JsonValidator {
    schema: super::Schema,
}

impl JsonValidator {
    pub fn new(schema: super::Schema) -> Self {
        Self { schema }
    }
    
    /// Validate a JSON value against the schema
    pub fn validate_value(&self, value: &Value) -> ValidationResult<()> {
        match value {
            Value::Object(map) => self.validate_object(map),
            _ => Err(ValidationError::Custom("Expected JSON object".to_string())),
        }
    }
    
    fn validate_object(&self, map: &serde_json::Map<String, Value>) -> ValidationResult<()> {
        let mut errors = Vec::new();
        
        // Check required fields
        for field in &self.schema.fields {
            if field.required && !map.contains_key(&field.name) {
                errors.push(ValidationError::MissingField {
                    field: field.name.clone(),
                });
                continue;
            }
            
            if let Some(value) = map.get(&field.name) {
                if let Err(e) = self.validate_field(field, value) {
                    errors.push(e);
                }
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else if errors.len() == 1 {
            Err(errors.into_iter().next().unwrap())
        } else {
            Err(ValidationError::Multiple(errors))
        }
    }
    
    fn validate_field(&self, field: &super::SchemaField, value: &Value) -> ValidationResult<()> {
        use super::FieldType;
        
        // Type checking
        let type_valid = match (&field.field_type, value) {
            (FieldType::String, Value::String(_)) => true,
            (FieldType::Integer, Value::Number(n)) => n.is_i64() || n.is_u64(),
            (FieldType::Float, Value::Number(_)) => true,
            (FieldType::Boolean, Value::Bool(_)) => true,
            (FieldType::Array(_), Value::Array(_)) => true,
            (FieldType::Object, Value::Object(_)) => true,
            _ => false,
        };
        
        if !type_valid {
            return Err(ValidationError::WrongType {
                field: field.name.clone(),
                expected: format!("{:?}", field.field_type),
                actual: format!("{}", value),
            });
        }
        
        // Apply constraints
        for constraint in &field.constraints {
            if let Err(msg) = constraint.validate(value) {
                return Err(ValidationError::ConstraintViolation {
                    field: field.name.clone(),
                    constraint: msg,
                });
            }
        }
        
        Ok(())
    }
}

/// Builder for validation errors
pub struct ValidationErrorBuilder {
    errors: Vec<ValidationError>,
}

impl ValidationErrorBuilder {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }
    
    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
    }
    
    pub fn add_missing_field(&mut self, field: &str) {
        self.add_error(ValidationError::MissingField {
            field: field.to_string(),
        });
    }
    
    pub fn add_constraint_violation(&mut self, field: &str, constraint: &str) {
        self.add_error(ValidationError::ConstraintViolation {
            field: field.to_string(),
            constraint: constraint.to_string(),
        });
    }
    
    pub fn build(self) -> Option<ValidationError> {
        match self.errors.len() {
            0 => None,
            1 => Some(self.errors.into_iter().next().unwrap()),
            _ => Some(ValidationError::Multiple(self.errors)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_validation_error_builder() {
        let mut builder = ValidationErrorBuilder::new();
        builder.add_missing_field("name");
        builder.add_constraint_violation("age", "must be >= 18");
        
        let error = builder.build().unwrap();
        match error {
            ValidationError::Multiple(errors) => assert_eq!(errors.len(), 2),
            _ => panic!("Expected multiple errors"),
        }
    }
}