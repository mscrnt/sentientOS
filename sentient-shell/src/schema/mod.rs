//! SentientOS Schema Validation Framework
//! 
//! A lightweight validation system for structured data exchange between
//! OS components, AI models, and shell commands.

pub mod validate;
pub mod schema;
pub mod macros;
pub mod constraints;
pub mod types;

pub use validate::{Validate, ValidationError, ValidationResult};
pub use schema::{Schema, SchemaField, FieldType};
pub use constraints::Constraint;

// Re-export for convenience
pub use serde::{Deserialize, Serialize};

/// Prelude module for common imports
pub mod prelude {
    pub use super::{Validate, ValidationError, ValidationResult};
    pub use super::macros::*;
    pub use super::constraints::*;
}