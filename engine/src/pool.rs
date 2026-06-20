use anyhow::Result;
use tracing::{error, info};
use wasmtime::{Config, Engine, InstanceAllocationStrategy, Linker, Module, PoolingAllocationConfig};
use wasmtime_wasi::p1::WasiP1Ctx;

use crate::runtime::{Runtime, RuntimeHandle, RuntimeStyle, execute_python};

pub struct RuntimePool {
    /// Pooling engine for Reactor modules (JavaScript/QuickJS).
    /// Uses pooling allocator for fast CoW instance cloning.
    pub engine_pooling: Engine,

    /// Standard engine for Command modules (Python/MicroPython).
    /// No pooling allocator — supports wasm_exceptions.
    pub engine_standard: Engine,

    /// Pre-compiled JS module (Reactor path).
    pub js_handle: Option<RuntimeHandle>,
    /// Pre-compiled Python module (Command path).
    pub python_module: Option<Module>,

    /// Shared linker for Python (WASI imports pre-registered).
    pub python_linker: Linker<WasiP1Ctx>,
}

impl RuntimePool {
    /// Called once at startup. Fails fast if engine setup fails.
    /// For individual WASM binaries, it attempts to load them, logging success
    /// or failure, but continues so that partial runtime availability is possible.
    pub async fn init() -> Result<Self> {
        // --- Engine 1: Pooling Engine (JavaScript) ---
        let mut pooling_config = Config::new();
        pooling_config.consume_fuel(true);
        pooling_config.memory_init_cow(true);

        let mut pool_alloc_config = PoolingAllocationConfig::new();
        pool_alloc_config.total_memories(300);
        pool_alloc_config.max_memory_size(256 * 1024 * 1024);
        pool_alloc_config.total_tables(300);
        pool_alloc_config.table_elements(10_000);
        pool_alloc_config.total_core_instances(300);

        pooling_config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool_alloc_config));
        
        let engine_pooling = Engine::new(&pooling_config)
            .map_err(|e| anyhow::anyhow!("Failed to create Wasmtime engine with pooling allocator config: {}", e))?;

        // --- Engine 2: Standard Engine (Python) ---
        let mut standard_config = Config::new();
        standard_config.consume_fuel(true);
        standard_config.memory_init_cow(true);
        standard_config.wasm_exceptions(true); // Required for MicroPython
        
        let engine_standard = Engine::new(&standard_config)
            .map_err(|e| anyhow::anyhow!("Failed to create standard Wasmtime engine: {}", e))?;

        let mut pool = RuntimePool {
            engine_pooling,
            engine_standard,
            js_handle: None,
            python_module: None,
            python_linker: Linker::new(&Engine::new(&standard_config).unwrap()), // overwritten below
        };

        // Linker for Python: register WASI imports
        let mut python_linker: Linker<WasiP1Ctx> = Linker::new(&pool.engine_standard);
        if let Err(e) = wasmtime_wasi::p1::add_to_linker_sync(&mut python_linker, |ctx| ctx) {
            error!(error = %e, "Failed to add WASI to Python linker");
        }
        
        // micropython-wasi.wasm imports a host function `host_result_cap` returning i32.
        if let Err(e) = python_linker.func_wrap("micropython_wasm", "host_result_cap", || -> i32 {
            1000000 // Just return a large capacity or 0. Let's return 0.
        }) {
            error!(error = %e, "Failed to add micropython_wasm::host_result_cap to Python linker");
        }

        // micropython-wasi.wasm imports a host function `host_call` with 6 i32 params returning i32.
        if let Err(e) = python_linker.func_wrap("micropython_wasm", "host_call", |_: i32, _: i32, _: i32, _: i32, _: i32, _: i32| -> i32 {
            0 // return failure or success, 0 is fine
        }) {
            error!(error = %e, "Failed to add micropython_wasm::host_call to Python linker");
        }
        
        pool.python_linker = python_linker;

        let runtimes = [Runtime::JavaScript, Runtime::Python];

        for runtime in runtimes {
            let path = runtime.wasm_path();
            match std::fs::read(&path) {
                Ok(wasm_bytes) => {
                    if runtime.style() == RuntimeStyle::Reactor {
                        match Module::new(&pool.engine_pooling, &wasm_bytes) {
                            Ok(module) => {
                                let mut linker: Linker<WasiP1Ctx> = Linker::new(&pool.engine_pooling);
                                if let Err(e) =
                                    wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |ctx| ctx)
                                {
                                    error!(runtime = runtime.label(), error = %e, "Failed to add WASI to linker");
                                    continue;
                                }
                                match linker.instantiate_pre(&module) {
                                    Ok(pre) => {
                                        info!(
                                            runtime = runtime.label(),
                                            path = %path.display(),
                                            size = wasm_bytes.len(),
                                            "Runtime loaded successfully"
                                        );
                                        let handle = RuntimeHandle { runtime, pre };
                                        match runtime {
                                            Runtime::JavaScript => pool.js_handle = Some(handle),
                                            Runtime::Python => {} // unreachable due to Reactor check
                                        }
                                    }
                                    Err(e) => {
                                        error!(runtime = runtime.label(), error = %e, "Failed to create InstancePre");
                                        println!("ERROR: Failed to create InstancePre for {}: {}", runtime.label(), e);
                                    }
                                }
                            }
                            Err(e) => {
                                error!(runtime = runtime.label(), error = %e, "Failed to compile WASM module");
                                println!("ERROR: Failed to compile WASM module for {}: {}", runtime.label(), e);
                            }
                        }
                    } else if runtime.style() == RuntimeStyle::Command {
                        match Module::new(&pool.engine_standard, &wasm_bytes) {
                            Ok(module) => {
                                info!(
                                    runtime = runtime.label(),
                                    path = %path.display(),
                                    size = wasm_bytes.len(),
                                    "Python Command Runtime loaded successfully"
                                );
                                pool.python_module = Some(module);
                            }
                            Err(e) => {
                                error!(runtime = runtime.label(), error = %e, "Failed to compile WASM module");
                                println!("ERROR: Failed to compile WASM module for {}: {}", runtime.label(), e);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("POOL INIT: Failed to read WASM binary for {} at {}: {}", runtime.label(), path.display(), e);
                    error!(
                        runtime = runtime.label(),
                        path = %path.display(),
                        error = %e,
                        "Failed to read WASM binary (expected if runtime is not yet implemented)"
                    );
                }
            }
        }

        Ok(pool)
    }

    pub fn get(&self, runtime: &Runtime) -> Option<&RuntimeHandle> {
        match runtime {
            Runtime::JavaScript => self.js_handle.as_ref(),
            Runtime::Python => None, // Python is a Command, no Handle
        }
    }

    pub async fn execute(
        &self,
        runtime: &Runtime,
        code: &str,
        fuel_limit: u64,
        timeout_ms: u64,
        memory_limit_mb: u32,
        ) -> Result<crate::ExecutionResult, crate::error::EngineError> {
        match runtime.style() {
            RuntimeStyle::Reactor => {
                let handle = match runtime {
                    Runtime::JavaScript => self.js_handle.as_ref(),
                    _ => None,
                }.ok_or(crate::error::EngineError::RuntimeUnavailable(runtime.language_id().to_string()))?;
                
                handle.execute(code, fuel_limit, timeout_ms, memory_limit_mb).await
            }
            RuntimeStyle::Command => {
                let module = self.python_module.as_ref()
                    .ok_or(crate::error::EngineError::RuntimeUnavailable("python".to_string()))?;
                
                execute_python(
                    module,
                    &self.python_linker,
                    &self.engine_standard,
                    code,
                    fuel_limit,
                    timeout_ms,
                ).await
            }
        }
    }
}
