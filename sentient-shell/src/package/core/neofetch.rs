use super::{CorePackage, PackageCategory};

pub struct Neofetch;

impl CorePackage for Neofetch {
    fn name(&self) -> &'static str { "neofetch" }
    fn version(&self) -> &'static str { "1.0.0" }
    fn description(&self) -> &'static str { "Display system information in a pretty way" }
    fn category(&self) -> PackageCategory { PackageCategory::System }
}

pub fn run() -> String {
    r#"
       _____            _   _            _   ____   _____
      / ____|          | | (_)          | | / __ \ / ____|
     | (___   ___ _ __ | |_ _  ___ _ __ | || |  | | (___
      \___ \ / _ \ '_ \| __| |/ _ \ '_ \| || |  | |\___ \
      ____) |  __/ | | | |_| |  __/ | | | || |__| |____) |
     |_____/ \___|_| |_|\__|_|\___|_| |_|_| \____/|_____/

     OS: SentientOS v0.1.0
     Kernel: AI-First Microkernel
     Shell: SentientShell v1.0
     AI Model: deepseek-v2:16b
     Memory: Dynamic AI-managed
     CPU: Quantum-ready
"#.to_string()
}