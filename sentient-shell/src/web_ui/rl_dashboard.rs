// RL Dashboard for Web UI
// Provides real-time visualization of training progress and policy performance

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use warp::{Filter, Rejection, Reply};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

use crate::rl_training::{get_training_stats, start_training, stop_training, RLTrainingConfig};
use crate::policy_injector::{get_injector_stats, start_policy_injector, stop_policy_injector};

/// RL Dashboard state
#[derive(Debug, Clone)]
pub struct RLDashboardState {
    /// Recent reward history for graph
    reward_history: Arc<RwLock<Vec<RewardPoint>>>,
    /// Training configuration
    training_config: Arc<RwLock<Option<RLTrainingConfig>>>,
    /// Policy checkpoints
    policy_checkpoints: Arc<RwLock<Vec<PolicyCheckpoint>>>,
}

#[derive(Debug, Clone, Serialize)]
struct RewardPoint {
    episode: usize,
    reward: f32,
    timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
struct PolicyCheckpoint {
    id: String,
    episode: usize,
    reward: f32,
    created_at: DateTime<Utc>,
}

impl RLDashboardState {
    pub fn new() -> Self {
        Self {
            reward_history: Arc::new(RwLock::new(Vec::new())),
            training_config: Arc::new(RwLock::new(None)),
            policy_checkpoints: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Update reward history from training stats
    pub async fn update_rewards(&self) {
        if let Some(stats) = get_training_stats().await {
            let mut history = self.reward_history.write().await;
            
            // Add new rewards
            for (i, &reward) in stats.recent_rewards.iter().enumerate() {
                let episode = stats.current_episode.saturating_sub(stats.recent_rewards.len() - i - 1);
                
                // Check if we already have this episode
                if !history.iter().any(|p| p.episode == episode) {
                    history.push(RewardPoint {
                        episode,
                        reward,
                        timestamp: Utc::now(),
                    });
                }
            }
            
            // Keep only last 1000 points
            if history.len() > 1000 {
                history.drain(0..history.len() - 1000);
            }
        }
    }
    
    /// Load policy checkpoints
    pub async fn load_checkpoints(&self) -> Result<()> {
        use tokio::fs;
        
        let checkpoint_dir = std::path::Path::new("/var/rl_checkpoints/policies");
        if !checkpoint_dir.exists() {
            return Ok(());
        }
        
        let mut checkpoints = Vec::new();
        let mut entries = fs::read_dir(checkpoint_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                let metadata_path = entry.path().join("metadata.json");
                if metadata_path.exists() {
                    if let Ok(content) = fs::read_to_string(&metadata_path).await {
                        if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&content) {
                            checkpoints.push(PolicyCheckpoint {
                                id: entry.file_name().to_string_lossy().to_string(),
                                episode: meta["metadata"]["episode"].as_u64().unwrap_or(0) as usize,
                                reward: meta["metadata"]["best_reward"].as_f64().unwrap_or(0.0) as f32,
                                created_at: meta["created_at"].as_str()
                                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                                    .map(|dt| dt.with_timezone(&Utc))
                                    .unwrap_or_else(Utc::now),
                            });
                        }
                    }
                }
            }
        }
        
        checkpoints.sort_by_key(|c| c.episode);
        *self.policy_checkpoints.write().await = checkpoints;
        
        Ok(())
    }
}

/// Create RL dashboard routes
pub fn rl_routes(state: Arc<RLDashboardState>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let rl_page = warp::path!("rl")
        .and(warp::get())
        .map(move || warp::reply::html(RL_DASHBOARD_HTML));
    
    let rl_api = warp::path("api").and(warp::path("rl")).and(
        get_status(state.clone())
            .or(get_rewards(state.clone()))
            .or(get_checkpoints(state.clone()))
            .or(start_training_route(state.clone()))
            .or(stop_training_route(state.clone()))
            .or(start_injector_route(state.clone()))
            .or(stop_injector_route(state.clone()))
    );
    
    rl_page.or(rl_api)
}

/// Get current RL status
fn get_status(state: Arc<RLDashboardState>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("status")
        .and(warp::get())
        .and(with_state(state))
        .and_then(handle_get_status)
}

async fn handle_get_status(state: Arc<RLDashboardState>) -> Result<impl Reply, Rejection> {
    // Update rewards
    state.update_rewards().await;
    
    let training_stats = get_training_stats().await;
    let injector_stats = get_injector_stats().await;
    
    let response = json!({
        "training": training_stats.map(|s| json!({
            "is_running": s.is_running,
            "current_episode": s.current_episode,
            "total_episodes": s.total_episodes,
            "best_reward": s.best_reward,
        })),
        "injector": injector_stats.map(|s| json!({
            "is_running": s.is_running,
            "total_injections": s.total_injections,
            "success_rate": s.success_rate,
            "avg_confidence": s.avg_confidence,
            "last_injection": s.last_injection,
        })),
    });
    
    Ok(warp::reply::json(&response))
}

/// Get reward history
fn get_rewards(state: Arc<RLDashboardState>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("rewards")
        .and(warp::get())
        .and(with_state(state))
        .and_then(handle_get_rewards)
}

async fn handle_get_rewards(state: Arc<RLDashboardState>) -> Result<impl Reply, Rejection> {
    let history = state.reward_history.read().await;
    
    let response = json!({
        "rewards": history.clone(),
        "count": history.len(),
    });
    
    Ok(warp::reply::json(&response))
}

/// Get policy checkpoints
fn get_checkpoints(state: Arc<RLDashboardState>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("checkpoints")
        .and(warp::get())
        .and(with_state(state))
        .and_then(handle_get_checkpoints)
}

async fn handle_get_checkpoints(state: Arc<RLDashboardState>) -> Result<impl Reply, Rejection> {
    // Reload checkpoints
    let _ = state.load_checkpoints().await;
    
    let checkpoints = state.policy_checkpoints.read().await;
    
    let response = json!({
        "checkpoints": checkpoints.clone(),
        "count": checkpoints.len(),
    });
    
    Ok(warp::reply::json(&response))
}

/// Start training
fn start_training_route(state: Arc<RLDashboardState>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("start-training")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_state(state))
        .and_then(handle_start_training)
}

