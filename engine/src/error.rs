//! Engine error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("WASM instantiation failed: {0}")]
    Instantiation(#[from] anyhow::Error),

    #[error("Execution exceeded fuel limit ({fuel_limit} units)")]
    FuelExhausted { fuel_limit: u64 },

    #[error("Execution exceeded wall-clock timeout ({timeout_ms}ms)")]
    WallClockTimeout { timeout_ms: u64 },

    #[error("Memory marshaling error: {0}")]
    MemoryMarshal(String),

    #[error("QuickJS eval error: {0}")]
    EvalError(String),
}
