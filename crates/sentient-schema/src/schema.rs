//! Schema definition types

use serde::{Deserialize, Serialize};
use crate::constraints::Constraint;

/// A complete schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub name: String,
    pub description: Option<String>,
    pub fields: Vec<SchemaField>,
}

/// A single field in a schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaField {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    pub default: Option<serde_json::Value>,
    pub description: Option<String>,
    pub constraints: Vec<Constraint>,
}

/// Supported field types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FieldType {
    String,
    Integer,
    Float,
    Boolean,
    Array(Box<FieldType>),
    Object,
    Optional(Box<FieldType>),
    Custom(String),
}

impl Schema {
    /// Create a new schema
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            fields: Vec::new(),
        }
    }
    
    /// Add a field to the schema
    pub fn field(mut self, field: SchemaField) -> Self {
        self.fields.push(field);
        self
    }
    
    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
    
    /// Validate that a JSON value conforms to this schema
    pub fn validate(&self, value: &serde_json::Value) -> crate::ValidationResult<()> {
        let validator = crate::validate::JsonValidator::new(self.clone());
        validator.validate_value(value)
    }
}

impl SchemaField {
    /// Create a new field
    pub fn new(name: impl Into<String>, field_type: FieldType) -> Self {
        Self {
            name: name.into(),
            field_type,
            required: true,
            default: None,
            description: None,
            constraints: Vec::new(),
        }
    }
    
    /// Make field optional
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }
    
    /// Set default value
    pub fn default(mut self, value: serde_json::Value) -> Self {
        self.default = Some(value);
        self.required = false;
        self
    }
    
    /// Add a constraint
    pub fn constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }
    
    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Builder for creating schemas fluently
pub struct SchemaBuilder {
    schema: Schema,
}

impl SchemaBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            schema: Schema::new(name),
        }
    }
    
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.schema.description = Some(desc.into());
        self
    }
    
    pub fn string_field(self, name: impl Into<String>) -> FieldBuilder {
        FieldBuilder::new(self, name.into(), FieldType::String)
    }
    
    pub fn integer_field(self, name: impl Into<String>) -> FieldBuilder {
        FieldBuilder::new(self, name.into(), FieldType::Integer)
    }
    
    pub fn boolean_field(self, name: impl Into<String>) -> FieldBuilder {
        FieldBuilder::new(self, name.into(), FieldType::Boolean)
    }
    
    pub fn build(self) -> Schema {
        self.schema
    }
}

/// Builder for creating fields
pub struct FieldBuilder {
    builder: SchemaBuilder,
    field: SchemaField,
}

impl FieldBuilder {
    fn new(builder: SchemaBuilder, name: String, field_type: FieldType) -> Self {
        Self {
            builder,
            field: SchemaField::new(name, field_type),
        }
    }
    
    pub fn required(mut self, required: bool) -> Self {
        self.field.required = required;
        self
    }
    
    pub fn min_length(mut self, min: usize) -> Self {
        self.field.constraints.push(Constraint::MinLength(min));
        self
    }
    
    pub fn max_length(mut self, max: usize) -> Self {
        self.field.constraints.push(Constraint::MaxLength(max));
        self
    }
    
    pub fn min(mut self, min: i64) -> Self {
        self.field.constraints.push(Constraint::Min(min));
        self
    }
    
    pub fn max(mut self, max: i64) -> Self {
        self.field.constraints.push(Constraint::Max(max));
        self
    }
    
    pub fn pattern(mut self, pattern: &str) -> Self {
        self.field.constraints.push(Constraint::Pattern(pattern.to_string()));
        self
    }
    
    pub fn default_value(mut self, value: serde_json::Value) -> Self {
        self.field = self.field.default(value);
        self
    }
    
    pub fn and(mut self) -> SchemaBuilder {
        self.builder.schema.fields.push(self.field);
        self.builder
    }
    
    pub fn build(mut self) -> Schema {
        self.builder.schema.fields.push(self.field);
        self.builder.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_schema_builder() {
        let schema = SchemaBuilder::new("User")
            .description("User profile schema")
            .string_field("name")
                .min_length(1)
                .max_length(100)
                .and()
            .integer_field("age")
                .min(18)
                .max(120)
                .and()
            .boolean_field("is_developer")
                .default_value(json!(true))
                .and()
            .build();
        
        assert_eq!(schema.name, "User");
        assert_eq!(schema.fields.len(), 3);
        assert_eq!(schema.fields[0].name, "name");
        assert_eq!(schema.fields[1].name, "age");
        assert_eq!(schema.fields[2].name, "is_developer");
        assert!(!schema.fields[2].required);
    }
}