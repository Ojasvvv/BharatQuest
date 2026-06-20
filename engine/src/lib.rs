//! Apatheia Engine
//!
//! Core WASM execution engine. Responsibilities:
//! - Load quickjs.wasm into a Wasmtime Engine with an InstancePre snapshot at startup.
//! - On each request: clone a fresh Instance via CoW semantics from InstancePre,
//!   write the JS source into WASM linear memory, call the exported eval function,
//!   read stdout/stderr from linear memory, then drop the Instance (zero-cost teardown).
//! - Fuel metering via Wasmtime's `Fuel` system for instruction-count limits.
//! - Wall-clock watchdog via `tokio::time::timeout` — fuel alone is NOT sufficient
//!   since instruction count doesn't bound wall-clock time evenly.
//! - Memory marshaling between host and WASM linear memory.
//!
//! Sandboxing note: WASM provides memory-safe isolation by construction via linear
//! memory isolation. This relies on Wasmtime's correctness and is not provably
//! immune to sandbox-escape bugs in the runtime itself.

pub mod error;
pub mod sandbox;
#[cfg(test)]
mod debug_test;

pub use error::{EngineError, JsError, JsErrorType};
pub use sandbox::{ExecutionResult, SandboxConfig, SandboxEngine};
