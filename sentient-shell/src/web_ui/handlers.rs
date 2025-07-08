use super::*;
use warp::Reply;
use serde_json::json;
use std::fs::OpenOptions;
use std::io::Write as IoWrite;

#[derive(Debug, Deserialize)]
pub struct ActivityQuery {
    limit: Option<usize>,
}

/// Get all dashboard data
pub async fn get_dashboard_all(
    state: Arc<DashboardState>,
) -> Result<impl Reply, warp::Rejection> {
    let metrics = state.metrics.read().await;
    let activity = state.activity_log.read().await;
    let services = state.service_status.read().await;
    
    let response = json!({
        "system": {
            "cpu_percent": metrics.cpu_percent,
            "memory_percent": metrics.memory_percent,
            "disk_usage": metrics.disk_usage,
            "process_count": metrics.process_count,
            "uptime": metrics.uptime,
        },
        "activity": activity.iter().rev().take(50).collect::<Vec<_>>(),
        "services": services.clone(),
        "timestamp": Utc::now().to_rfc3339(),
    });
    
    Ok(warp::reply::json(&response))
}

/// Get system status
pub async fn get_system_status(
    state: Arc<DashboardState>,
) -> Result<impl Reply, warp::Rejection> {
    let metrics = state.metrics.read().await;
    
    let response = json!({
        "cpu_percent": metrics.cpu_percent,
        "memory_percent": metrics.memory_percent,
        "disk_usage": metrics.disk_usage,
        "process_count": metrics.process_count,
        "uptime": metrics.uptime,
        "timestamp": Utc::now().to_rfc3339(),
    });
    
    Ok(warp::reply::json(&response))
}

/// Get recent activity
pub async fn get_recent_activity(
    query: ActivityQuery,
    state: Arc<DashboardState>,
) -> Result<impl Reply, warp::Rejection> {
    let activity = state.activity_log.read().await;
    let limit = query.limit.unwrap_or(50);
    
    let entries: Vec<_> = activity
        .iter()
        .rev()
        .take(limit)
        .cloned()
        .collect();
    
    Ok(warp::reply::json(&entries))
}

/// Inject a goal into the system
pub async fn inject_goal(
    request: InjectGoalRequest,
    _state: Arc<DashboardState>,
) -> Result<impl Reply, warp::Rejection> {
    if request.goal.trim().is_empty() {
        return Ok(warp::reply::with_status(
            warp::reply::json(&json!({"error": "Goal cannot be empty"})),
            warp::http::StatusCode::BAD_REQUEST,
        ));
    }
    
    let injection = json!({
        "goal": request.goal,
        "source": "web_ui",
        "timestamp": Utc::now().to_rfc3339(),
        "reasoning": "Manual injection from native dashboard",
        "priority": request.priority.unwrap_or_else(|| "medium".to_string()),
        "injected": true,
        "processed": false,
    });
    
    // Write to goal injection file
    let logs_dir = Path::new("logs");
    std::fs::create_dir_all(logs_dir).map_err(|e| {
        log::error!("Failed to create logs directory: {}", e);
        warp::reject::reject()
    })?;
    
    let injection_file = logs_dir.join("goal_injections.jsonl");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&injection_file)
        .map_err(|e| {
            log::error!("Failed to open injection file: {}", e);
            warp::reject::reject()
        })?;
    
    writeln!(file, "{}", serde_json::to_string(&injection).unwrap())
        .map_err(|e| {
            log::error!("Failed to write injection: {}", e);
            warp::reject::reject()
        })?;
    
    log::info!("Goal injected via web UI: {}", request.goal);
    
    Ok(warp::reply::with_status(
        warp::reply::json(&json!({
            "status": "success",
            "goal": request.goal,
            "timestamp": Utc::now().to_rfc3339(),
        })),
        warp::http::StatusCode::OK,
    ))
}