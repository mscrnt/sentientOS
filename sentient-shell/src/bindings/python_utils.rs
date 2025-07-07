use pyo3::prelude::*;
use anyhow::{Result, anyhow};
use log::{info, warn};
use std::collections::HashMap;

/// Python package management utilities
pub struct PythonPackageChecker;

impl PythonPackageChecker {
    /// Check if required packages are installed and return their versions
    pub fn check_packages(packages: &[&str]) -> Result<HashMap<String, String>> {
        Python::with_gil(|py| {
            let mut versions = HashMap::new();
            let mut missing = Vec::new();
            
            for package in packages {
                match Self::get_package_version(py, package) {
                    Ok(version) => {
                        info!("✓ {} ({})", package, version);
                        versions.insert(package.to_string(), version);
                    }
                    Err(_) => {
                        warn!("✗ {} not found", package);
                        missing.push(*package);
                    }
                }
            }
            
            if !missing.is_empty() {
                let missing_list = missing.join(", ");
                return Err(anyhow!(
                    "Missing required Python packages: {}. Install with: pip3 install {}",
                    missing_list,
                    missing.join(" ")
                ));
            }
            
            Ok(versions)
        })
    }
    
    /// Get version of a specific package
    fn get_package_version(py: Python, package: &str) -> Result<String> {
        // Try importlib.metadata first (Python 3.8+)
        let code = format!(r#"
try:
    import importlib.metadata
    version = importlib.metadata.version("{}")
except:
    # Fallback for older Python
    import pkg_resources
    version = pkg_resources.get_distribution("{}").version
version
"#, package, package);
        
        match py.eval(&code, None, None) {
            Ok(version) => Ok(version.extract()?),
            Err(_) => {
                // Try direct import with __version__
                let module = py.import(package)?;
                if let Ok(version) = module.getattr("__version__") {
                    Ok(version.extract()?)
                } else {
                    Err(anyhow!("Package {} not found", package))
                }
            }
        }
    }
    
    /// Install missing packages using pip
    pub fn install_packages(packages: &[&str]) -> Result<()> {
        Python::with_gil(|py| {
            let subprocess = py.import("subprocess")?;
            let sys = py.import("sys")?;
            
            let python_exe = sys.getattr("executable")?;
            
            info!("Installing packages: {}", packages.join(", "));
            
            let args = vec![
                python_exe,
                py.eval("-m", None, None)?,
                py.eval("pip", None, None)?,
                py.eval("install", None, None)?,
            ];
            
            let mut cmd_args = args;
            for package in packages {
                cmd_args.push(py.eval(&format!("'{}'", package), None, None)?);
            }
            
            subprocess.call_method1("check_call", (cmd_args,))?;
            
            info!("✅ Packages installed successfully");
            Ok(())
        })
    }
}

/// Secure Python sandbox configuration
pub struct PythonSandbox;

impl PythonSandbox {
    /// Configure Python environment with security restrictions
    pub fn configure() -> Result<()> {
        Python::with_gil(|py| {
            // Restrict dangerous imports
            let code = r#"
import sys
import builtins

# Save original __import__ for allowed modules
_original_import = builtins.__import__

# Define allowed modules
ALLOWED_MODULES = {
    'numpy', 'torch', 'pickle', 'json', 'pathlib', 'os.path',
    'collections', 'typing', 'dataclasses', 'enum', 'math',
    'datetime', 'itertools', 'functools', 're'
}

def restricted_import(name, *args, **kwargs):
    # Check if module or parent module is allowed
    module_parts = name.split('.')
    base_module = module_parts[0]
    
    if base_module not in ALLOWED_MODULES:
        # Special cases for submodules
        if name.startswith('torch.') or name.startswith('numpy.'):
            return _original_import(name, *args, **kwargs)
        raise ImportError(f"Import of '{name}' is not allowed in sandbox")
    
    return _original_import(name, *args, **kwargs)

# Apply restriction
builtins.__import__ = restricted_import

# Disable dangerous functions
def _disabled(*args, **kwargs):
    raise RuntimeError("This function is disabled in sandbox mode")

# Disable subprocess and os.system
if 'subprocess' in sys.modules:
    del sys.modules['subprocess']
if 'os' in sys.modules and hasattr(sys.modules['os'], 'system'):
    sys.modules['os'].system = _disabled
"#;
            
            py.run(code, None, None)?;
            info!("🔒 Python sandbox configured");
            Ok(())
        })
    }
}

/// Python execution timeout wrapper
pub struct TimeoutExecutor;

impl TimeoutExecutor {
    /// Execute Python code with timeout
    pub fn execute_with_timeout<F, R>(
        timeout_seconds: u64,
        func: F
    ) -> Result<R>
    where
        F: FnOnce() -> Result<R> + Send + 'static,
        R: Send + 'static,
    {
        use std::sync::mpsc;
        use std::thread;
        use std::time::Duration;
        
        let (tx, rx) = mpsc::channel();
        
        thread::spawn(move || {
            let result = func();
            let _ = tx.send(result);
        });
        
        match rx.recv_timeout(Duration::from_secs(timeout_seconds)) {
            Ok(result) => result,
            Err(_) => Err(anyhow!("Python execution timed out after {} seconds", timeout_seconds)),
        }
    }
}