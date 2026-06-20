use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use wasmtime::InstancePre;
use wasmtime::Store;
use wasmtime_wasi::p2::pipe::MemoryOutputPipe;
use wasmtime_wasi::p1::WasiP1Ctx;
use wasmtime_wasi::WasiCtxBuilder;

use crate::error::{EngineError, JsError, JsErrorType, status_codes};
use apatheia_telemetry::ExecutionMetrics;
use crate::ExecutionResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Runtime {
    JavaScript,
    Python,
}

/// Determines which execution path the engine uses for this runtime.
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeStyle {
    /// Reactor module: exports _initialize, alloc_buffer, eval_code,
    /// read_output. Used for QuickJS. Supports InstancePre + CoW cloning.
    Reactor,

    /// Command module: exports _start, reads code from WASI argv -c,
    /// writes output to WASI stdio. Used for MicroPython.
    /// Cannot use InstancePre — instantiated fresh per request.
    Command,
}

impl Runtime {
    pub fn style(&self) -> RuntimeStyle {
        match self {
            Runtime::JavaScript => RuntimeStyle::Reactor,
            Runtime::Python     => RuntimeStyle::Command,
        }
    }

    pub fn wasm_path(&self) -> PathBuf {
        let base = std::env::var("WASM_BINARY_DIR")
            .unwrap_or_else(|_| "./wasm-runtimes".to_string());
        match self {
            Runtime::JavaScript => PathBuf::from(&base).join("quickjs.wasm"),
            Runtime::Python     => PathBuf::from(&base).join("micropython-wasi.wasm"),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Runtime::JavaScript => "JavaScript (QuickJS)",
            Runtime::Python     => "Python (MicroPython)",
        }
    }

    pub fn language_id(&self) -> &'static str {
        match self {
            Runtime::JavaScript => "javascript",
            Runtime::Python     => "python",
        }
    }

    pub fn runtime_notes(&self) -> Option<&'static str> {
        match self {
            Runtime::JavaScript => None,
            Runtime::Python     => Some("MicroPython 1.x — standard library subset only"),
        }
    }
}

pub struct RuntimeHandle {
    pub runtime: Runtime,
    pub pre: InstancePre<WasiP1Ctx>,
}

fn parse_output(buf: &[u8]) -> (String, String) {
    if buf.is_empty() {
        return (String::new(), String::new());
    }
    let s = String::from_utf8_lossy(buf);
    if s.starts_with('e') {
        (String::new(), s[1..].to_string())
    } else if s.starts_with('s') {
        (s[1..].to_string(), String::new())
    } else {
        (s.to_string(), String::new())
    }
}