async fn handle_start_training(
    config: RLTrainingConfig,
    state: Arc<RLDashboardState>,
) -> Result<impl Reply, Rejection> {
    // Store config
    *state.training_config.write().await = Some(config.clone());
    
    // Start training
    match start_training(config).await {
        Ok(_) => Ok(warp::reply::json(&json!({
            "status": "started",
            "message": "Training started successfully"
        }))),
        Err(e) => Ok(warp::reply::json(&json!({
            "status": "error",
            "message": format!("Failed to start training: {}", e)
        }))),
    }
}

/// Stop training
fn stop_training_route(state: Arc<RLDashboardState>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("stop-training")
        .and(warp::post())
        .and_then(handle_stop_training)
}

async fn handle_stop_training() -> Result<impl Reply, Rejection> {
    match stop_training().await {
        Ok(_) => Ok(warp::reply::json(&json!({
            "status": "stopped",
            "message": "Training stopped successfully"
        }))),
        Err(e) => Ok(warp::reply::json(&json!({
            "status": "error",
            "message": format!("Failed to stop training: {}", e)
        }))),
    }
}

/// Start injector
fn start_injector_route(state: Arc<RLDashboardState>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("start-injector")
        .and(warp::post())
        .and_then(handle_start_injector)
}

async fn handle_start_injector() -> Result<impl Reply, Rejection> {
    match start_policy_injector().await {
        Ok(_) => Ok(warp::reply::json(&json!({
            "status": "started",
            "message": "Policy injector started successfully"
        }))),
        Err(e) => Ok(warp::reply::json(&json!({
            "status": "error",
            "message": format!("Failed to start injector: {}", e)
        }))),
    }
}

/// Stop injector
fn stop_injector_route(state: Arc<RLDashboardState>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("stop-injector")
        .and(warp::post())
        .and_then(handle_stop_injector)
}

async fn handle_stop_injector() -> Result<impl Reply, Rejection> {
    match stop_policy_injector().await {
        Ok(_) => Ok(warp::reply::json(&json!({
            "status": "stopped",
            "message": "Policy injector stopped successfully"
        }))),
        Err(e) => Ok(warp::reply::json(&json!({
            "status": "error",
            "message": format!("Failed to stop injector: {}", e)
        }))),
    }
}

/// Helper to pass state to handlers
fn with_state(state: Arc<RLDashboardState>) -> impl Filter<Extract = (Arc<RLDashboardState>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || state.clone())
}

