// Minimal Hive Server for SentientOS
// Collects and shares anonymous fix patches across instances

use axum::{
    routing::{get, post},
    Router, Json,
    http::StatusCode,
    extract::State,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tokio;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HiveDelta {
    id: String,
    timestamp: SystemTime,
    machine_id: String,
    error_fingerprint: String,
    fix_description: String,
    patch_content: String,
    success_rate: f32,
    test_results: Vec<TestResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestResult {
    success: bool,
    output: String,
    duration_ms: u64,
}

#[derive(Clone)]
struct AppState {
    deltas: Arc<Mutex<HashMap<String, Vec<HiveDelta>>>>,
    stats: Arc<Mutex<ServerStats>>,
}

#[derive(Debug, Default)]
struct ServerStats {
    total_deltas: u64,
    unique_errors: u64,
    successful_fixes: u64,
    machines_connected: u64,
}

#[tokio::main]
async fn main() {
    // Initialize logging
    env_logger::init();

    let state = AppState {
        deltas: Arc::new(Mutex::new(HashMap::new())),
        stats: Arc::new(Mutex::new(ServerStats::default())),
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/api/v1/delta", post(submit_delta))
        .route("/api/v1/deltas/:fingerprint", get(get_deltas))
        .route("/api/v1/stats", get(get_stats))
        .route("/health", get(health_check))
        .with_state(state);

    let addr = "0.0.0.0:8888";
    println!("🐝 Hive Server starting on {}", addr);
    
    axum::Server::bind(&addr.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn index() -> &'static str {
    r#"
    🐝 SentientOS Hive Server
    
    API Endpoints:
    POST /api/v1/delta - Submit a fix delta
    GET  /api/v1/deltas/:fingerprint - Get fixes for an error fingerprint
    GET  /api/v1/stats - Get server statistics
    GET  /health - Health check
    "#
}

async fn submit_delta(
    State(state): State<AppState>,
    Json(delta): Json<HiveDelta>,
) -> Result<Json<SubmitResponse>, StatusCode> {
    let fingerprint = delta.error_fingerprint.clone();
    
    // Update stats
    {
        let mut stats = state.stats.lock().unwrap();
        stats.total_deltas += 1;
        if delta.test_results.iter().any(|r| r.success) {
            stats.successful_fixes += 1;
        }
    }
    
    // Store delta
    {
        let mut deltas = state.deltas.lock().unwrap();
        deltas.entry(fingerprint.clone())
            .or_insert_with(Vec::new)
            .push(delta);
        
        // Update unique errors count
        let mut stats = state.stats.lock().unwrap();
        stats.unique_errors = deltas.len() as u64;
    }
    
    Ok(Json(SubmitResponse {
        accepted: true,
        delta_id: fingerprint,
        message: "Delta recorded successfully".to_string(),
    }))
}

async fn get_deltas(
    State(state): State<AppState>,
    axum::extract::Path(fingerprint): axum::extract::Path<String>,
) -> Result<Json<DeltasResponse>, StatusCode> {
    let deltas = state.deltas.lock().unwrap();
    
    if let Some(fixes) = deltas.get(&fingerprint) {
        // Return only the most successful fixes
        let mut sorted_fixes = fixes.clone();
        sorted_fixes.sort_by(|a, b| {
            b.success_rate.partial_cmp(&a.success_rate).unwrap()
        });
        
        Ok(Json(DeltasResponse {
            fingerprint,
            deltas: sorted_fixes.into_iter().take(5).collect(),
            count: fixes.len(),
        }))
    } else {
        Ok(Json(DeltasResponse {
            fingerprint,
            deltas: vec![],
            count: 0,
        }))
    }
}

async fn get_stats(State(state): State<AppState>) -> Json<ServerStats> {
    let stats = state.stats.lock().unwrap();
    Json(stats.clone())
}

async fn health_check() -> &'static str {
    "OK"
}

#[derive(Serialize)]
struct SubmitResponse {
    accepted: bool,
    delta_id: String,
    message: String,
}

#[derive(Serialize)]
struct DeltasResponse {
    fingerprint: String,
    deltas: Vec<HiveDelta>,
    count: usize,
}

impl Clone for ServerStats {
    fn clone(&self) -> Self {
        Self {
            total_deltas: self.total_deltas,
            unique_errors: self.unique_errors,
            successful_fixes: self.successful_fixes,
            machines_connected: self.machines_connected,
        }
    }
}

// Cargo.toml for hive-server:
/*
[package]
name = "hive-server"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.6"
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
env_logger = "0.10"
*/