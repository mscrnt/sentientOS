// Native Web UI for SentientOS Admin Panel
// Serves a dashboard for monitoring and control

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::{Filter, Reply};
use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use std::path::Path;

pub mod handlers;
pub mod metrics;

use handlers::*;
use metrics::SystemMetrics;

/// Dashboard state
pub struct DashboardState {
    pub metrics: Arc<RwLock<SystemMetrics>>,
    pub activity_log: Arc<RwLock<VecDeque<ActivityEntry>>>,
    pub service_status: Arc<RwLock<Vec<ServiceStatus>>>,
}

/// Activity log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEntry {
    pub timestamp: DateTime<Utc>,
    pub goal: String,
    pub command: String,
    pub output: String,
    pub success: bool,
    pub reward: f32,
    pub execution_time: f32,
    pub source: String,
}

/// Service status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub name: String,
    pub status: String,
    pub pid: Option<u32>,
    pub uptime: Option<u64>,
}

/// Goal injection request
#[derive(Debug, Deserialize)]
pub struct InjectGoalRequest {
    pub goal: String,
    pub priority: Option<String>,
}

impl DashboardState {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(SystemMetrics::new())),
            activity_log: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            service_status: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Update system metrics
    pub async fn update_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.update();
    }
    
    /// Add activity entry
    pub async fn add_activity(&self, entry: ActivityEntry) {
        let mut log = self.activity_log.write().await;
        if log.len() >= 1000 {
            log.pop_front();
        }
        log.push_back(entry);
    }
    
    /// Update service status
    pub async fn update_service_status(&self, services: Vec<ServiceStatus>) {
        let mut status = self.service_status.write().await;
        *status = services;
    }
}

/// Start the web UI server
pub async fn start_server(port: u16) -> Result<()> {
    let state = Arc::new(DashboardState::new());
    
    // Clone for background tasks
    let state_clone = state.clone();
    
    // Start metrics updater
    tokio::spawn(async move {
        loop {
            state_clone.update_metrics().await;
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });
    
    // Define routes
    let routes = routes(state);
    
    // Start server
    log::info!("🎛️ SentientOS Admin Panel starting on http://0.0.0.0:{}", port);
    warp::serve(routes)
        .run(([0, 0, 0, 0], port))
        .await;
    
    Ok(())
}

/// Configure all routes
fn routes(
    state: Arc<DashboardState>
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type"])
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE"]);
    
    // Static files (HTML, CSS, JS)
    let index = warp::path::end()
        .map(|| warp::reply::html(INDEX_HTML));
    
    // API routes
    let api = warp::path("api");
    
    let dashboard_all = api
        .and(warp::path("dashboard"))
        .and(warp::path("all"))
        .and(warp::path::end())
        .and(with_state(state.clone()))
        .and_then(handlers::get_dashboard_all);
    
    let system_status = api
        .and(warp::path("system"))
        .and(warp::path("status"))
        .and(warp::path::end())
        .and(with_state(state.clone()))
        .and_then(handlers::get_system_status);
    
    let activity_recent = api
        .and(warp::path("activity"))
        .and(warp::path("recent"))
        .and(warp::path::end())
        .and(warp::query::<ActivityQuery>())
        .and(with_state(state.clone()))
        .and_then(handlers::get_recent_activity);
    
    let inject_goal = api
        .and(warp::path("goal"))
        .and(warp::path("inject"))
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(handlers::inject_goal);
    
    // Combine all routes
    index
        .or(dashboard_all)
        .or(system_status)
        .or(activity_recent)
        .or(inject_goal)
        .with(cors)
}

/// Helper to inject state into handlers
fn with_state(
    state: Arc<DashboardState>
) -> impl Filter<Extract = (Arc<DashboardState>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || state.clone())
}

