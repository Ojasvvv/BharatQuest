//! Apatheia API Server
//!
//! Axum-based REST + WebSocket server for the Apatheia code sandbox.
//!
//! Endpoints (planned):
//! - POST /execute  — Submit JS code for sandboxed execution, returns result + metrics.
//! - GET  /ws       — WebSocket endpoint for streaming execution output.
//! - GET  /health   — Health check with engine readiness status.
//! - GET  /metrics  — Aggregated execution metrics (latency histograms, etc.)

use axum::{routing::get, Router};

#[tokio::main]
async fn main() {
    // Initialize tracing
    apatheia_telemetry::init_tracing();

    tracing::info!("Apatheia API server starting");

    let app = Router::new()
        .route("/health", get(health));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind to port 3000");

    tracing::info!("Listening on 0.0.0.0:3000");

    axum::serve(listener, app)
        .await
        .expect("server error");
}

async fn health() -> &'static str {
    "ok"
}
