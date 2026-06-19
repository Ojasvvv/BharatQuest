//! Apatheia QuickJS WASM
//!
//! Build infrastructure for cross-compiling the QuickJS interpreter to wasm32-wasi.
//!
//! This crate does NOT contain prebuilt binaries. Instead, it provides a reproducible
//! build script (`build.rs`) that compiles vendored QuickJS C sources using a WASI SDK
//! toolchain, producing a `quickjs.wasm` module that the engine crate loads at startup.
//!
//! The resulting WASM module exports an `eval` function that:
//! 1. Accepts a pointer + length to a JS source string in linear memory
//! 2. Evaluates the JS using the QuickJS interpreter
//! 3. Writes stdout/stderr output to designated regions in linear memory
//! 4. Returns a status code

/// Path where the built quickjs.wasm will be placed (relative to OUT_DIR).
pub const WASM_OUTPUT_FILENAME: &str = "quickjs.wasm";
