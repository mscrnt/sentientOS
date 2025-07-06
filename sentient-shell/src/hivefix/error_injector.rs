use super::*;
use anyhow::Result;
use crate::hivefix::agent::HiveFixAgent;

/// Test error scenarios for HiveFix validation
#[derive(Debug, Clone)]
pub enum TestError {
    /// Break a package's functionality
    BrokenPackage { name: String, error_type: PackageError },
    /// Simulate a panic
    Panic { message: String },
    /// Create an infinite loop
    InfiniteLoop,
    /// Memory allocation abuse
    MemoryAbuse { size_mb: usize },
    /// File system corruption
    CorruptedFile { path: String },
    /// Configuration error
    ConfigError { key: String, bad_value: String },
}

#[derive(Debug, Clone)]
pub enum PackageError {
    SyntaxError,
    RuntimeError,
    MissingDependency,
    CorruptedBinary,
}

pub struct ErrorInjector;

impl ErrorInjector {
    /// Inject a test error into the system
    pub fn inject(error: TestError) -> Result<ErrorEvent> {
        match error {
            TestError::BrokenPackage { name, error_type } => {
                Self::inject_package_error(&name, error_type)
            }
            TestError::Panic { message } => {
                Self::inject_panic(&message)
            }
            TestError::InfiniteLoop => {
                Self::inject_infinite_loop()
            }
            TestError::MemoryAbuse { size_mb } => {
                Self::inject_memory_abuse(size_mb)
            }
            TestError::CorruptedFile { path } => {
                Self::inject_file_corruption(&path)
            }
            TestError::ConfigError { key, bad_value } => {
                Self::inject_config_error(&key, &bad_value)
            }
        }
    }
    
    fn inject_package_error(name: &str, error_type: PackageError) -> Result<ErrorEvent> {
        let (message, stack_trace) = match error_type {
            PackageError::SyntaxError => (
                format!("Syntax error in package '{}': unexpected token at line 42", name),
                Some("  at parse_expression (src/package/core/{}.rs:42:15)\n  at run (src/package/core/{}.rs:10:5)".to_string())
            ),
            PackageError::RuntimeError => (
                format!("Runtime error in package '{}': division by zero", name),
                Some("  at calculate (src/package/core/{}.rs:55:20)\n  at run (src/package/core/{}.rs:15:10)".to_string())
            ),
            PackageError::MissingDependency => (
                format!("Package '{}' missing required dependency", name),
                None
            ),
            PackageError::CorruptedBinary => (
                format!("Package '{}' binary is corrupted", name),
                None
            ),
        };
        
        Ok(ErrorEvent {
            id: format!("test_pkg_error_{}", chrono::Utc::now().timestamp()),
            timestamp: std::time::SystemTime::now(),
            source: ErrorSource::Package(name.to_string()),
            message,
            stack_trace: stack_trace.map(|s| s.replace("{}", name)),
            context: Some(format!("Injected test error for package '{}'", name)),
        })
    }
    
    fn inject_panic(message: &str) -> Result<ErrorEvent> {
        Ok(ErrorEvent {
            id: format!("test_panic_{}", chrono::Utc::now().timestamp()),
            timestamp: std::time::SystemTime::now(),
            source: ErrorSource::System,
            message: format!("thread 'main' panicked at '{}', src/main.rs:123:5", message),
            stack_trace: Some(
                "stack backtrace:\n\
                   0: rust_begin_unwind\n\
                   1: core::panicking::panic_fmt\n\
                   2: sentient_shell::main\n\
                   3: std::rt::lang_start::{{closure}}\n\
                   4: std::rt::lang_start_internal".to_string()
            ),
            context: Some("Injected panic for testing".to_string()),
        })
    }
    
