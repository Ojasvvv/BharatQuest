//! Apatheia API Server
//!
//! Axum-based REST + WebSocket server for the Apatheia code sandbox.
//!
//! Endpoints (planned):
//! - POST /execute  — Submit JS code for sandboxed execution, returns result + metrics.
//! - GET  /ws       — WebSocket endpoint for streaming execution output.
//! - GET  /health   — Health check with engine readiness status.
//! - GET  /metrics  — Aggregated execution metrics (latency histograms, etc.)

mod handlers;
mod models;
mod state;

use apatheia_engine::RuntimePool;
use axum::{
    routing::{get, post},
    Router,
};
use std::path::PathBuf;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use crate::state::AppState;

pub fn build_app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/execute", post(handlers::execute_handler))
        .route("/v1/runtimes", get(handlers::runtimes_handler))
        .route("/v1/execute/stream", get(handlers::stream_metrics_handler))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

#[tokio::main]
async fn main() {
    apatheia_telemetry::init_tracing();
    tracing::info!("Apatheia API server starting");

    let pool = RuntimePool::init().await.expect("Failed to initialize RuntimePool");
    let state = AppState::new(pool);

    let retry_counts = state.retry_counts.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            let now = std::time::Instant::now();
            let mut counts = retry_counts.lock().unwrap();
            counts.retain(|_, &mut (_, timestamp)| {
                now.duration_since(timestamp).as_secs() < 600
            });
        }
    });

    let app = build_app(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .expect("failed to bind to port 8080");

    tracing::info!("Listening on 0.0.0.0:8080");

    axum::serve(listener, app)
        .await
        .expect("server error");
}

async fn health() -> &'static str {
    "ok"
}

#[cfg(test)]
mod handlers_test;
