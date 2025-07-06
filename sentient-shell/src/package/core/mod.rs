// Core packages module for SentientOS
// These are essential first-party packages built into the shell

pub mod calc;
pub mod neofetch;
pub mod joke;
pub mod todo;
pub mod ask;
pub mod timer;
pub mod scratch;
pub mod df;
pub mod top;
pub mod ps;
pub mod hivefix;


// Trait for core packages
pub trait CorePackage {
    fn name(&self) -> &'static str;
    fn version(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn category(&self) -> PackageCategory;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PackageCategory {
    System,
    Knowledge,
    Utils,
    Dev,
    Creative,
}

impl std::fmt::Display for PackageCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackageCategory::System => write!(f, "System"),
            PackageCategory::Knowledge => write!(f, "Knowledge"),
            PackageCategory::Utils => write!(f, "Utils"),
            PackageCategory::Dev => write!(f, "Dev"),
            PackageCategory::Creative => write!(f, "Creative"),
        }
    }
}