impl RuntimeHandle {
    pub async fn execute(
        &self,
        code: &str,
        fuel_limit: u64,
        timeout_ms: u64,
        _memory_limit_mb: u32,
    ) -> Result<ExecutionResult, EngineError> {
        let code = code.to_string();
        let pre = self.pre.clone();
        let runtime_type = self.runtime;

        let execute_future = tokio::task::spawn_blocking(move || {
            let total_start = Instant::now();

            // --- Create fresh WASI context with pipe-captured stdio ---
            let stdout_pipe = MemoryOutputPipe::new(256 * 1024);
            let stderr_pipe = MemoryOutputPipe::new(256 * 1024);

            let wasi_ctx = WasiCtxBuilder::new()
                .stdout(stdout_pipe.clone())
                .stderr(stderr_pipe.clone())
                .allow_blocking_current_thread(true)
                .build_p1();

            let engine = pre.module().engine().clone();
            let mut store = Store::new(&engine, wasi_ctx);

            // Set fuel budget for this execution.
            store
                .set_fuel(fuel_limit)
                .map_err(|e| EngineError::Instantiation(e.into()))?;

            // --- Clone instance from InstancePre (timed separately) ---
            let clone_start = Instant::now();
            let instance = pre
                .instantiate(&mut store)
                .map_err(|e| EngineError::Instantiation(e.into()))?;

            // Call _initialize to run WASI libc CRT global constructors.
            let init_func = instance
                .get_typed_func::<(), ()>(&mut store, "_initialize")
                .map_err(|e| EngineError::Instantiation(e.into()))?;

            init_func
                .call(&mut store, ())
                .map_err(|e| EngineError::Instantiation(e.into()))?;

            let clone_us = clone_start.elapsed().as_micros() as u64;

            // --- Memory Marshaling ---
            let marshal_start = Instant::now();

            let alloc_buffer_func = instance
                .get_typed_func::<u32, u32>(&mut store, "alloc_buffer")
                .map_err(|e| EngineError::MemoryMarshal(e.to_string()))?;

            let code_bytes = code.as_bytes();
            let len = code_bytes.len() as u32;

            let ptr = alloc_buffer_func
                .call(&mut store, len)
                .map_err(|e| EngineError::MemoryMarshal(e.to_string()))?;

            let memory = instance
                .get_memory(&mut store, "memory")
                .ok_or_else(|| EngineError::MemoryMarshal("Failed to find exported memory".to_string()))?;

            memory
                .write(&mut store, ptr as usize, code_bytes)
                .map_err(|e| EngineError::MemoryMarshal(e.to_string()))?;

            let memory_marshal_us = marshal_start.elapsed().as_micros() as u64;

            // --- Evaluate Code ---
            let eval_start = Instant::now();
            let eval_result = match runtime_type {
                Runtime::JavaScript => {
                    let eval_func = instance
                        .get_typed_func::<(u32, u32), i32>(&mut store, "eval_js")
                        .map_err(|e| EngineError::EvalError(e.to_string()))?;
                    eval_func.call(&mut store, (ptr, len))
                }
                _ => {
                    let eval_func = instance
                        .get_typed_func::<u32, i32>(&mut store, "eval_code")
                        .map_err(|e| EngineError::EvalError(e.to_string()))?;
                    eval_func.call(&mut store, len)
                }
            };

            let eval_us = eval_start.elapsed().as_micros() as u64;
            let fuel_consumed = fuel_limit.saturating_sub(store.get_fuel().unwrap_or(0));

            let mut metrics = ExecutionMetrics {
                instance_clone_time_us: clone_us,
                memory_marshal_us,
                execution_time_us: eval_us,
                total_time_us: 0, // Set later
                fuel_consumed,
            };

            // --- Process the result and read outputs ---
            let result_code = match eval_result {
                Ok(status) => status,
                Err(e) => {
                    // Determine if fuel exhaustion or other Trap
                    if let Some(trap) = e.downcast_ref::<wasmtime::Trap>() {
                        if *trap == wasmtime::Trap::OutOfFuel {
                            metrics.total_time_us = total_start.elapsed().as_micros() as u64;
                            return Err(EngineError::FuelExhausted {
                                fuel_limit,
                                metrics,
                            });
                        }
                    }
                    metrics.total_time_us = total_start.elapsed().as_micros() as u64;
                    return Err(EngineError::EvalError(e.to_string()));
                }
            };

            let (stdout, stderr) = match runtime_type {
                Runtime::JavaScript => {
                    let stdout_bytes = stdout_pipe.contents();
                    let stderr_bytes = stderr_pipe.contents();
                    (String::from_utf8_lossy(&stdout_bytes).into_owned(), String::from_utf8_lossy(&stderr_bytes).into_owned())
                }
                _ => {
                    let read_output_func = instance.get_typed_func::<(), u32>(&mut store, "read_output")
                        .map_err(|e| EngineError::MemoryMarshal(e.to_string()))?;
                    let out_ptr = read_output_func.call(&mut store, ()).unwrap();
                    let memory = instance.get_memory(&mut store, "memory").unwrap();
                    let mut buf = Vec::new();
                    let mut i = out_ptr as usize;
                    let mut byte = [0u8; 1];
                    while memory.read(&store, i, &mut byte).is_ok() && byte[0] != 0 {
                        buf.push(byte[0]);
                        i += 1;
                    }
                    parse_output(&buf)
                }
            };

            // 0 = Success
            // 1 = SyntaxError / ParseError
            // 2 = RuntimeError / Exception thrown during eval
            let error = if result_code != 0 {
                Some(JsError {
                    message: stderr.clone(),
                    stack_trace: None,
                    error_type: if result_code == status_codes::PARSE_ERROR {
                        JsErrorType::Parse
                    } else {
                        JsErrorType::Runtime
                    },
                })
            } else {
                None
            };

            metrics.total_time_us = total_start.elapsed().as_micros() as u64;

            let runtime_notes = match runtime_type {
                Runtime::Python => Some("MicroPython (WASI)".to_string()),
                _ => None,
            };

            Ok(ExecutionResult {
                stdout,
                stderr,
                error,
                status_code: result_code,
                metrics,
                runtime_notes,
            })
        });

        // Wrap the spawned blocking task in a wall-clock timeout
        match tokio::time::timeout(Duration::from_millis(timeout_ms), execute_future).await {
            Ok(Ok(result)) => result,
            Ok(Err(join_err)) => Err(EngineError::EvalError(join_err.to_string())),
            Err(_) => {
                // Time out elapsed
                let metrics = ExecutionMetrics {
                    instance_clone_time_us: 0,
                    memory_marshal_us: 0,
                    execution_time_us: 0,
                    total_time_us: timeout_ms * 1000,
                    fuel_consumed: 0,
                };
                Err(EngineError::WallClockTimeout {
                    timeout_ms,
                    metrics,
                })
            }
        }
    }
}

