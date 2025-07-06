use anyhow::Result;
use super::{CorePackage, PackageCategory};

pub struct HiveFix;

impl CorePackage for HiveFix {
    fn name(&self) -> &'static str { "hivefix" }
    fn version(&self) -> &'static str { "1.0.0" }
    fn description(&self) -> &'static str { "Self-healing AI agent that monitors and fixes system errors" }
    fn category(&self) -> PackageCategory { PackageCategory::System }
}

// Global hivefix agent instance
use std::sync::Mutex;
lazy_static::lazy_static! {
    static ref AGENT: Mutex<Option<crate::hivefix::agent::HiveFixAgent>> = Mutex::new(None);
}

pub fn run(args: &[&str]) -> Result<String> {
    // Initialize agent if needed
    let mut agent_lock = AGENT.lock().unwrap();
    if agent_lock.is_none() {
        let config = crate::hivefix::HiveFixConfig::default();
        *agent_lock = Some(crate::hivefix::agent::HiveFixAgent::new(config));
    }
    
    if let Some(agent) = agent_lock.as_mut() {
        crate::hivefix::api::handle_command(agent, args)
    } else {
        Err(anyhow::anyhow!("Failed to initialize HiveFix agent"))
    }
}