//! Apatheia FFI Bridge
//!
//! Provides the SSRF-firewalled `fetch()` host function that QuickJS guest code
//! can call. Isolated as its own crate for independent unit testing.
//!
//! Design constraint: QuickJS's C eval loop is synchronous, but our Rust host call
//! (reqwest) is async. We must NOT block the tokio executor thread on the HTTP call.
//! The proper solution (Phase 4) involves yielding from the WASM guest back to the
//! host, performing the async HTTP call, then resuming the guest with the result.

pub mod error;

pub use error::FfiBridgeError;
