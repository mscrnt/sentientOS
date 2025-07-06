//! Stream parser for early command detection in AI responses
//! Enables the OS to prepare resources while LLM is still streaming

use std::collections::VecDeque;
use serde::{Deserialize, Serialize};

/// Command prefix types for early detection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CommandPrefix {
    /// !@ - Validated execution (requires schema validation)
    Validated,
    
    /// !# - Dangerous operation (requires confirmation)
    Dangerous,
    
    /// !$ - System command (elevated privileges)
    System,
    
    /// !& - Background execution
    Background,
    
    /// !~ - Sandboxed execution
    Sandboxed,
    
    /// No prefix - normal execution
    None,
}

impl CommandPrefix {
    /// Parse prefix from the start of a string
    pub fn parse(s: &str) -> (Self, &str) {
        if s.starts_with("!@") {
            (Self::Validated, &s[2..])
        } else if s.starts_with("!#") {
            (Self::Dangerous, &s[2..])
        } else if s.starts_with("!$") {
            (Self::System, &s[2..])
        } else if s.starts_with("!&") {
            (Self::Background, &s[2..])
        } else if s.starts_with("!~") {
            (Self::Sandboxed, &s[2..])
        } else {
            (Self::None, s)
        }
    }
    
    /// Get prefix string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Validated => "!@",
            Self::Dangerous => "!#",
            Self::System => "!$",
            Self::Background => "!&",
            Self::Sandboxed => "!~",
            Self::None => "",
        }
    }
}

/// Early detection state for streaming responses
#[derive(Debug)]
pub struct StreamDetector {
    /// Buffer for incomplete data
    buffer: VecDeque<char>,
    
    /// Detected prefix (if any)
    prefix: Option<CommandPrefix>,
    
    /// Whether we've detected the command start
    command_started: bool,
    
    /// Accumulated command text
    command_buffer: String,
}

impl StreamDetector {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::with_capacity(8),
            prefix: None,
            command_started: false,
            command_buffer: String::new(),
        }
    }
    
    /// Feed a chunk of streaming data
    pub fn feed(&mut self, chunk: &str) -> StreamDetection {
        // If we haven't detected a prefix yet, look for one
        if self.prefix.is_none() && !self.command_started {
            for ch in chunk.chars() {
                self.buffer.push_back(ch);
                
                // Check if we have enough characters to detect a prefix
                if self.buffer.len() >= 2 {
                    let first_two: String = self.buffer.iter().take(2).collect();
                    
                    match first_two.as_str() {
                        "!@" | "!#" | "!$" | "!&" | "!~" => {
                            let (prefix, _) = CommandPrefix::parse(&first_two);
                            self.prefix = Some(prefix.clone());
                            self.command_started = true;
                            self.buffer.clear();
                            
                            // Return early detection
                            return StreamDetection::PrefixDetected(prefix);
                        }
                        _ => {
                            // Not a prefix, might be regular text
                            if self.buffer.len() > 2 {
                                self.command_started = true;
                                self.prefix = Some(CommandPrefix::None);
                            }
                        }
                    }
                }
            }
        }
        
        // Accumulate command text
        if self.command_started {
            self.command_buffer.push_str(chunk);
            
            // Check for command boundaries (newline, etc)
            if chunk.contains('\n') {
                let command = self.command_buffer.trim().to_string();
                self.command_buffer.clear();
                
                return StreamDetection::CommandComplete {
                    prefix: self.prefix.clone().unwrap_or(CommandPrefix::None),
                    command,
                };
            }
        }
        
        StreamDetection::Partial
    }
    
    /// Reset the detector
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.prefix = None;
        self.command_started = false;
        self.command_buffer.clear();
    }
}

/// Result of stream detection
#[derive(Debug)]
pub enum StreamDetection {
    /// Detected a command prefix early
    PrefixDetected(CommandPrefix),
    
    /// Complete command detected
    CommandComplete {
        prefix: CommandPrefix,
        command: String,
    },
    
    /// Partial data, keep feeding
    Partial,
}

/// Command preparation based on prefix
pub struct CommandPreparer;

impl CommandPreparer {
    /// Prepare system resources based on detected prefix
    pub fn prepare_for_prefix(prefix: &CommandPrefix) {
        match prefix {
            CommandPrefix::Validated => {
                // Pre-load schema validator
                log::debug!("Pre-loading schema validator for validated command");
            }
            
            CommandPrefix::Dangerous => {
                // Prepare confirmation UI
                log::debug!("Preparing confirmation prompt for dangerous command");
            }
            
            CommandPrefix::System => {
                // Check permissions early
                log::debug!("Checking system permissions");
            }
            
            CommandPrefix::Background => {
                // Prepare background executor
                log::debug!("Preparing background job executor");
            }
            
            CommandPrefix::Sandboxed => {
                // Initialize sandbox environment
                log::debug!("Initializing sandbox environment");
            }
            
            CommandPrefix::None => {
                // Standard preparation
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_prefix_parsing() {
        assert_eq!(CommandPrefix::parse("!@ echo hello"), (CommandPrefix::Validated, " echo hello"));
        assert_eq!(CommandPrefix::parse("!# rm -rf /"), (CommandPrefix::Dangerous, " rm -rf /"));
        assert_eq!(CommandPrefix::parse("normal text"), (CommandPrefix::None, "normal text"));
    }
    
    #[test]
    fn test_stream_detection() {
        let mut detector = StreamDetector::new();
        
        // Feed prefix in chunks
        match detector.feed("!") {
            StreamDetection::Partial => {}
            _ => panic!("Should be partial"),
        }
        
        match detector.feed("@") {
            StreamDetection::PrefixDetected(CommandPrefix::Validated) => {}
            _ => panic!("Should detect validated prefix"),
        }
        
        // Feed command
        detector.feed(" echo test");
        match detector.feed("\n") {
            StreamDetection::CommandComplete { prefix, command } => {
                assert_eq!(prefix, CommandPrefix::Validated);
                assert_eq!(command, " echo test");
            }
            _ => panic!("Should complete command"),
        }
    }
}