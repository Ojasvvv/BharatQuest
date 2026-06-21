use apatheia_engine::RuntimePool;
use apatheia_telemetry::ExecutionMetrics;
use std::sync::Arc;
use tokio::sync::broadcast;
use serde::Serialize;
use std::sync::Mutex;
use std::collections::HashMap;
use std::time::Instant;
use dashmap::DashMap;
use governor::DefaultDirectRateLimiter;
use rusqlite::Connection;

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
    pub valid_api_keys: Arc<Vec<String>>,
    pub rate_limiters: Arc<DashMap<String, DefaultDirectRateLimiter>>,
    pub db: Arc<Mutex<Connection>>,
}

impl AppState {
    pub fn new(pool: RuntimePool) -> Self {
        let (metrics_tx, _) = broadcast::channel(100);
        
        let api_keys_env = std::env::var("APATHEIA_API_KEYS").unwrap_or_default();
        let valid_api_keys = api_keys_env
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
            
        let conn = Connection::open("metrics.db").expect("Failed to open SQLite database");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS executions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                request_id TEXT NOT NULL,
                status TEXT NOT NULL,
                language TEXT NOT NULL,
                instance_clone_time_us INTEGER NOT NULL,
                execution_time_us INTEGER NOT NULL,
                memory_marshal_us INTEGER NOT NULL,
                total_time_us INTEGER NOT NULL,
                fuel_consumed INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).expect("Failed to create executions table");

        Self {
            pool: Arc::new(pool),
            metrics_tx,
            retry_counts: Arc::new(Mutex::new(HashMap::new())),
            valid_api_keys: Arc::new(valid_api_keys),
            rate_limiters: Arc::new(DashMap::new()),
            db: Arc::new(Mutex::new(conn)),
        }
    }
}
