import re
with open("engine/src/runtime.rs", "r") as f:
    content = f.read()

# Replace the evaluation and output reading logic
old_eval = """            // --- Evaluate JS ---
            let eval_js_func = instance
                .get_typed_func::<(u32, u32), i32>(&mut store, "eval_js")
                .map_err(|e| EngineError::EvalError(e.to_string()))?;

            let eval_start = Instant::now();

            // Synchronous call blocks the worker thread (allowed because of spawn_blocking)
            let eval_result = eval_js_func.call(&mut store, (ptr, len));"""

new_eval = """            // --- Evaluate Code ---
            let eval_start = Instant::now();
            let eval_result = match self.runtime {
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
            };"""

content = content.replace(old_eval, new_eval)

old_read = """            let stdout_bytes = stdout_pipe.contents();
            let stderr_bytes = stderr_pipe.contents();

            let stdout = String::from_utf8_lossy(&stdout_bytes).into_owned();
            let stderr = String::from_utf8_lossy(&stderr_bytes).into_owned();"""

new_read = """            let (stdout, stderr) = match self.runtime {
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
            };"""

content = content.replace(old_read, new_read)

old_ok = """            Ok(ExecutionResult {
                stdout,
                stderr,
                error,
                status_code: result_code,
                metrics,
            })"""

new_ok = """            let runtime_notes = match self.runtime {
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
            })"""

content = content.replace(old_ok, new_ok)

with open("engine/src/runtime.rs", "w") as f:
    f.write(content)