/// RL Dashboard HTML
const RL_DASHBOARD_HTML: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>SentientOS RL Dashboard</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            margin: 0;
            padding: 0;
            background: #0a0a0a;
            color: #e0e0e0;
        }
        
        .container {
            max-width: 1400px;
            margin: 0 auto;
            padding: 20px;
        }
        
        .header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 30px;
            padding-bottom: 20px;
            border-bottom: 1px solid #333;
        }
        
        h1 {
            margin: 0;
            color: #00ff88;
            font-size: 28px;
        }
        
        .controls {
            display: flex;
            gap: 10px;
        }
        
        button {
            background: #00ff88;
            color: #000;
            border: none;
            padding: 10px 20px;
            border-radius: 4px;
            cursor: pointer;
            font-weight: 600;
            transition: all 0.2s;
        }
        
        button:hover {
            background: #00cc6a;
            transform: translateY(-1px);
        }
        
        button:disabled {
            background: #333;
            color: #666;
            cursor: not-allowed;
            transform: none;
        }
        
        .grid {
            display: grid;
            grid-template-columns: 2fr 1fr;
            gap: 20px;
            margin-bottom: 20px;
        }
        
        .card {
            background: #1a1a1a;
            border: 1px solid #333;
            border-radius: 8px;
            padding: 20px;
        }
        
        .card h2 {
            margin: 0 0 15px 0;
            color: #00ff88;
            font-size: 18px;
        }
        
        .status {
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 20px;
        }
        
        .status-item {
            background: #222;
            padding: 15px;
            border-radius: 6px;
        }
        
        .status-label {
            color: #888;
            font-size: 14px;
            margin-bottom: 5px;
        }
        
        .status-value {
            font-size: 24px;
            font-weight: 600;
            color: #00ff88;
        }
        
        #rewardChart {
            width: 100%;
            height: 300px;
            background: #111;
            border-radius: 4px;
            position: relative;
        }
        
        .chart-loading {
            position: absolute;
            top: 50%;
            left: 50%;
            transform: translate(-50%, -50%);
            color: #666;
        }
        
        .policy-list {
            max-height: 300px;
            overflow-y: auto;
        }
        
        .policy-item {
            background: #222;
            padding: 10px;
            margin-bottom: 10px;
            border-radius: 4px;
            display: flex;
            justify-content: space-between;
            align-items: center;
        }
        
        .policy-info {
            flex: 1;
        }
        
        .policy-episode {
            color: #888;
            font-size: 14px;
        }
        
        .policy-reward {
            color: #00ff88;
            font-weight: 600;
        }
        
        .injector-stats {
            display: grid;
            grid-template-columns: repeat(2, 1fr);
            gap: 10px;
        }
        
        .stat-box {
            background: #222;
            padding: 10px;
            border-radius: 4px;
        }
        
        .stat-label {
            color: #888;
            font-size: 12px;
        }
        
        .stat-value {
            color: #00ff88;
            font-size: 18px;
            font-weight: 600;
        }
        
        .config-form {
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 15px;
            margin-top: 20px;
        }
        
        .form-group {
            display: flex;
            flex-direction: column;
        }
        
        .form-group label {
            color: #888;
            font-size: 14px;
            margin-bottom: 5px;
        }
        
        .form-group input, .form-group select {
            background: #222;
            border: 1px solid #444;
            color: #e0e0e0;
            padding: 8px;
            border-radius: 4px;
        }
        
        .loader {
            display: inline-block;
            width: 16px;
            height: 16px;
            border: 2px solid #00ff88;
            border-radius: 50%;
            border-top-color: transparent;
            animation: spin 1s linear infinite;
        }
        
        @keyframes spin {
            to { transform: rotate(360deg); }
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🧠 SentientOS RL Dashboard</h1>
            <div class="controls">
                <button id="startTraining" onclick="startTraining()">Start Training</button>
                <button id="stopTraining" onclick="stopTraining()" disabled>Stop Training</button>
                <button id="startInjector" onclick="startInjector()">Start Injector</button>
                <button id="stopInjector" onclick="stopInjector()" disabled>Stop Injector</button>
            </div>
        </div>
        
        <div class="grid">
            <div class="card">
                <h2>📊 Reward Graph</h2>
                <div id="rewardChart">
                    <div class="chart-loading">Loading chart...</div>
                    <canvas id="rewardCanvas"></canvas>
                </div>
            </div>
            
            <div class="card">
                <h2>📈 Training Status</h2>
                <div class="status">
                    <div class="status-item">
                        <div class="status-label">Current Episode</div>
                        <div class="status-value" id="currentEpisode">-</div>
                    </div>
                    <div class="status-item">
                        <div class="status-label">Best Reward</div>
                        <div class="status-value" id="bestReward">-</div>
                    </div>
                </div>
            </div>
        </div>
        
        <div class="grid">
            <div class="card">
                <h2>🎯 Policy Injector</h2>
                <div class="injector-stats">
                    <div class="stat-box">
                        <div class="stat-label">Total Injections</div>
                        <div class="stat-value" id="totalInjections">0</div>
                    </div>
                    <div class="stat-box">
                        <div class="stat-label">Success Rate</div>
                        <div class="stat-value" id="successRate">0%</div>
                    </div>
                    <div class="stat-box">
                        <div class="stat-label">Avg Confidence</div>
                        <div class="stat-value" id="avgConfidence">0.0</div>
                    </div>
                    <div class="stat-box">
                        <div class="stat-label">Last Injection</div>
                        <div class="stat-value" id="lastInjection">Never</div>
                    </div>
                </div>
            </div>
            
            <div class="card">
                <h2>💾 Policy Checkpoints</h2>
                <div class="policy-list" id="policyList">
                    <div class="policy-item">
                        <div class="policy-info">
                            <div>No checkpoints yet</div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        
        <div class="card">
            <h2>⚙️ Training Configuration</h2>
            <div class="config-form">
                <div class="form-group">
                    <label>Agent Type</label>
                    <select id="agentType">
                        <option value="ppo">PPO</option>
                        <option value="dqn">DQN (Coming Soon)</option>
                        <option value="random">Random</option>
                    </select>
                </div>
                <div class="form-group">
                    <label>Environment</label>
                    <select id="environment">
                        <option value="goal-task">Goal Task</option>
                        <option value="jsonl">JSONL Replay</option>
                        <option value="cartpole">CartPole (Test)</option>
                    </select>
                </div>
                <div class="form-group">
                    <label>Episodes</label>
                    <input type="number" id="episodes" value="1000" min="10" max="10000">
                </div>
                <div class="form-group">
                    <label>Learning Rate</label>
                    <input type="number" id="learningRate" value="0.0003" step="0.0001" min="0.00001" max="0.1">
                </div>
            </div>
        </div>
    </div>
    
    <script>
        let rewardChart = null;
        let updateInterval = null;
        
        // Initialize
        async function init() {
            await updateStatus();
            await updateRewards();
            await updateCheckpoints();
            
            // Set up auto-refresh
            updateInterval = setInterval(async () => {
                await updateStatus();
                await updateRewards();
            }, 2000);
        }
        
        // Update status
        async function updateStatus() {
            try {
                const response = await fetch('/api/rl/status');
                const data = await response.json();
                
                // Update training status
                if (data.training) {
                    document.getElementById('currentEpisode').textContent = 
                        `${data.training.current_episode} / ${data.training.total_episodes}`;
                    document.getElementById('bestReward').textContent = 
                        data.training.best_reward.toFixed(3);
                    
                    // Update buttons
                    document.getElementById('startTraining').disabled = data.training.is_running;
                    document.getElementById('stopTraining').disabled = !data.training.is_running;
                }
                
                // Update injector status
                if (data.injector) {
                    document.getElementById('totalInjections').textContent = data.injector.total_injections;
                    document.getElementById('successRate').textContent = 
                        (data.injector.success_rate * 100).toFixed(1) + '%';
                    document.getElementById('avgConfidence').textContent = 
                        data.injector.avg_confidence.toFixed(3);
                    
                    if (data.injector.last_injection) {
                        const lastInj = new Date(data.injector.last_injection);
                        const ago = Math.floor((Date.now() - lastInj) / 1000);
                        document.getElementById('lastInjection').textContent = `${ago}s ago`;
                    }
                    
                    // Update buttons
                    document.getElementById('startInjector').disabled = data.injector.is_running;
                    document.getElementById('stopInjector').disabled = !data.injector.is_running;
                }
            } catch (error) {
                console.error('Failed to update status:', error);
            }
        }
        
        // Update reward graph
        async function updateRewards() {
            try {
                const response = await fetch('/api/rl/rewards');
                const data = await response.json();
                
                if (data.rewards && data.rewards.length > 0) {
                    drawRewardChart(data.rewards);
                }
            } catch (error) {
                console.error('Failed to update rewards:', error);
            }
        }
        
        // Draw reward chart
        function drawRewardChart(rewards) {
            const canvas = document.getElementById('rewardCanvas');
            const ctx = canvas.getContext('2d');
            const container = document.getElementById('rewardChart');
            
            // Set canvas size
            canvas.width = container.clientWidth;
            canvas.height = container.clientHeight;
            
            // Clear canvas
            ctx.fillStyle = '#111';
            ctx.fillRect(0, 0, canvas.width, canvas.height);
            
            if (rewards.length < 2) return;
            
            // Find min/max
            const minReward = Math.min(...rewards.map(r => r.reward));
            const maxReward = Math.max(...rewards.map(r => r.reward));
            const range = maxReward - minReward || 1;
            
            // Draw grid
            ctx.strokeStyle = '#333';
            ctx.lineWidth = 1;
            for (let i = 0; i <= 5; i++) {
                const y = (canvas.height / 5) * i;
                ctx.beginPath();
                ctx.moveTo(0, y);
                ctx.lineTo(canvas.width, y);
                ctx.stroke();
            }
            
            // Draw reward line
            ctx.strokeStyle = '#00ff88';
            ctx.lineWidth = 2;
            ctx.beginPath();
            
            rewards.forEach((point, i) => {
                const x = (i / (rewards.length - 1)) * canvas.width;
                const y = canvas.height - ((point.reward - minReward) / range) * canvas.height * 0.9 - canvas.height * 0.05;
                
                if (i === 0) {
                    ctx.moveTo(x, y);
                } else {
                    ctx.lineTo(x, y);
                }
            });
            
            ctx.stroke();
            
            // Draw labels
            ctx.fillStyle = '#888';
            ctx.font = '12px sans-serif';
            ctx.fillText(maxReward.toFixed(2), 5, 15);
            ctx.fillText(minReward.toFixed(2), 5, canvas.height - 5);
            
            // Hide loading message
            document.querySelector('.chart-loading').style.display = 'none';
        }
        
        // Update checkpoints
        async function updateCheckpoints() {
            try {
                const response = await fetch('/api/rl/checkpoints');
                const data = await response.json();
                
                const list = document.getElementById('policyList');
                
                if (data.checkpoints && data.checkpoints.length > 0) {
                    list.innerHTML = data.checkpoints
                        .slice(-10)  // Show last 10
                        .reverse()
                        .map(cp => `
                            <div class="policy-item">
                                <div class="policy-info">
                                    <div>${cp.id}</div>
                                    <div class="policy-episode">Episode ${cp.episode}</div>
                                </div>
                                <div class="policy-reward">${cp.reward.toFixed(3)}</div>
                            </div>
                        `).join('');
                }
            } catch (error) {
                console.error('Failed to update checkpoints:', error);
            }
        }
        
        // Start training
        async function startTraining() {
            const config = {
                agent_type: document.getElementById('agentType').value,
                environment: document.getElementById('environment').value,
                episodes: parseInt(document.getElementById('episodes').value),
                learning_rate: parseFloat(document.getElementById('learningRate').value),
                steps_per_rollout: 200,
                checkpoint_interval: 100,
                log_interval: 10,
                reward_goal_threshold: 0.8,
                observation_dim: 64,
                action_dim: 10
            };
            
            try {
                const response = await fetch('/api/rl/start-training', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(config)
                });
                const data = await response.json();
                
                if (data.status === 'started') {
                    console.log('Training started');
                } else {
                    alert('Failed to start training: ' + data.message);
                }
            } catch (error) {
                console.error('Failed to start training:', error);
            }
        }
        
        // Stop training
        async function stopTraining() {
            try {
                const response = await fetch('/api/rl/stop-training', {
                    method: 'POST'
                });
                const data = await response.json();
                
                if (data.status === 'stopped') {
                    console.log('Training stopped');
                }
            } catch (error) {
                console.error('Failed to stop training:', error);
            }
        }
        
        // Start injector
        async function startInjector() {
            try {
                const response = await fetch('/api/rl/start-injector', {
                    method: 'POST'
                });
                const data = await response.json();
                
                if (data.status === 'started') {
                    console.log('Injector started');
                } else {
                    alert('Failed to start injector: ' + data.message);
                }
            } catch (error) {
                console.error('Failed to start injector:', error);
            }
        }
        
        // Stop injector
        async function stopInjector() {
            try {
                const response = await fetch('/api/rl/stop-injector', {
                    method: 'POST'
                });
                const data = await response.json();
                
                if (data.status === 'stopped') {
                    console.log('Injector stopped');
                }
            } catch (error) {
                console.error('Failed to stop injector:', error);
            }
        }
        
        // Initialize on load
        window.addEventListener('load', init);
        
        // Cleanup on unload
        window.addEventListener('beforeunload', () => {
            if (updateInterval) {
                clearInterval(updateInterval);
            }
        });
    </script>
</body>
</html>
"#;