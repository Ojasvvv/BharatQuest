use apatheia_engine::RuntimePool;
use apatheia_telemetry::ExecutionMetrics;
use std::sync::Arc;
use tokio::sync::broadcast;
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct StreamEvent {
    pub request_id: String,
    pub status: String,
    pub metrics: ExecutionMetrics,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: Arc<RuntimePool>,
    pub metrics_tx: broadcast::Sender<StreamEvent>,
}

impl AppState {
    pub fn new(pool: RuntimePool) -> Self {
        let (metrics_tx, _) = broadcast::channel(100);
        Self {
            pool: Arc::new(pool),
            metrics_tx,
        }
    }
}
