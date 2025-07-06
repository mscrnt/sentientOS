use super::*;
use anyhow::Result;

/// Prompt templates for different error types
pub struct PromptTemplates;

impl PromptTemplates {
    pub fn get_prompt(error: &ErrorEvent) -> String {
        let base_context = format!(
            "You are HiveFix, an AI-powered self-healing agent for SentientOS. \
            You must analyze errors and propose safe, minimal fixes. \
            Always prioritize system stability and security.\n\n"
        );
        
        let specific_prompt = match &error.source {
            ErrorSource::Shell => Self::shell_error_prompt(error),
            ErrorSource::Package(name) => Self::package_error_prompt(error, name),
            ErrorSource::Kernel => Self::kernel_error_prompt(error),
            ErrorSource::System => Self::system_error_prompt(error),
            ErrorSource::User => Self::user_error_prompt(error),
        };
        
        format!("{}{}", base_context, specific_prompt)
    }
    
    fn shell_error_prompt(error: &ErrorEvent) -> String {
        format!(
            "Shell Error Analysis:\n\
            Error: {}\n\
            {}\n\n\
            Analyze this shell error and provide:\n\
            1. Root cause analysis\n\
            2. A minimal fix that preserves shell functionality\n\
            3. Steps to test the fix\n\
            4. Potential side effects\n\n\
            Format your response as:\n\
            CAUSE: [brief explanation]\n\
            FIX: [code or configuration change]\n\
            TEST: [verification steps]\n\
            RISK: [potential issues]",
            error.message,
            error.stack_trace.as_deref().map(|s| format!("Stack trace:\n{}", s)).unwrap_or_default()
        )
    }
    
    fn package_error_prompt(error: &ErrorEvent, package_name: &str) -> String {
        format!(
            "Package Error Analysis for '{}':\n\
            Error: {}\n\
            {}\n\n\
            This error occurred in the '{}' package. Analyze and provide:\n\
            1. Is this a code bug, missing dependency, or configuration issue?\n\
            2. Minimal fix that maintains package functionality\n\
            3. Does this affect other packages?\n\
            4. Test strategy\n\n\
            Consider the package's purpose and provide a fix that:\n\
            - Preserves the package's core functionality\n\
            - Doesn't break dependent packages\n\
            - Can be safely tested in isolation\n\n\
            Format as:\n\
            TYPE: [bug/dependency/config]\n\
            FIX: [specific code changes]\n\
            DEPENDENCIES: [affected packages]\n\
            TEST: [test commands]",
            package_name,
            error.message,
            error.stack_trace.as_deref().map(|s| format!("Stack trace:\n{}", s)).unwrap_or_default(),
            package_name
        )
    }
    
    fn kernel_error_prompt(error: &ErrorEvent) -> String {
        format!(
            "CRITICAL: Kernel Error Analysis\n\
            Error: {}\n\
            {}\n\n\
            This is a kernel-level error. Exercise EXTREME CAUTION.\n\
            \n\
            Analyze whether this can be safely fixed without:\n\
            1. Requiring a reboot\n\
            2. Corrupting system state\n\
            3. Breaking hardware compatibility\n\
            \n\
            If a fix is possible, provide:\n\
            - Configuration-only changes (preferred)\n\
            - Module reload commands\n\
            - State cleanup procedures\n\
            \n\
            If the error requires kernel modification, respond with:\n\
            SAFE: false\n\
            REASON: [why this needs manual intervention]\n\
            \n\
            Otherwise:\n\
            SAFE: true\n\
            FIX: [safe remediation steps]\n\
            VERIFY: [how to confirm fix worked]",
            error.message,
            error.context.as_deref().unwrap_or("No additional context")
        )
    }
    
    fn system_error_prompt(error: &ErrorEvent) -> String {
        format!(
            "System Error Analysis:\n\
            Error: {}\n\
            Context: {}\n\n\
            Analyze this system-level error and determine:\n\
            1. Is this a configuration issue, resource problem, or service failure?\n\
            2. Can it be fixed without system restart?\n\
            3. What's the minimal intervention needed?\n\
            \n\
            Provide a fix that:\n\
            - Uses existing system tools\n\
            - Preserves system stability\n\
            - Can be rolled back if needed\n\
            \n\
            Format:\n\
            CATEGORY: [config/resource/service]\n\
            SEVERITY: [low/medium/high]\n\
            FIX: [specific commands or changes]\n\
            ROLLBACK: [how to undo if needed]",
            error.message,
            error.context.as_deref().unwrap_or("No context provided")
        )
    }
    
    fn user_error_prompt(error: &ErrorEvent) -> String {
        format!(
            "User Command Error:\n\
            Error: {}\n\
            {}\n\n\
            This error resulted from user action. Analyze whether:\n\
            1. The user made a syntax error\n\
            2. The command has a bug\n\
            3. Required resources are missing\n\
            \n\
            Provide guidance that:\n\
            - Helps the user understand what went wrong\n\
            - Suggests the correct usage\n\
            - Fixes any underlying issues if present\n\
            \n\
            Format:\n\
            USER_ERROR: [yes/no]\n\
            GUIDANCE: [help text for user]\n\
            FIX: [any system fixes needed]\n\
            EXAMPLE: [correct usage example]",
            error.message,
            error.stack_trace.as_deref().unwrap_or("No stack trace")
        )
    }
    
    /// Get a prompt for testing a specific fix
    pub fn get_test_prompt(fix: &FixCandidate) -> String {
        format!(
            "Validate this fix for safety and correctness:\n\n\
            Fix ID: {}\n\
            Description: {}\n\
            Patch:\n{}\n\n\
            Analyze:\n\
            1. Does this fix address the root cause?\n\
            2. Are there any security risks?\n\
            3. Could this break other functionality?\n\
            4. What specific tests would validate this fix?\n\
            \n\
            Respond with:\n\
            VALID: [true/false]\n\
            RISKS: [list any concerns]\n\
            TESTS: [specific test commands]\n\
            CONFIDENCE: [0.0-1.0]",
            fix.id,
            fix.description,
            fix.patch
        )
    }
}

/// Enhanced prompt with system metadata
pub fn create_enhanced_prompt(
    error: &ErrorEvent,
    system_info: &SystemInfo,
) -> String {
    let base = PromptTemplates::get_prompt(error);
    
    format!(
        "{}\n\n\
        System Context:\n\
        - Shell Version: {}\n\
        - Uptime: {} seconds\n\
        - Recent Commands: {}\n\
        - Memory Usage: {}%\n\
        - Active Packages: {}\n",
        base,
        system_info.shell_version,
        system_info.uptime_seconds,
        system_info.recent_commands.join(", "),
        system_info.memory_usage_percent,
        system_info.active_packages.join(", ")
    )
}

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub shell_version: String,
    pub uptime_seconds: u64,
    pub recent_commands: Vec<String>,
    pub memory_usage_percent: u8,
    pub active_packages: Vec<String>,
}