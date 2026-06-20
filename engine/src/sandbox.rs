//! Apatheia Sandbox Engine
//!
//! The core WASM-based JS execution sandbox. Each `SandboxEngine` holds a
//! pre-compiled, pre-linked QuickJS WASM module (`InstancePre`). On each
//! `execute()` call it:
//!
//! 1. Creates a fresh `Store` with its own WASI context (pipes for stdout/stderr)
//! 2. Clones a fresh instance from `InstancePre` (CoW semantics via pooling allocator)
//! 3. Writes the JS source into WASM linear memory via `alloc_buffer` + `memory.write()`
//! 4. Calls `eval_js`, wrapped in a `tokio::time::timeout` wall-clock backstop
//! 5. Reads stdout/stderr from both WASI pipes and linear-memory buffers
//! 6. Drops the instance (the Store going out of scope reclaims the pooling slot)
//!
//! **Isolation guarantee**: Every execution gets a fresh instance. No global state
//! leaks between executions because the instance (and its linear memory) is brand new.
//!
//! **Three distinct timing metrics** (never merged):
//! - `instance_clone_time_us`: InstancePre → live instance
//! - `js_execution_time_us`: eval_js call only
//! - `total_time_us`: full round trip including memory marshaling

use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use tracing::info;
use wasmtime::{
    Config, Engine, InstanceAllocationStrategy, InstancePre, Linker, Memory, Module,
    PoolingAllocationConfig, Store, Trap, TypedFunc,
};
use wasmtime_wasi::pipe::MemoryOutputPipe;
use wasmtime_wasi::preview1::WasiP1Ctx;
use wasmtime_wasi::WasiCtxBuilder;

use crate::error::{status_codes, EngineError, JsError, JsErrorType};
use apatheia_telemetry::ExecutionMetrics;

/// Configuration for the sandbox engine.
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Path to the compiled quickjs.wasm module.
    pub wasm_path: PathBuf,
    /// Fuel limit per execution (Wasmtime fuel units).
    /// Each WASM instruction consumes ~1 fuel unit.
    pub fuel_limit: u64,
    /// Wall-clock timeout per execution. Independent of fuel —
    /// guards against host-call stalls that don't consume fuel.
    pub wall_clock_timeout: Duration,
    /// Maximum WASM linear memory size in bytes.
    /// This is enforced by the pooling allocator.
    pub max_memory_bytes: usize,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            wasm_path: PathBuf::from("quickjs-wasm/build/quickjs.wasm"),
            fuel_limit: 10_000_000,
            wall_clock_timeout: Duration::from_secs(5),
            // 256 MiB — matches the --max-memory flag in build.sh
            max_memory_bytes: 256 * 1024 * 1024,
        }
    }
}

/// Result of a successful JS execution (may contain JS-level errors).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExecutionResult {
    /// Captured stdout output from the WASM module.
    pub stdout: String,
    /// Captured stderr output from the WASM module.
    pub stderr: String,
    /// JS-level error if the eval returned a non-zero status code.
    pub error: Option<JsError>,
    /// Raw status code from eval_js (0=success, 1=runtime, 2=parse).
    pub status_code: i32,
    /// Timing and resource usage metrics.
    pub metrics: ExecutionMetrics,
}

/// The sandbox engine. Holds pre-compiled WASM state and is reused across
/// many executions. Thread-safe (`Engine` and `InstancePre` are `Send + Sync`).
pub struct SandboxEngine {
    engine: Engine,
    instance_pre: InstancePre<WasiP1Ctx>,
    config: SandboxConfig,
}