    fn inject_infinite_loop() -> Result<ErrorEvent> {
        Ok(ErrorEvent {
            id: format!("test_loop_{}", chrono::Utc::now().timestamp()),
            timestamp: std::time::SystemTime::now(),
            source: ErrorSource::User,
            message: "Process exceeded maximum execution time (30s)".to_string(),
            stack_trace: Some("  at user_function (input:1:1)\n  at execute_command (src/shell_state.rs:45:10)".to_string()),
            context: Some("Injected infinite loop detection".to_string()),
        })
    }
    
    fn inject_memory_abuse(size_mb: usize) -> Result<ErrorEvent> {
        Ok(ErrorEvent {
            id: format!("test_memory_{}", chrono::Utc::now().timestamp()),
            timestamp: std::time::SystemTime::now(),
            source: ErrorSource::System,
            message: format!("Memory allocation failed: requested {} MB exceeds limit", size_mb),
            stack_trace: Some("  at allocate (src/mm/allocator.rs:89:15)\n  at Vec::reserve (alloc/vec.rs:123:20)".to_string()),
            context: Some("Injected memory abuse detection".to_string()),
        })
    }
    
    fn inject_file_corruption(path: &str) -> Result<ErrorEvent> {
        Ok(ErrorEvent {
            id: format!("test_corruption_{}", chrono::Utc::now().timestamp()),
            timestamp: std::time::SystemTime::now(),
            source: ErrorSource::System,
            message: format!("File '{}' appears to be corrupted: invalid header", path),
            stack_trace: None,
            context: Some("Injected file corruption error".to_string()),
        })
    }
    
    fn inject_config_error(key: &str, bad_value: &str) -> Result<ErrorEvent> {
        Ok(ErrorEvent {
            id: format!("test_config_{}", chrono::Utc::now().timestamp()),
            timestamp: std::time::SystemTime::now(),
            source: ErrorSource::System,
            message: format!("Configuration error: invalid value '{}' for key '{}'", bad_value, key),
            stack_trace: None,
            context: Some("Injected configuration error".to_string()),
        })
    }
}

/// Test harness for HiveFix
pub struct HiveFixTestHarness {
    agent: HiveFixAgent,
    injected_errors: Vec<ErrorEvent>,
}

impl HiveFixTestHarness {
    pub fn new() -> Self {
        let config = HiveFixConfig {
            auto_fix: true,
            sandbox_timeout_ms: 5000, // Shorter timeout for tests
            ..Default::default()
        };
        
        Self {
            agent: HiveFixAgent::new(config),
            injected_errors: Vec::new(),
        }
    }
    
    pub fn run_test_scenario(&mut self, scenario: TestScenario) -> Result<TestReport> {
        let mut report = TestReport::new(scenario.name.clone());
        
        // Start the agent
        self.agent.start()?;
        
        // Inject errors
        for test_error in scenario.errors {
            let error = ErrorInjector::inject(test_error)?;
            self.injected_errors.push(error.clone());
            
            // Simulate adding to error log
            self.agent.error_history.lock().unwrap().push(error);
        }
        
        // Wait for processing
        std::thread::sleep(std::time::Duration::from_secs(2));
        
        // Check results
        let fixes = self.agent.get_fix_candidates();
        report.fixes_generated = fixes.len();
        
        for (_, fix) in fixes {
            if fix.tested {
                report.fixes_tested += 1;
                if let Some(result) = &fix.test_result {
                    if result.success {
                        report.fixes_passed += 1;
                    }
                }
            }
        }
        
        // Stop agent
        self.agent.stop();
        
        Ok(report)
    }
}

#[derive(Debug)]
pub struct TestScenario {
    pub name: String,
    pub errors: Vec<TestError>,
}

#[derive(Debug)]
pub struct TestReport {
    pub scenario_name: String,
    pub fixes_generated: usize,
    pub fixes_tested: usize,
    pub fixes_passed: usize,
    pub errors: Vec<String>,
}

impl TestReport {
    fn new(scenario_name: String) -> Self {
        Self {
            scenario_name,
            fixes_generated: 0,
            fixes_tested: 0,
            fixes_passed: 0,
            errors: Vec::new(),
        }
    }
}