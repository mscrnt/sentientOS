//! Example of using sentient-schema for validation

use sentient_schema::prelude::*;
use sentient_schema::{SchemaBuilder, schema, sentient_struct};
use serde_json::json;

// Define a user profile with validation
sentient_struct! {
    UserProfile {
        name: String => min_length: 1, max_length: 100,
        age: u8 => min: 18, max: 120,
        email: String => min_length: 5,
        is_developer: bool => default: true,
    }
}

fn main() {
    println!("=== Sentient Schema Validation Examples ===\n");
    
    // Example 1: Programmatic schema building
    let user_schema = SchemaBuilder::new("User")
        .description("User registration schema")
        .string_field("username")
            .min_length(3)
            .max_length(20)
            .pattern("[a-zA-Z0-9_]+")
            .and()
        .integer_field("age")
            .min(13)
            .max(150)
            .and()
        .string_field("email")
            .pattern(".*@.*")
            .and()
        .boolean_field("terms_accepted")
            .required(true)
            .and()
        .build();
    
    // Test valid data
    let valid_user = json!({
        "username": "alice_doe",
        "age": 25,
        "email": "alice@example.com",
        "terms_accepted": true
    });
    
    match user_schema.validate(&valid_user) {
        Ok(_) => println!("✅ Valid user data accepted"),
        Err(e) => println!("❌ Validation failed: {}", e),
    }
    
    // Test invalid data
    let invalid_user = json!({
        "username": "a",  // Too short
        "age": 200,       // Too old
        "email": "not-an-email",
        "terms_accepted": false
    });
    
    match user_schema.validate(&invalid_user) {
        Ok(_) => println!("✅ Valid user data accepted"),
        Err(e) => println!("❌ Validation failed: {}", e),
    }
    
    // Example 2: Using the macro-generated struct
    let profile = UserProfile {
        name: "Bob Smith".to_string(),
        age: 25,
        email: "bob@example.com".to_string(),
        is_developer: true,
    };
    
    match profile.validate() {
        Ok(_) => println!("\n✅ Valid profile created"),
        Err(e) => println!("\n❌ Profile validation failed: {}", e),
    }
    
    // Test invalid profile
    let invalid_profile = UserProfile {
        name: "".to_string(),  // Empty name
        age: 16,               // Too young
        email: "bob".to_string(),
        is_developer: false,
    };
    
    match invalid_profile.validate() {
        Ok(_) => println!("✅ Valid profile created"),
        Err(e) => println!("❌ Profile validation failed: {}", e),
    }
    
    // Example 3: Command validation schema
    let command_schema = schema!("ShellCommand" => {
        command: String [min_length = 1, pattern = "^[a-zA-Z0-9_-]+$"],
        args: String [max_length = 1000],
        dangerous: Boolean,
    });
    
    println!("\n=== Command Validation ===");
    
    let safe_cmd = json!({
        "command": "ls",
        "args": "-la /home",
        "dangerous": false
    });
    
    match command_schema.validate(&safe_cmd) {
        Ok(_) => println!("✅ Safe command validated"),
        Err(e) => println!("❌ Command validation failed: {}", e),
    }
    
    let dangerous_cmd = json!({
        "command": "rm",
        "args": "-rf /",
        "dangerous": true
    });
    
    match command_schema.validate(&dangerous_cmd) {
        Ok(_) => println!("⚠️  Dangerous command validated (requires confirmation)"),
        Err(e) => println!("❌ Command validation failed: {}", e),
    }
}