impl SandboxEngine {
    /// Create a new sandbox engine from configuration.
    ///
    /// This performs one-time setup:
    /// - Configures Wasmtime with pooling allocator + fuel metering
    /// - Compiles the QuickJS WASM module (expensive, done once)
    /// - Pre-links WASI imports into an `InstancePre` snapshot
    pub fn new(config: SandboxConfig) -> Result<Self> {
        // --- Wasmtime Engine Configuration ---
        let mut engine_config = Config::new();

        // Enable fuel consumption — every WASM instruction consumes fuel.
        // When fuel runs out, execution traps with Trap::OutOfFuel.
        engine_config.consume_fuel(true);

        // Enable Copy-on-Write memory initialization.
        // When cloning instances from InstancePre, memory pages are shared
        // until written to, making instantiation near-zero-cost.
        engine_config.memory_init_cow(true);

        // Configure the pooling allocator explicitly.
        // This pre-allocates virtual memory slots for instances, avoiding
        // per-instantiation mmap/munmap syscalls.
        let mut pool = PoolingAllocationConfig::new();
        pool.total_memories(100);
        pool.max_memory_size(config.max_memory_bytes);
        pool.total_tables(100);
        pool.table_elements(10_000);
        pool.total_core_instances(100);

        engine_config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));

        let engine = Engine::new(&engine_config)
            .context("Failed to create Wasmtime engine with pooling allocator config")?;

        // --- Load and compile the WASM module ---
        let wasm_bytes = std::fs::read(&config.wasm_path)
            .with_context(|| format!("Failed to read WASM module at {:?}", config.wasm_path))?;

        let module = Module::new(&engine, &wasm_bytes)
            .context("Failed to compile QuickJS WASM module")?;

        // --- Pre-link WASI imports ---
        // We use WASI preview1 because our WASM module was compiled with
        // wasi-sdk (which targets wasi_snapshot_preview1).
        let mut linker: Linker<WasiP1Ctx> = Linker::new(&engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx| ctx)
            .context("Failed to add WASI preview1 to linker")?;

        // Create the InstancePre — this type-checks all imports against the
        // module once, so per-execution instantiation skips this work.
        let instance_pre = linker
            .instantiate_pre(&module)
            .context("Failed to create InstancePre from linked module")?;

        info!(
            wasm_path = %config.wasm_path.display(),
            wasm_size_bytes = wasm_bytes.len(),
            fuel_limit = config.fuel_limit,
            wall_clock_timeout_ms = config.wall_clock_timeout.as_millis() as u64,
            max_memory_bytes = config.max_memory_bytes,
            "SandboxEngine initialized"
        );

        Ok(Self {
            engine,
            instance_pre,
            config,
        })
    }

    /// Execute a JS source string in an isolated sandbox.
    ///
    /// Each call gets a completely fresh WASM instance — no state leaks
    /// between executions.
    ///
    /// Returns `ExecutionResult` on successful execution (which may contain
    /// JS-level errors), or `EngineError` for infrastructure failures
    /// (fuel exhaustion, wall-clock timeout, memory limit, etc.)
    pub async fn execute(&self, js_source: &str) -> Result<ExecutionResult, EngineError> {
        let total_start = Instant::now();

        // --- Create fresh WASI context with pipe-captured stdio ---
        let stdout_pipe = MemoryOutputPipe::new(256 * 1024);
        let stderr_pipe = MemoryOutputPipe::new(256 * 1024);

        let wasi_ctx = WasiCtxBuilder::new()
            .stdout(stdout_pipe.clone())
            .stderr(stderr_pipe.clone())
            // Allow WASI to block the current thread for I/O operations.
            // Without this, WASI preview1 internally uses tokio::block_on
            // for stdio writes, which panics with "Cannot start a runtime
            // from within a runtime" when called from inside a tokio context
            // (e.g., spawn_blocking inside #[tokio::test]).
            .allow_blocking_current_thread(true)
            .build_p1();

        let mut store = Store::new(&self.engine, wasi_ctx);

        // Set fuel budget for this execution.
        store
            .set_fuel(self.config.fuel_limit)
            .map_err(|e| EngineError::Instantiation(e))?;

        // --- Clone instance from InstancePre (timed separately) ---
        let clone_start = Instant::now();
        let instance = self
            .instance_pre
            .instantiate(&mut store)
            .map_err(|e| EngineError::Instantiation(e))?;

        // Call _initialize to run WASI libc CRT global constructors.
        // Our WASM module is built as a reactor (-mexec-model=reactor), which
        // exports _initialize instead of _start. This initializes malloc,
        // stdio, and other libc state. Without this call, alloc_buffer would
        // return garbage pointers and eval_js would fail.
        let initialize: TypedFunc<(), ()> = instance
            .get_typed_func(&mut store, "_initialize")
            .map_err(|e| {
                EngineError::MemoryMarshal(format!("_initialize export not found: {e}"))
            })?;
        initialize
            .call(&mut store, ())
            .map_err(|e| EngineError::Instantiation(e.into()))?;

        let instance_clone_time_us = clone_start.elapsed().as_micros() as u64;

        // --- Get exported functions ---
        let alloc_buffer: TypedFunc<i32, i32> = instance
            .get_typed_func(&mut store, "alloc_buffer")
            .map_err(|e| {
                EngineError::MemoryMarshal(format!("alloc_buffer export not found: {e}"))
            })?;

        let eval_js: TypedFunc<(i32, i32), i32> = instance
            .get_typed_func(&mut store, "eval_js")
            .map_err(|e| EngineError::MemoryMarshal(format!("eval_js export not found: {e}")))?;

        let get_output_ptr: TypedFunc<(), i32> = instance
            .get_typed_func(&mut store, "get_output_ptr")
            .map_err(|e| {
                EngineError::MemoryMarshal(format!("get_output_ptr export not found: {e}"))
            })?;

        let get_output_len: TypedFunc<(), i32> = instance
            .get_typed_func(&mut store, "get_output_len")
            .map_err(|e| {
                EngineError::MemoryMarshal(format!("get_output_len export not found: {e}"))
            })?;

        let get_error_ptr: TypedFunc<(), i32> = instance
            .get_typed_func(&mut store, "get_error_ptr")
            .map_err(|e| {
                EngineError::MemoryMarshal(format!("get_error_ptr export not found: {e}"))
            })?;

        let get_error_len: TypedFunc<(), i32> = instance
            .get_typed_func(&mut store, "get_error_len")
            .map_err(|e| {
                EngineError::MemoryMarshal(format!("get_error_len export not found: {e}"))
            })?;

        let memory: Memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| EngineError::MemoryMarshal("memory export not found".to_string()))?;

        // --- Write JS source into WASM linear memory ---
        // QuickJS requires the input string to be null-terminated.
        let mut js_string = js_source.to_string();
        js_string.push('\0');
        let js_bytes = js_string.as_bytes();
        let alloc_len = js_bytes.len() as i32;
        let eval_len = alloc_len - 1;

        let ptr = alloc_buffer
            .call(&mut store, alloc_len)
            .map_err(|e| EngineError::MemoryMarshal(format!("alloc_buffer call failed: {e}")))?;

        if ptr == 0 {
            return Err(EngineError::MemoryMarshal(
                "alloc_buffer returned null pointer".to_string(),
            ));
        }

        memory
            .write(&mut store, ptr as usize, js_bytes)
            .map_err(|e| EngineError::MemoryMarshal(format!("memory.write failed: {e}")))?;

        // --- Execute eval_js, wrapped in wall-clock timeout ---
        let eval_start = Instant::now();

        let timeout_duration = self.config.wall_clock_timeout;
        let fuel_limit = self.config.fuel_limit;

        // Run eval_js on a raw OS thread (not tokio's thread pool).
        //
        // Why not spawn_blocking? tokio's blocking pool threads carry a
        // runtime handle, causing wasmtime-wasi to panic with "Cannot start
        // a runtime from within a runtime" when WASI internally calls
        // block_on for stdio. Raw std::thread::spawn creates a thread with
        // no tokio context, avoiding this entirely.
        //
        // tokio::time::timeout on the oneshot receiver still provides the
        // wall-clock backstop.
        let (tx, rx) = tokio::sync::oneshot::channel();
        std::thread::spawn(move || {
            let status = eval_js.call(&mut store, (ptr, eval_len));
            let _ = tx.send((status, store));
        });

        let eval_result = tokio::time::timeout(timeout_duration, rx).await;

        let js_execution_time_us = eval_start.elapsed().as_micros() as u64;

        // --- Handle timeout / channel / trap errors ---
        let (eval_status, mut store) = match eval_result {
            Err(_elapsed) => {
                // Wall-clock timeout fired before eval completed
                return Err(EngineError::WallClockTimeout {
                    timeout_ms: timeout_duration.as_millis() as u64,
                });
            }
            Ok(Err(_recv_error)) => {
                // Channel closed — the eval thread panicked or was dropped
                return Err(EngineError::EvalError(
                    "eval thread dropped without sending result".to_string(),
                ));
            }
            Ok(Ok((Err(trap_error), _store))) => {
                // WASM trap — check if it's fuel exhaustion or memory limit
                if let Some(trap) = trap_error.downcast_ref::<Trap>() {
                    match trap {
                        Trap::OutOfFuel => {
                            return Err(EngineError::FuelExhausted { fuel_limit });
                        }
                        Trap::UnreachableCodeReached => {
                            // QuickJS calls abort() when malloc fails, which
                            // compiles to `unreachable` in WASM. This typically
                            // means memory limit exceeded.
                            return Err(EngineError::MemoryLimitExceeded);
                        }
                        _ => {
                            return Err(EngineError::EvalError(format!(
                                "WASM trap: {trap_error}"
                            )));
                        }
                    }
                }
                return Err(EngineError::EvalError(format!(
                    "eval_js call failed: {trap_error}"
                )));
            }
            Ok(Ok((Ok(status), store))) => (status, store),
        };

        // --- Read output from WASI pipes ---
        let wasi_stdout = stdout_pipe.contents();
        let wasi_stderr = stderr_pipe.contents();

        let stdout_str = String::from_utf8_lossy(&wasi_stdout).to_string();
        let stderr_str = String::from_utf8_lossy(&wasi_stderr).to_string();

        // --- Read output from linear memory buffers ---
        // These capture console.log output and structured error info from wrapper.c
        let lm_output = read_linear_memory_string(&mut store, &memory, &get_output_ptr, &get_output_len);
        let lm_error = read_linear_memory_string(&mut store, &memory, &get_error_ptr, &get_error_len);

        // Combine: prefer WASI pipe output for stdout, linear memory for errors.
        // The WASI pipe captures raw fd_write output; the linear memory buffer
        // captures console.log assembled by wrapper.c. Both should be the same
        // for console.log, but WASI pipe is more reliable for raw writes.
        let final_stdout = if !stdout_str.is_empty() {
            stdout_str
        } else {
            lm_output
        };

        // For errors, the linear memory buffer has structured exception data
        let final_stderr = if !lm_error.is_empty() {
            lm_error.clone()
        } else {
            stderr_str
        };

        // --- Parse JS error if status is non-zero ---
        let js_error = match eval_status {
            status_codes::RUNTIME_ERROR => {
                let (message, stack_trace) = parse_error_output(&final_stderr);
                Some(JsError {
                    error_type: JsErrorType::Runtime,
                    message,
                    stack_trace,
                })
            }
            status_codes::PARSE_ERROR => {
                let (message, stack_trace) = parse_error_output(&final_stderr);
                Some(JsError {
                    error_type: JsErrorType::Parse,
                    message,
                    stack_trace,
                })
            }
            _ => None,
        };

        // --- Compute fuel consumed ---
        let remaining_fuel = store.get_fuel().unwrap_or(0);
        let fuel_consumed = self.config.fuel_limit.saturating_sub(remaining_fuel);

        // --- Compute total time ---
        let total_time_us = total_start.elapsed().as_micros() as u64;

        let metrics = ExecutionMetrics {
            instantiation_us: instance_clone_time_us,
            eval_us: js_execution_time_us,
            memory_marshal_us: total_time_us
                .saturating_sub(instance_clone_time_us)
                .saturating_sub(js_execution_time_us),
            total_request_us: total_time_us,
            fuel_consumed,
        };

        // --- Log the three distinct timing metrics ---
        info!(
            instance_clone_time_us = instance_clone_time_us,
            js_execution_time_us = js_execution_time_us,
            total_time_us = total_time_us,
            fuel_consumed = fuel_consumed,
            status_code = eval_status,
            "JS execution completed"
        );

        Ok(ExecutionResult {
            stdout: final_stdout,
            stderr: final_stderr,
            error: js_error,
            status_code: eval_status,
            metrics,
        })
    }
}

