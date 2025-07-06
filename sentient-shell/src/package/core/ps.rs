use anyhow::Result;
use super::{CorePackage, PackageCategory};

pub struct Ps;

impl CorePackage for Ps {
    fn name(&self) -> &'static str { "ps" }
    fn version(&self) -> &'static str { "1.0.0" }
    fn description(&self) -> &'static str { "List running processes" }
    fn category(&self) -> PackageCategory { PackageCategory::System }
}

pub fn run(args: &[&str]) -> Result<String> {
    let show_all = args.contains(&"-a") || args.contains(&"--all");
    
    let mut output = String::from("  PID TTY          TIME CMD\n");
    
    // Basic processes always shown
    output.push_str("    1 ?        00:00:15 init\n");
    output.push_str("  128 pts/0    00:01:23 sentient-shell\n");
    
    if show_all {
        // Additional system processes
        output.push_str("   42 ?        00:42:31 ollama-daemon\n");
        output.push_str("   64 ?        00:05:12 ai-scheduler\n");
        output.push_str("  256 ?        00:05:42 quantum-sync\n");
        output.push_str("  512 ?        00:15:20 neural-cache\n");
        output.push_str(" 1024 ?        00:02:45 memory-optimizer\n");
    }
    
    Ok(output)
}