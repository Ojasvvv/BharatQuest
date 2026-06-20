//! Engine error types.

use thiserror::Error;

/// Status codes returned by the QuickJS `eval_js` FFI function.
///
/// These must match the values in `wrapper.c`.
pub mod status_codes {
    /// JS evaluated successfully.
    pub const SUCCESS: i32 = 0;
    /// JS runtime error (exception thrown during execution).
    pub const RUNTIME_ERROR: i32 = 1;
    /// JS parse/syntax error.
    pub const PARSE_ERROR: i32 = 2;
    // Note: fuel exhaustion (conceptually code 3) is detected at the Wasmtime
    // host level via Trap::OutOfFuel. The WASM code never returns to eval_js
    // when fuel runs out, so there is no C-side code for this.
}

/// Classification of JS-level errors.
#[derive(Debug, Clone, serde::Serialize)]
pub enum JsErrorType {
    /// Exception thrown during execution (TypeError, ReferenceError, manual throw, etc.)
    Runtime,
    /// Syntax/parse error — the JS source couldn't be parsed.
    Parse,
}

/// Structured JS error with type, message, and optional stack trace.
#[derive(Debug, Clone, serde::Serialize)]
pub struct JsError {
    pub error_type: JsErrorType,
    pub message: String,
    pub stack_trace: Option<String>,
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("WASM instantiation failed: {0}")]
    Instantiation(#[from] anyhow::Error),

    #[error("Execution exceeded fuel limit ({fuel_limit} units)")]
    FuelExhausted { fuel_limit: u64 },

    #[error("Execution exceeded wall-clock timeout ({timeout_ms}ms)")]
    WallClockTimeout { timeout_ms: u64 },

    #[error("Memory limit exceeded: WASM linear memory allocation trapped")]
    MemoryLimitExceeded,

    #[error("Memory marshaling error: {0}")]
    MemoryMarshal(String),

    #[error("QuickJS eval error: {0}")]
    EvalError(String),
}
