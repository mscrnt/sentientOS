//! Test the AI stream parser for early command detection

use sentient_shell::ai_router::stream_parser::{StreamDetector, StreamDetection, CommandPrefix};

fn main() {
    println!("=== Testing AI Stream Parser ===\n");
    
    // Simulate streaming AI responses
    let test_streams = vec![
        // Validated command
        ("Here's how to list files: ", "!@ ls -la /home\n"),
        
        // Dangerous command
        ("To remove everything: ", "!# rm -rf /\n"),
        
        // System command
        ("Start the service: ", "!$ service nginx start\n"),
        
        // Background task
        ("Run backup: ", "!& backup_system.sh\n"),
        
        // Sandboxed execution
        ("Test script: ", "!~ python test.py\n"),
        
        // Normal text
        ("The answer is ", "42, as always.\n"),
    ];
    
    for (prefix_text, command_text) in test_streams {
        let mut detector = StreamDetector::new();
        
        println!("Stream: '{}{}'", prefix_text, command_text.trim());
        
        // Feed prefix text first
        match detector.feed(prefix_text) {
            StreamDetection::Partial => {
                println!("  -> Partial data received");
            }
            _ => {}
        }
        
        // Feed command text character by character to simulate streaming
        for ch in command_text.chars() {
            match detector.feed(&ch.to_string()) {
                StreamDetection::PrefixDetected(prefix) => {
                    println!("  🔍 Early detection: {} command incoming!", prefix.as_str());
                    
                    // OS can prepare resources here
                    match prefix {
                        CommandPrefix::Validated => {
                            println!("     → Loading schema validator...");
                        }
                        CommandPrefix::Dangerous => {
                            println!("     → Preparing confirmation dialog...");
                        }
                        CommandPrefix::System => {
                            println!("     → Checking system permissions...");
                        }
                        CommandPrefix::Background => {
                            println!("     → Initializing job queue...");
                        }
                        CommandPrefix::Sandboxed => {
                            println!("     → Creating sandbox environment...");
                        }
                        CommandPrefix::None => {}
                    }
                }
                
                StreamDetection::CommandComplete { prefix, command } => {
                    println!("  ✅ Complete: {} {}", prefix.as_str(), command.trim());
                }
                
                StreamDetection::Partial => {
                    // Keep accumulating
                }
            }
        }
        
        println!();
    }
    
    // Test chunked streaming
    println!("=== Testing Chunked Streaming ===\n");
    
    let mut detector = StreamDetector::new();
    let chunks = vec!["!", "@", " ec", "ho ", "hel", "lo\n"];
    
    for chunk in chunks {
        println!("Chunk: '{}'", chunk);
        match detector.feed(chunk) {
            StreamDetection::PrefixDetected(prefix) => {
                println!("  -> Detected prefix: {}", prefix.as_str());
            }
            StreamDetection::CommandComplete { prefix, command } => {
                println!("  -> Complete: {} {}", prefix.as_str(), command);
            }
            StreamDetection::Partial => {
                println!("  -> Accumulating...");
            }
        }
    }
}