/// Execute a Python script via the MicroPython command module.
///
/// MicroPython runs as a WASI command: argv = ["micropython", "-c", code]
/// stdout/stderr are captured via MemoryOutputPipe.
/// There is no alloc_buffer/eval_code FFI — the module is a black box.
pub async fn execute_python(
    module: &wasmtime::Module,
    linker: &wasmtime::Linker<WasiP1Ctx>,
    engine: &wasmtime::Engine,
    code: &str,
    fuel_limit: u64,
    timeout_ms: u64,
) -> Result<ExecutionResult, EngineError> {

    let code = code.to_string();
    let fuel_limit = fuel_limit;
    let timeout_ms = timeout_ms;
    let module = module.clone();
    let linker = linker.clone();
    let engine = engine.clone();

    tokio::task::spawn_blocking(move || {
        // Measure instance creation time
        let t_start = std::time::Instant::now();

        // Build WASI context with argv and captured stdio
        let stdout_pipe = MemoryOutputPipe::new(65536);
        let stderr_pipe = MemoryOutputPipe::new(65536);

        let wasi_ctx = WasiCtxBuilder::new()
            .args(&["micropython", "-c", &code])
            .stdout(stdout_pipe.clone())
            .stderr(stderr_pipe.clone())
            .build_p1();

        let instance_clone_time_us = t_start.elapsed().as_micros() as u64;

        // Create store with WASI context and fuel
        let mut store = Store::new(&engine, wasi_ctx);
        store.set_fuel(fuel_limit)
             .map_err(|e| EngineError::Internal(e.to_string()))?;


        // Wall-clock timeout
        let deadline = std::time::Instant::now()
            + std::time::Duration::from_millis(timeout_ms);

        // Instantiate
        let t_exec_start = std::time::Instant::now();
        let instance = linker.instantiate(&mut store, &module)
            .map_err(|e| EngineError::Internal(e.to_string()))?;

        // Call _start (the WASI entry point)
        let start_fn = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .map_err(|e| EngineError::Internal(
                format!("_start not found: {}", e)
            ))?;

        let exec_result = start_fn.call(&mut store, ());
        let execution_time_us = t_exec_start.elapsed().as_micros() as u64;

        // Check wall-clock
        if std::time::Instant::now() > deadline {
            return Err(EngineError::WallClockTimeout {
                timeout_ms,
                metrics: ExecutionMetrics {
                    instance_clone_time_us,
                    execution_time_us,
                    memory_marshal_us: 0,
                    total_time_us: t_start.elapsed().as_micros() as u64,
                    fuel_consumed: fuel_limit - store.get_fuel().unwrap_or(0),
                }
            });
        }

        // Read captured output
        let stdout_bytes = stdout_pipe.contents();
        let stderr_bytes = stderr_pipe.contents();
        let stdout = String::from_utf8_lossy(&stdout_bytes).to_string();
        let stderr = String::from_utf8_lossy(&stderr_bytes).to_string();

        // Total time
        let total_time_us = t_start.elapsed().as_micros() as u64;
        // memory_marshal_us: MicroPython doesn't have a separate marshal phase.
        let memory_marshal_us = 0u64;

        // Interpret the result
        match exec_result {
            Ok(()) => {
                // Exit code 0 = success
                Ok(ExecutionResult {
                    stdout,
                    stderr: String::new(), // We discard stderr on success or it's empty
                    error: None,
                    status_code: 0,
                    metrics: ExecutionMetrics {
                        instance_clone_time_us,
                        execution_time_us,
                        memory_marshal_us,
                        total_time_us,
                        fuel_consumed: fuel_limit
                            - store.get_fuel().unwrap_or(0),
                    },
                    runtime_notes: Runtime::Python.runtime_notes()
                        .map(|s| s.to_string()),
                })
            }
            Err(e) => {
                // Check if it's a fuel exhaustion trap
                if let Some(trap) = e.downcast_ref::<wasmtime::Trap>() {
                    if *trap == wasmtime::Trap::OutOfFuel {
                        return Err(EngineError::FuelExhausted {
                            fuel_limit,
                            metrics: ExecutionMetrics {
                                instance_clone_time_us,
                                execution_time_us,
                                memory_marshal_us,
                                total_time_us,
                                fuel_consumed: fuel_limit - store.get_fuel().unwrap_or(0),
                            }
                        });
                    }
                }
                // Check if it's a WASI exit (normal exit with code)
                if let Some(exit) = e.downcast_ref::<wasmtime_wasi::I32Exit>() {
                    // Non-zero exit = Python raised an unhandled exception
                    return Ok(ExecutionResult {
                        stdout,
                        stderr: stderr.clone(), // contains the Python traceback
                        error: Some(JsError {
                            message: stderr,
                            stack_trace: None,
                            error_type: JsErrorType::Runtime,
                        }),
                        status_code: exit.0,
                        metrics: ExecutionMetrics {
                            instance_clone_time_us,
                            execution_time_us,
                            memory_marshal_us,
                            total_time_us,
                            fuel_consumed: fuel_limit
                                - store.get_fuel().unwrap_or(0),
                        },
                        runtime_notes: Runtime::Python.runtime_notes()
                            .map(|s| s.to_string()),
                    });
                }
                // Other trap
                Err(EngineError::Internal(e.to_string()))
            }
        }
    })
    .await
    .map_err(|e| EngineError::Internal(e.to_string()))?
}