// Static HTML for the dashboard
const INDEX_HTML: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <title>SentientOS Native Dashboard</title>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #0a0a0a;
            color: #e0e0e0;
            line-height: 1.6;
        }
        
        .header {
            background: #1a1a1a;
            border-bottom: 2px solid #00ff88;
            padding: 20px;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }
        
        h1 {
            color: #00ff88;
            font-size: 2em;
            display: flex;
            align-items: center;
            gap: 10px;
        }
        
        .rust-badge {
            background: #ce422b;
            color: white;
            padding: 2px 8px;
            border-radius: 4px;
            font-size: 0.6em;
            font-weight: bold;
        }
        
        .container {
            max-width: 1600px;
            margin: 0 auto;
            padding: 20px;
        }
        
        .grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(350px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }
        
        .card {
            background: #1a1a1a;
            border: 1px solid #333;
            border-radius: 8px;
            padding: 20px;
            position: relative;
            overflow: hidden;
        }
        
        .card-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 15px;
        }
        
        .card-title {
            color: #00ff88;
            font-size: 1.2em;
            font-weight: bold;
        }
        
        .metric-value {
            font-size: 2.5em;
            font-weight: bold;
            color: #00ff88;
            margin: 10px 0;
        }
        
        .metric-label {
            color: #888;
            font-size: 0.9em;
        }
        
        .activity-feed {
            background: #0d0d0d;
            border-radius: 4px;
            padding: 15px;
            max-height: 400px;
            overflow-y: auto;
            font-family: 'Courier New', monospace;
            font-size: 0.9em;
        }
        
        .log-entry {
            margin-bottom: 8px;
            padding: 8px;
            border-left: 3px solid #444;
            background: rgba(255, 255, 255, 0.02);
            word-break: break-word;
        }
        
        .log-success { border-left-color: #00ff88; }
        .log-error { border-left-color: #ff4444; }
        
        .status-indicator {
            display: inline-block;
            width: 10px;
            height: 10px;
            border-radius: 50%;
            margin-right: 5px;
            animation: pulse 2s infinite;
        }
        
        .status-active { background: #00ff88; }
        .status-inactive { background: #ff4444; }
        
        @keyframes pulse {
            0% { opacity: 1; }
            50% { opacity: 0.5; }
            100% { opacity: 1; }
        }
        
        .btn {
            padding: 10px 20px;
            background: #00ff88;
            color: #000;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            font-weight: bold;
            transition: all 0.3s;
        }
        
        .btn:hover {
            background: #00cc66;
            transform: translateY(-1px);
        }
        
        .goal-input {
            width: 100%;
            padding: 10px;
            background: #0d0d0d;
            border: 1px solid #333;
            color: #e0e0e0;
            border-radius: 4px;
            margin-bottom: 10px;
            font-size: 1em;
        }
        
        .goal-input:focus {
            outline: none;
            border-color: #00ff88;
        }
        
        .service-item {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 8px 0;
        }
        
        .chart {
            height: 150px;
            background: #0d0d0d;
            border-radius: 4px;
            margin-top: 10px;
            position: relative;
            overflow: hidden;
            display: flex;
            align-items: flex-end;
            padding: 10px;
            gap: 2px;
        }
        
        .chart-bar {
            flex: 1;
            background: #00ff88;
            opacity: 0.8;
            border-radius: 2px 2px 0 0;
            transition: height 0.3s ease;
        }
        
        .timestamp {
            color: #666;
            font-size: 0.85em;
        }
        
        .footer {
            text-align: center;
            padding: 20px;
            color: #666;
            border-top: 1px solid #333;
            margin-top: 40px;
        }
    </style>
</head>
<body>
    <div class="header">
        <h1>🧠 SentientOS Dashboard <span class="rust-badge">RUST</span></h1>
        <div id="last-update">Connecting...</div>
    </div>
    
    <div class="container">
        <div class="grid">
            <div class="card">
                <div class="card-header">
                    <div class="card-title">System Metrics</div>
                </div>
                <div>
                    <div class="metric-label">CPU Usage</div>
                    <div class="metric-value" id="cpu-percent">-</div>
                </div>
                <div>
                    <div class="metric-label">Memory Usage</div>
                    <div class="metric-value" id="memory-percent">-</div>
                </div>
                <div>
                    <div class="metric-label">Disk Usage</div>
                    <div class="metric-value" id="disk-percent">-</div>
                </div>
                <div class="chart" id="cpu-chart"></div>
            </div>
            
            <div class="card">
                <div class="card-header">
                    <div class="card-title">Service Status</div>
                </div>
                <div id="service-status">
                    <div class="service-item">
                        <span>Loading services...</span>
                    </div>
                </div>
            </div>
            
            <div class="card">
                <div class="card-header">
                    <div class="card-title">Goal Injection</div>
                </div>
                <input type="text" class="goal-input" id="goal-input" 
                       placeholder="Enter a goal (e.g., 'Check disk usage')"
                       onkeypress="if(event.key==='Enter') injectGoal()">
                <button class="btn" onclick="injectGoal()">Inject Goal</button>
                <div style="margin-top: 10px">
                    <button class="btn" style="background: #4488ff" onclick="injectTestGoal()">
                        Test Goal
                    </button>
                </div>
            </div>
        </div>
        
        <div class="card">
            <div class="card-header">
                <div class="card-title">Recent Activity</div>
                <div class="timestamp">Auto-refresh: 5s</div>
            </div>
            <div class="activity-feed" id="activity-feed">
                <div class="log-entry">Loading activity...</div>
            </div>
        </div>
    </div>
    
    <div class="footer">
        SentientOS Native Dashboard - Built with Rust 🦀
    </div>
    
    <script>
        let refreshInterval;
        let cpuHistory = new Array(30).fill(0);
        
        async function refreshData() {
            try {
                const response = await fetch('/api/dashboard/all');
                const data = await response.json();
                
                // Update system metrics
                document.getElementById('cpu-percent').textContent = 
                    data.system.cpu_percent.toFixed(1) + '%';
                document.getElementById('memory-percent').textContent = 
                    data.system.memory_percent.toFixed(1) + '%';
                document.getElementById('disk-percent').textContent = 
                    data.system.disk_usage.toFixed(1) + '%';
                
                // Update CPU chart
                cpuHistory.push(data.system.cpu_percent);
                cpuHistory.shift();
                updateChart();
                
                // Update service status
                const serviceHtml = data.services.map(s => `
                    <div class="service-item">
                        <div>
                            <span class="status-indicator status-${s.status === 'running' ? 'active' : 'inactive'}"></span>
                            ${s.name}
                        </div>
                        <span style="color: #666">${s.status}</span>
                    </div>
                `).join('');
                document.getElementById('service-status').innerHTML = serviceHtml;
                
                // Update activity feed
                const activityHtml = data.activity.slice(0, 20).map(a => {
                    const time = new Date(a.timestamp).toLocaleTimeString();
                    const goalShort = a.goal.length > 60 ? 
                        a.goal.substring(0, 60) + '...' : a.goal;
                    
                    return `
                        <div class="log-entry log-${a.success ? 'success' : 'error'}">
                            <div style="display: flex; justify-content: space-between;">
                                <strong>${time}</strong>
                                <span style="color: #00ff88">reward: ${a.reward.toFixed(2)}</span>
                            </div>
                            <div style="margin-top: 4px">${goalShort}</div>
                            ${a.output ? `<div style="margin-top: 4px; color: #666; font-size: 0.85em">${a.output.substring(0, 100)}...</div>` : ''}
                        </div>
                    `;
                }).join('');
                
                document.getElementById('activity-feed').innerHTML = 
                    activityHtml || '<div class="log-entry">No recent activity</div>';
                
                // Update timestamp
                document.getElementById('last-update').textContent = 
                    'Last update: ' + new Date().toLocaleTimeString();
                    
            } catch (error) {
                console.error('Failed to refresh data:', error);
                document.getElementById('last-update').textContent = 'Connection error';
            }
        }
        
        function updateChart() {
            const chart = document.getElementById('cpu-chart');
            const max = Math.max(...cpuHistory, 1);
            
            chart.innerHTML = cpuHistory.map(value => {
                const height = (value / max) * 100;
                return `<div class="chart-bar" style="height: ${height}%"></div>`;
            }).join('');
        }
        
        async function injectGoal() {
            const input = document.getElementById('goal-input');
            const goal = input.value.trim();
            
            if (!goal) {
                alert('Please enter a goal');
                return;
            }
            
            try {
                const response = await fetch('/api/goal/inject', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ goal, priority: 'medium' })
                });
                
                if (response.ok) {
                    const result = await response.json();
                    input.value = '';
                    
                    // Show success feedback
                    input.style.borderColor = '#00ff88';
                    setTimeout(() => {
                        input.style.borderColor = '#333';
                    }, 1000);
                    
                    // Refresh immediately to show new goal
                    refreshData();
                } else {
                    const error = await response.text();
                    alert('Failed to inject goal: ' + error);
                }
            } catch (error) {
                console.error('Failed to inject goal:', error);
                alert('Error injecting goal: ' + error.message);
            }
        }
        
        function injectTestGoal() {
            const testGoals = [
                "Check system health and resource usage",
                "Monitor disk I/O patterns for anomalies",
                "Analyze memory usage by top processes",
                "Review network connections for unusual activity",
                "Scan system logs for recent errors"
            ];
            
            const goal = testGoals[Math.floor(Math.random() * testGoals.length)];
            document.getElementById('goal-input').value = goal;
            injectGoal();
        }
        
        // Start auto-refresh
        refreshData();
        refreshInterval = setInterval(refreshData, 5000);
        
        // Cleanup on page unload
        window.addEventListener('beforeunload', () => {
            if (refreshInterval) clearInterval(refreshInterval);
        });
    </script>
</body>
</html>
"#;