/// Read a string from WASM linear memory using ptr/len getter functions.
fn read_linear_memory_string(
    store: &mut Store<WasiP1Ctx>,
    memory: &Memory,
    get_ptr: &TypedFunc<(), i32>,
    get_len: &TypedFunc<(), i32>,
) -> String {
    let ptr = get_ptr.call(&mut *store, ()).unwrap_or(0);
    let len = get_len.call(&mut *store, ()).unwrap_or(0);

    if ptr == 0 || len <= 0 {
        return String::new();
    }

    let mut buf = vec![0u8; len as usize];
    match memory.read(&*store, ptr as usize, &mut buf) {
        Ok(()) => String::from_utf8_lossy(&buf).to_string(),
        Err(_) => String::new(),
    }
}

/// Parse error output from wrapper.c into (message, optional stack_trace).
///
/// wrapper.c writes the exception string first, then a newline, then the stack
/// trace. We split on the first newline to separate them.
fn parse_error_output(error_str: &str) -> (String, Option<String>) {
    let trimmed = error_str.trim();
    if trimmed.is_empty() {
        return ("Unknown error".to_string(), None);
    }

    // The error format from wrapper.c is:
    //   <exception_message>\n<stack_trace>\n
    // Split on first newline to get message vs stack
    if let Some(first_newline) = trimmed.find('\n') {
        let message = trimmed[..first_newline].trim().to_string();
        let stack = trimmed[first_newline + 1..].trim().to_string();
        let stack_opt = if stack.is_empty() { None } else { Some(stack) };
        (message, stack_opt)
    } else {
        (trimmed.to_string(), None)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Get the path to quickjs.wasm relative to the workspace root.
    fn wasm_path() -> PathBuf {
        // When running tests, the working directory is the workspace root.
        // The WASM module is at quickjs-wasm/build/quickjs.wasm.
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        PathBuf::from(manifest_dir)
            .parent()
            .unwrap()
            .join("quickjs-wasm/build/quickjs.wasm")
    }

    fn test_config() -> SandboxConfig {
        SandboxConfig {
            wasm_path: wasm_path(),
            fuel_limit: 50_000_000, // 50M — enough for normal JS, not enough for infinite loops
            wall_clock_timeout: Duration::from_secs(5),
            max_memory_bytes: 256 * 1024 * 1024, // 256 MiB
        }
    }

    fn init_tracing() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter("info")
            .with_test_writer()
            .try_init();
    }

    /// Test 1: Successful execution returns correct stdout.
    #[tokio::test]
    async fn test_successful_execution() {
        init_tracing();
        let engine = SandboxEngine::new(test_config()).expect("engine init failed");

        let result = engine
            .execute(r#"console.log("hello world")"#)
            .await
            .expect("execution failed");

        assert_eq!(result.status_code, 0, "Expected success status");
        assert!(
            result.stdout.contains("hello world"),
            "stdout should contain 'hello world', got: {:?}",
            result.stdout
        );
        assert!(result.error.is_none(), "No error expected");

        // Verify all three timing metrics are present and sensible
        assert!(result.metrics.instantiation_us > 0, "instantiation_us should be > 0");
        assert!(result.metrics.eval_us > 0, "eval_us should be > 0");
        assert!(result.metrics.total_request_us > 0, "total_request_us should be > 0");
        assert!(
            result.metrics.total_request_us >= result.metrics.instantiation_us,
            "total should be >= instantiation"
        );

        println!(
            "\n=== TIMING METRICS ===\n\
             instance_clone_time_us: {}\n\
             js_execution_time_us: {}\n\
             total_time_us: {}\n\
             fuel_consumed: {}\n\
             =====================\n",
            result.metrics.instantiation_us,
            result.metrics.eval_us,
            result.metrics.total_request_us,
            result.metrics.fuel_consumed,
        );
    }

    /// Test 2: A thrown JS exception is captured with type/message/trace,
    /// doesn't crash the host.
    #[tokio::test]
    async fn test_exception_captured() {
        init_tracing();
        let engine = SandboxEngine::new(test_config()).expect("engine init failed");

        let result = engine
            .execute(r#"throw new Error("boom")"#)
            .await
            .expect("execution should succeed at infra level");

        assert_eq!(
            result.status_code,
            status_codes::RUNTIME_ERROR,
            "Expected runtime error status"
        );
        assert!(result.error.is_some(), "Expected a JS error");

        let js_error = result.error.unwrap();
        assert!(
            matches!(js_error.error_type, JsErrorType::Runtime),
            "Expected runtime error type"
        );
        assert!(
            js_error.message.contains("boom"),
            "Error message should contain 'boom', got: {:?}",
            js_error.message
        );
        // Stack trace should be present for Error objects
        assert!(
            js_error.stack_trace.is_some(),
            "Stack trace should be present for Error objects"
        );

        println!(
            "Exception captured successfully:\n  type: {:?}\n  message: {}\n  stack: {:?}",
            js_error.error_type, js_error.message, js_error.stack_trace
        );
    }

    /// Test 3: Infinite loop traps via fuel within bounded wall-clock time.
    #[tokio::test]
    async fn test_infinite_loop_fuel_trap() {
        init_tracing();

        let mut config = test_config();
        // Use a smaller fuel limit to make the test faster
        config.fuel_limit = 1_000_000;
        config.wall_clock_timeout = Duration::from_secs(10);

        let engine = SandboxEngine::new(config).expect("engine init failed");

        let start = Instant::now();
        let result = engine.execute("while(true){}").await;
        let elapsed = start.elapsed();

        assert!(
            matches!(result, Err(EngineError::FuelExhausted { .. })),
            "Expected FuelExhausted error, got: {result:?}"
        );

        assert!(
            elapsed < Duration::from_secs(10),
            "Fuel trap should fire well within wall-clock timeout, took {:?}",
            elapsed
        );

        println!(
            "Infinite loop correctly trapped by fuel in {:?}",
            elapsed
        );
    }

    /// Test 4: Two sequential executions don't leak state (isolation test).
    ///
    /// Sets a global variable in execution A, confirms execution B
    /// cannot see it.
    #[tokio::test]
    async fn test_isolation_between_executions() {
        init_tracing();
        let engine = SandboxEngine::new(test_config()).expect("engine init failed");

        // Execution A: set a global variable
        let result_a = engine
            .execute("globalThis.secret = 42; console.log('set')")
            .await
            .expect("execution A failed");
        assert_eq!(result_a.status_code, 0);
        assert!(result_a.stdout.contains("set"));

        // Execution B: check if the global exists
        let result_b = engine
            .execute("console.log(typeof globalThis.secret)")
            .await
            .expect("execution B failed");
        assert_eq!(result_b.status_code, 0);

        assert!(
            result_b.stdout.contains("undefined"),
            "Execution B should NOT see globalThis.secret from execution A. \
             Expected 'undefined', got: {:?}",
            result_b.stdout
        );

        println!(
            "Isolation confirmed: execution B sees typeof secret = {:?}",
            result_b.stdout.trim()
        );
    }

    /// Test 5: Memory limit enforcement — allocate beyond the configured cap,
    /// confirm a trap rather than host-process OOM.
    #[tokio::test]
    async fn test_memory_limit_enforcement() {
        init_tracing();

        let mut config = test_config();
        // Give plenty of fuel so we hit memory limits, not fuel limits
        config.fuel_limit = 500_000_000;
        config.wall_clock_timeout = Duration::from_secs(10);

        let engine = SandboxEngine::new(config).expect("engine init failed");

        // This JS tries to allocate unbounded arrays until it hits the memory limit.
        // QuickJS has its own 16 MiB internal limit (set in wrapper.c), which will
        // cause it to throw an InternalError or abort.
        let result = engine
            .execute("let a = []; while(true) { a.push(new Array(1000000)); }")
            .await;

        // Should either trap (MemoryLimitExceeded/FuelExhausted) or return a JS error.
        // It must NOT cause a host-process OOM.
        match &result {
            Err(EngineError::MemoryLimitExceeded) => {
                println!("Memory limit enforced via WASM trap (unreachable)");
            }
            Err(EngineError::FuelExhausted { .. }) => {
                println!("Memory allocation loop exhausted fuel before OOM");
            }
            Ok(exec_result) if exec_result.status_code != 0 => {
                println!(
                    "QuickJS internal memory limit triggered JS error: {:?}",
                    exec_result.error
                );
            }
            other => {
                panic!(
                    "Expected a memory-related trap or JS error, got: {other:?}"
                );
            }
        }

        println!("Host process survived — no OOM crash");
    }
}
