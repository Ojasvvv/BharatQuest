//! Apatheia Telemetry
//!
//! Shared observability infrastructure: tracing initialization, metrics structs,
//! and latency recording types used by the engine and API crates.
//!
//! Design note: cold-instantiation time and total request latency (including FFI/JSON
//! overhead) are tracked as SEPARATE measurements — never conflated.

use serde::Serialize;

/// Latency breakdown for a single execution request.
///
/// All durations are in microseconds to avoid floating-point ambiguity.
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionMetrics {
    /// Time to instantiate a fresh WASM Instance from the InstancePre snapshot (µs).
    pub instance_clone_time_us: u64,
    /// Time spent inside the QuickJS eval call (µs).
    pub execution_time_us: u64,
    /// Time to marshal data in/out of WASM linear memory (µs).
    pub memory_marshal_us: u64,
    /// Total wall-clock time from request receipt to response send (µs).
    pub total_time_us: u64,
    /// Fuel consumed by this execution (Wasmtime fuel units).
    pub fuel_consumed: u64,
}

/// Initialize the global tracing subscriber.
///
/// Call this once at server startup.
pub fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();
}
