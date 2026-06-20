use std::path::PathBuf;
use std::time::Duration;

use crate::error::JsError;
use apatheia_telemetry::ExecutionMetrics;

/// Configuration for the sandbox engine (kept for backward compatibility where needed)
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub wasm_path: PathBuf,
    pub fuel_limit: u64,
    pub wall_clock_timeout: Duration,
    pub max_memory_bytes: usize,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            wasm_path: PathBuf::from("quickjs-wasm/build/quickjs.wasm"),
            fuel_limit: 10_000_000,
            wall_clock_timeout: Duration::from_secs(5),
            max_memory_bytes: 256 * 1024 * 1024,
        }
    }
}

/// Result of a successful JS execution (may contain JS-level errors).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub error: Option<JsError>,
    pub status_code: i32,
    pub metrics: ExecutionMetrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_notes: Option<String>,
}
