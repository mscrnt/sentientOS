use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use super::{CorePackage, PackageCategory};

pub struct Todo;

impl CorePackage for Todo {
    fn name(&self) -> &'static str { "todo" }
    fn version(&self) -> &'static str { "1.0.0" }
    fn description(&self) -> &'static str { "AI-powered task manager with priority suggestions" }
    fn category(&self) -> PackageCategory { PackageCategory::Utils }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TodoItem {
    pub id: usize,
    pub task: String,
    pub priority: Priority,
    pub completed: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum Priority {
    High,
    Medium,
    Low,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::High => write!(f, "HIGH"),
            Priority::Medium => write!(f, "MED"),
            Priority::Low => write!(f, "LOW"),
        }
    }
}

// Global todo storage (in-memory for now)
use std::sync::Mutex;
lazy_static::lazy_static! {
    static ref TODOS: Mutex<HashMap<usize, TodoItem>> = Mutex::new(HashMap::new());
    static ref NEXT_ID: Mutex<usize> = Mutex::new(1);
}

pub fn run(args: &[&str]) -> Result<String> {
    if args.is_empty() {
        return Ok(help());
    }

    match args[0] {
        "add" => {
            if args.len() < 2 {
                return Ok("Usage: todo add <task>".to_string());
            }
            let task = args[1..].join(" ");
            add_todo(&task)
        },
        "list" | "ls" => list_todos(),
        "done" => {
            if args.len() < 2 {
                return Ok("Usage: todo done <id>".to_string());
            }
            let id: usize = args[1].parse()
                .map_err(|_| anyhow::anyhow!("Invalid todo ID"))?;
            mark_done(id)
        },
        "rm" | "remove" => {
            if args.len() < 2 {
                return Ok("Usage: todo rm <id>".to_string());
            }
            let id: usize = args[1].parse()
                .map_err(|_| anyhow::anyhow!("Invalid todo ID"))?;
            remove_todo(id)
        },
        "tag" => {
            if args.len() < 3 {
                return Ok("Usage: todo tag <id> <tag>".to_string());
            }
            let id: usize = args[1].parse()
                .map_err(|_| anyhow::anyhow!("Invalid todo ID"))?;
            let tag = args[2];
            add_tag(id, tag)
        },
        _ => Ok(help()),
    }
}

fn help() -> String {
    "Todo - AI-powered task manager\n\
     Commands:\n\
     todo add <task>   - Add a new task\n\
     todo list         - List all tasks\n\
     todo done <id>    - Mark task as completed\n\
     todo rm <id>      - Remove a task\n\
     todo tag <id> <tag> - Add a tag to a task".to_string()
}

fn add_todo(task: &str) -> Result<String> {
    let mut todos = TODOS.lock().unwrap();
    let mut next_id = NEXT_ID.lock().unwrap();
    
    // Simple AI-like priority detection based on keywords
    let priority = detect_priority(task);
    
    let todo = TodoItem {
        id: *next_id,
        task: task.to_string(),
        priority,
        completed: false,
        tags: vec![],
    };
    
    todos.insert(*next_id, todo);
    *next_id += 1;
    
    Ok(format!("Added todo #{} with {} priority", *next_id - 1, priority))
}

fn detect_priority(task: &str) -> Priority {
    let task_lower = task.to_lowercase();
    
    // High priority keywords
    if task_lower.contains("urgent") || 
       task_lower.contains("critical") || 
       task_lower.contains("asap") ||
       task_lower.contains("immediately") {
        return Priority::High;
    }
    
    // Low priority keywords
    if task_lower.contains("maybe") || 
       task_lower.contains("someday") || 
       task_lower.contains("eventually") {
        return Priority::Low;
    }
    
    Priority::Medium
}

fn list_todos() -> Result<String> {
    let todos = TODOS.lock().unwrap();
    
    if todos.is_empty() {
        return Ok("No todos yet. Add one with 'todo add <task>'".to_string());
    }
    
    let mut output = String::from("Todo List:\n");
    let mut sorted_todos: Vec<_> = todos.values().collect();
    
    // Sort by priority (high first) and then by ID
    sorted_todos.sort_by_key(|t| (
        match t.priority {
            Priority::High => 0,
            Priority::Medium => 1,
            Priority::Low => 2,
        },
        t.id
    ));
    
    for todo in sorted_todos {
        let status = if todo.completed { "✓" } else { " " };
        let tags = if todo.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", todo.tags.join(", "))
        };
        
        output.push_str(&format!(
            "[{}] #{} ({}) {}{}\n",
            status, todo.id, todo.priority, todo.task, tags
        ));
    }
    
    Ok(output)
}

fn mark_done(id: usize) -> Result<String> {
    let mut todos = TODOS.lock().unwrap();
    
    match todos.get_mut(&id) {
        Some(todo) => {
            todo.completed = true;
            Ok(format!("Marked todo #{} as completed", id))
        },
        None => Err(anyhow::anyhow!("Todo #{} not found", id)),
    }
}

fn remove_todo(id: usize) -> Result<String> {
    let mut todos = TODOS.lock().unwrap();
    
    match todos.remove(&id) {
        Some(_) => Ok(format!("Removed todo #{}", id)),
        None => Err(anyhow::anyhow!("Todo #{} not found", id)),
    }
}

fn add_tag(id: usize, tag: &str) -> Result<String> {
    let mut todos = TODOS.lock().unwrap();
    
    match todos.get_mut(&id) {
        Some(todo) => {
            if !todo.tags.contains(&tag.to_string()) {
                todo.tags.push(tag.to_string());
            }
            Ok(format!("Added tag '{}' to todo #{}", tag, id))
        },
        None => Err(anyhow::anyhow!("Todo #{} not found", id)),
    }
}