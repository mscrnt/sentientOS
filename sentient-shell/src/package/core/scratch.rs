use anyhow::Result;
use std::collections::HashMap;
use super::{CorePackage, PackageCategory};

pub struct Scratch;

impl CorePackage for Scratch {
    fn name(&self) -> &'static str { "scratch" }
    fn version(&self) -> &'static str { "1.0.0" }
    fn description(&self) -> &'static str { "Quick buffer for text/code snippets" }
    fn category(&self) -> PackageCategory { PackageCategory::Utils }
}

// Global scratch storage (in-memory)
use std::sync::Mutex;
lazy_static::lazy_static! {
    static ref SCRATCHES: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}

pub fn run(args: &[&str]) -> Result<String> {
    if args.is_empty() {
        return Ok(help());
    }
    
    match args[0] {
        "new" | "add" => {
            if args.len() < 3 {
                return Ok("Usage: scratch new <name> <content>".to_string());
            }
            let name = args[1];
            let content = args[2..].join(" ");
            add_scratch(name, &content)
        },
        "get" | "show" => {
            if args.len() < 2 {
                return Ok("Usage: scratch get <name>".to_string());
            }
            get_scratch(args[1])
        },
        "list" | "ls" => list_scratches(),
        "rm" | "remove" => {
            if args.len() < 2 {
                return Ok("Usage: scratch rm <name>".to_string());
            }
            remove_scratch(args[1])
        },
        "append" => {
            if args.len() < 3 {
                return Ok("Usage: scratch append <name> <content>".to_string());
            }
            let name = args[1];
            let content = args[2..].join(" ");
            append_scratch(name, &content)
        },
        _ => Ok(help()),
    }
}

fn help() -> String {
    "Scratch - Quick buffer for text/code snippets\n\
     Commands:\n\
     scratch new <name> <content>    - Create a new scratch\n\
     scratch get <name>              - Show scratch content\n\
     scratch list                    - List all scratches\n\
     scratch rm <name>               - Remove a scratch\n\
     scratch append <name> <content> - Append to existing scratch".to_string()
}

fn add_scratch(name: &str, content: &str) -> Result<String> {
    let mut scratches = SCRATCHES.lock().unwrap();
    scratches.insert(name.to_string(), content.to_string());
    Ok(format!("Created scratch '{}'", name))
}

fn get_scratch(name: &str) -> Result<String> {
    let scratches = SCRATCHES.lock().unwrap();
    match scratches.get(name) {
        Some(content) => Ok(format!("=== {} ===\n{}", name, content)),
        None => Err(anyhow::anyhow!("Scratch '{}' not found", name)),
    }
}

fn list_scratches() -> Result<String> {
    let scratches = SCRATCHES.lock().unwrap();
    
    if scratches.is_empty() {
        return Ok("No scratches yet. Create one with 'scratch new <name> <content>'".to_string());
    }
    
    let mut output = String::from("Scratches:\n");
    for (name, content) in scratches.iter() {
        let preview = if content.len() > 50 {
            format!("{}...", &content[..50])
        } else {
            content.clone()
        };
        output.push_str(&format!("  {} - {}\n", name, preview));
    }
    
    Ok(output)
}

fn remove_scratch(name: &str) -> Result<String> {
    let mut scratches = SCRATCHES.lock().unwrap();
    match scratches.remove(name) {
        Some(_) => Ok(format!("Removed scratch '{}'", name)),
        None => Err(anyhow::anyhow!("Scratch '{}' not found", name)),
    }
}

fn append_scratch(name: &str, content: &str) -> Result<String> {
    let mut scratches = SCRATCHES.lock().unwrap();
    match scratches.get_mut(name) {
        Some(existing) => {
            existing.push('\n');
            existing.push_str(content);
            Ok(format!("Appended to scratch '{}'", name))
        },
        None => Err(anyhow::anyhow!("Scratch '{}' not found", name)),
    }
}