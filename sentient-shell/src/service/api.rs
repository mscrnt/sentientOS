use super::*;
use super::manager::{get_service_manager, ServiceManager};
use anyhow::Result;

// CLI API for service commands
pub fn handle_command(args: &[&str]) -> Result<String> {
    if args.is_empty() {
        return Ok(help());
    }

    let manager = get_service_manager();

    match args[0] {
        "list" => list_services(manager),
        "status" => {
            if args.len() < 2 {
                return Ok("Usage: service status <name>".to_string());
            }
            show_service_status(manager, args[1])
        },
        "start" => {
            if args.len() < 2 {
                return Ok("Usage: service start <name>".to_string());
            }
            start_service(manager, args[1])
        },
        "stop" => {
            if args.len() < 2 {
                return Ok("Usage: service stop <name>".to_string());
            }
            stop_service(manager, args[1])
        },
        "restart" => {
            if args.len() < 2 {
                return Ok("Usage: service restart <name>".to_string());
            }
            restart_service(manager, args[1])
        },
        "logs" => {
            if args.len() < 2 {
                return Ok("Usage: service logs <name> [lines]".to_string());
            }
            let lines = args.get(2)
                .and_then(|s| s.parse().ok())
                .unwrap_or(20);
            show_service_logs(manager, args[1], lines)
        },
        "init" => {
            manager.init()?;
            Ok("Service manager initialized".to_string())
        },
        "run" => {
            if args.len() < 2 {
                return Ok("Usage: service run <name>".to_string());
            }
            run_service(args[1])
        },
        _ => Ok(help()),
    }
}

fn help() -> String {
    r#"Service Manager (sentd) - Manage system services

Commands:
  service list              List all services
  service status <name>     Show service status
  service start <name>      Start a service
  service stop <name>       Stop a service
  service restart <name>    Restart a service
  service logs <name> [n]   Show last n log lines (default: 20)
  service init             Initialize service manager
  service run <name>        Run a service directly (for service mode)

Services are configured in TOML files at /etc/sentient/services/"#.to_string()
}

/// Run a service directly (used when shell is launched in service mode)
fn run_service(name: &str) -> Result<String> {
    use crate::services::ServiceRunner;
    use tokio::runtime::Runtime;
    
    let rt = Runtime::new()?;
    rt.block_on(async {
        ServiceRunner::run_service(name).await
    })?;
    
    Ok(format!("Service {} completed", name))
}

fn list_services(manager: &ServiceManager) -> Result<String> {
    let services = manager.list_services()?;
    
    if services.is_empty() {
        return Ok("No services configured".to_string());
    }

    let mut output = String::from("Services:\n");
    output.push_str("NAME            STATUS       PID     RESTARTS\n");
    output.push_str("─────────────────────────────────────────────\n");

    for service in services {
        let pid_str = service.pid
            .map(|p| p.to_string())
            .unwrap_or_else(|| "-".to_string());

        output.push_str(&format!(
            "{:<15} {:<12} {:<7} {}\n",
            service.name,
            service.status.to_string(),
            pid_str,
            service.restart_count
        ));
    }

    Ok(output)
}

fn show_service_status(manager: &ServiceManager, name: &str) -> Result<String> {
    let info = manager.get_service_status(name)?;
    
    let mut output = format!("Service: {}\n", info.name);
    output.push_str(&format!("Status: {}\n", info.status));
    
    if let Some(pid) = info.pid {
        output.push_str(&format!("PID: {}\n", pid));
    }
    
    if let Some(started) = info.started_at {
        let duration = std::time::SystemTime::now()
            .duration_since(started)
            .unwrap_or_default();
        output.push_str(&format!("Uptime: {}s\n", duration.as_secs()));
    }
    
    output.push_str(&format!("Restarts: {}\n", info.restart_count));
    
    if let Some(code) = info.last_exit_code {
        output.push_str(&format!("Last exit code: {}\n", code));
    }

    // Add health status if available
    if let Some(health_status) = get_service_manager()
        .health_monitor
        .get_status(name) 
    {
        output.push_str(&format!("Health: {:?}\n", health_status));
    }

    Ok(output)
}

fn start_service(manager: &ServiceManager, name: &str) -> Result<String> {
    manager.start_service(name)?;
    Ok(format!("Service '{}' started", name))
}

fn stop_service(manager: &ServiceManager, name: &str) -> Result<String> {
    manager.stop_service(name)?;
    Ok(format!("Service '{}' stopped", name))
}

fn restart_service(manager: &ServiceManager, name: &str) -> Result<String> {
    manager.restart_service(name)?;
    Ok(format!("Service '{}' restarted", name))
}

fn show_service_logs(manager: &ServiceManager, name: &str, lines: usize) -> Result<String> {
    let logs = manager.get_service_logs(name, lines);
    
    if logs.is_empty() {
        Ok(format!("No logs available for service '{}'", name))
    } else {
        Ok(logs.join("\n"))
    }
}