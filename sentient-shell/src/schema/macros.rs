//! Declarative macros for schema definition

/// Create a validated struct with schema
/// 
/// Example:
/// ```
/// sentient_struct! {
///     UserProfile {
///         name: String => min_length: 1, max_length: 100,
///         age: u8 => min: 18, max: 120,
///         email: String => pattern: EMAIL,
///         is_developer: bool => default: true,
///     }
/// }
/// ```
#[macro_export]
macro_rules! sentient_struct {
    (
        $name:ident {
            $(
                $field:ident : $ftype:ty $(=> $($constraint:ident : $value:expr),* )?
            ),* $(,)?
        }
    ) => {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct $name {
            $(pub $field: $ftype,)*
        }
        
        impl $super::Validate for $name {
            fn validate(&self) -> $super::ValidationResult<()> {
                let mut builder = $super::validate::ValidationErrorBuilder::new();
                
                // Validate each field
                $(
                    sentient_struct!(@validate_field builder, self, $field, $ftype $(, $($constraint : $value),*)?);
                )*
                
                if let Some(error) = builder.build() {
                    Err(error)
                } else {
                    Ok(())
                }
            }
            
            fn schema() -> $super::Schema {
                use $super::schema::{Schema, SchemaField, FieldType};
                use $super::constraints::Constraint;
                
                let mut schema = Schema::new(stringify!($name));
                
                $(
                    let mut field = SchemaField::new(
                        stringify!($field),
                        sentient_struct!(@field_type $ftype)
                    );
                    
                    $($(
                        field = field.constraint(sentient_struct!(@constraint $constraint : $value));
                    )*)?
                    
                    schema.fields.push(field);
                )*
                
                schema
            }
        }
    };
    
    // Helper: validate a field
    (@validate_field $builder:ident, $self:ident, $field:ident, $ftype:ty) => {
        // Basic validation - just check the field exists
    };
    
    (@validate_field $builder:ident, $self:ident, $field:ident, $ftype:ty, $($constraint:ident : $value:expr),*) => {
        $(
            sentient_struct!(@apply_constraint $builder, $self.$field, stringify!($field), $constraint : $value);
        )*
    };
    
    // Helper: apply constraint
    (@apply_constraint $builder:ident, $field_value:expr, $field_name:expr, min : $value:expr) => {
        if $field_value < $value {
            $builder.add_constraint_violation($field_name, &format!("must be at least {}", $value));
        }
    };
    
    (@apply_constraint $builder:ident, $field_value:expr, $field_name:expr, max : $value:expr) => {
        if $field_value > $value {
            $builder.add_constraint_violation($field_name, &format!("must be at most {}", $value));
        }
    };
    
    (@apply_constraint $builder:ident, $field_value:expr, $field_name:expr, min_length : $value:expr) => {
        if $field_value.len() < $value {
            $builder.add_constraint_violation($field_name, &format!("length must be at least {}", $value));
        }
    };
    
    (@apply_constraint $builder:ident, $field_value:expr, $field_name:expr, max_length : $value:expr) => {
        if $field_value.len() > $value {
            $builder.add_constraint_violation($field_name, &format!("length must be at most {}", $value));
        }
    };
    
    (@apply_constraint $builder:ident, $field_value:expr, $field_name:expr, pattern : $value:expr) => {
        // Pattern matching would go here
    };
    
    (@apply_constraint $builder:ident, $field_value:expr, $field_name:expr, default : $value:expr) => {
        // Default values don't need validation
    };
    
    // Helper: map Rust type to FieldType
    (@field_type String) => { FieldType::String };
    (@field_type &str) => { FieldType::String };
    (@field_type bool) => { FieldType::Boolean };
    (@field_type u8) => { FieldType::Integer };
    (@field_type u16) => { FieldType::Integer };
    (@field_type u32) => { FieldType::Integer };
    (@field_type u64) => { FieldType::Integer };
    (@field_type i8) => { FieldType::Integer };
    (@field_type i16) => { FieldType::Integer };
    (@field_type i32) => { FieldType::Integer };
    (@field_type i64) => { FieldType::Integer };
    (@field_type f32) => { FieldType::Float };
    (@field_type f64) => { FieldType::Float };
    (@field_type $other:ty) => { FieldType::Custom(stringify!($other).to_string()) };
    
    // Helper: create constraint
    (@constraint min : $value:expr) => {
        Constraint::Min($value as i64)
    };
    (@constraint max : $value:expr) => {
        Constraint::Max($value as i64)
    };
    (@constraint min_length : $value:expr) => {
        Constraint::MinLength($value)
    };
    (@constraint max_length : $value:expr) => {
        Constraint::MaxLength($value)
    };
    (@constraint pattern : $value:expr) => {
        Constraint::Pattern($value.to_string())
    };
    (@constraint default : $value:expr) => {
        Constraint::Custom("default".to_string(), stringify!($value).to_string())
    };
}

/// Quick validation helper
#[macro_export]
macro_rules! validate {
    ($value:expr) => {
        match $super::Validate::validate(&$value) {
            Ok(()) => Ok($value),
            Err(e) => Err(e),
        }
    };
}

/// Schema definition helper
#[macro_export]
macro_rules! schema {
    ($name:expr => {
        $($field:ident : $ftype:ident $([$($constraint:ident = $value:expr),*])?),* $(,)?
    }) => {{
        use $super::schema::{Schema, SchemaField, FieldType};
        use $super::constraints::*;
        
        let mut schema = Schema::new($name);
        
        $(
            let mut field = SchemaField::new(stringify!($field), FieldType::$ftype);
            
            $($(
                field = field.constraint(schema!(@constraint $constraint = $value));
            )*)?
            
            schema.fields.push(field);
        )*
        
        schema
    }};
    
    (@constraint min = $value:expr) => { min($value) };
    (@constraint max = $value:expr) => { max($value) };
    (@constraint min_length = $value:expr) => { min_length($value) };
    (@constraint max_length = $value:expr) => { max_length($value) };
    (@constraint pattern = $value:expr) => { pattern($value) };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_schema_macro() {
        let schema = schema!("TestSchema" => {
            name: String [min_length = 1, max_length = 50],
            age: Integer [min = 0, max = 150],
            active: Boolean,
        });
        
        assert_eq!(schema.name, "TestSchema");
        assert_eq!(schema.fields.len(), 3);
    }
}