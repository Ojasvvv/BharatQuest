use apatheia_engine::RuntimePool;
use apatheia_telemetry::ExecutionMetrics;
use std::sync::Arc;
use tokio::sync::broadcast;
use serde::Serialize;
use std::sync::Mutex;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Clone, Serialize)]
pub struct StreamEvent {
    pub request_id: String,
    pub language: String,
    pub status: String,
    pub metrics: ExecutionMetrics,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: Arc<RuntimePool>,
    pub metrics_tx: broadcast::Sender<StreamEvent>,
    pub retry_counts: Arc<Mutex<HashMap<String, (u8, Instant)>>>,
}

impl AppState {
    pub fn new(pool: RuntimePool) -> Self {
        let (metrics_tx, _) = broadcast::channel(100);
        Self {
            pool: Arc::new(pool),
            metrics_tx,
            retry_counts: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}
