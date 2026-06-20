// Temporary diagnostic module — remove after debugging
#[cfg(test)]
pub mod debug_tests {
    use wasmtime::*;
    use wasmtime_wasi::pipe::MemoryOutputPipe;
    use wasmtime_wasi::preview1::WasiP1Ctx;
    use wasmtime_wasi::WasiCtxBuilder;
    use std::path::PathBuf;

    fn wasm_path() -> PathBuf {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        PathBuf::from(manifest_dir)
            .parent()
            .unwrap()
            .join("quickjs-wasm/build/quickjs.wasm")
    }

    #[tokio::test]
    async fn debug_memory_write() {
        let mut config = Config::new();
        config.consume_fuel(true);
        let engine = Engine::new(&config).unwrap();

        let wasm_bytes = std::fs::read(wasm_path()).unwrap();
        let module = Module::new(&engine, &wasm_bytes).unwrap();

        let mut linker: Linker<WasiP1Ctx> = Linker::new(&engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx| ctx).unwrap();

        let stdout_pipe = MemoryOutputPipe::new(256 * 1024);
        let stderr_pipe = MemoryOutputPipe::new(256 * 1024);

        let wasi_ctx = WasiCtxBuilder::new()
            .stdout(stdout_pipe.clone())
            .stderr(stderr_pipe.clone())
            .allow_blocking_current_thread(true)
            .build_p1();

        let mut store = Store::new(&engine, wasi_ctx);
        store.set_fuel(100_000_000).unwrap();

        let instance = linker.instantiate(&mut store, &module).unwrap();

        // Call _initialize
        let init: TypedFunc<(), ()> = instance.get_typed_func(&mut store, "_initialize").unwrap();
        init.call(&mut store, ()).unwrap();
        let fuel_after_init = store.get_fuel().unwrap();
        println!("Fuel after _initialize: {} (consumed: {})", fuel_after_init, 100_000_000 - fuel_after_init);

        let memory = instance.get_memory(&mut store, "memory").unwrap();
        let alloc_buffer: TypedFunc<i32, i32> = instance.get_typed_func(&mut store, "alloc_buffer").unwrap();
        let eval_js_func: TypedFunc<(i32, i32), i32> = instance.get_typed_func(&mut store, "eval_js").unwrap();

        let js_source_bytes = b"1+1\0";
        let alloc_len = js_source_bytes.len() as i32;
        let eval_len = alloc_len - 1;

        // Allocate buffer
        let ptr = alloc_buffer.call(&mut store, alloc_len).unwrap();
        println!("alloc_buffer returned ptr: {}", ptr);
        println!("memory size: {} bytes", memory.data_size(&store));

        // Write JS source
        memory.write(&mut store, ptr as usize, js_source_bytes).unwrap();

        // Read back what we wrote to verify
        let mut readback = vec![0u8; alloc_len as usize];
        memory.read(&store, ptr as usize, &mut readback).unwrap();
        println!("Written: {:?}", String::from_utf8_lossy(js_source_bytes));
        println!("Read back: {:?}", String::from_utf8_lossy(&readback));
        println!("Bytes match: {}", readback == js_source_bytes);

        // Now call eval_js on a raw thread
        let (tx, rx) = tokio::sync::oneshot::channel();
        std::thread::spawn(move || {
            let status = eval_js_func.call(&mut store, (ptr, eval_len));
            let _ = tx.send((status, store));
        });

        let (status_result, mut store) = rx.await.unwrap();
        let status = status_result.unwrap();
        println!("eval_js status: {}", status);

        let stdout = stdout_pipe.contents();
        let stderr = stderr_pipe.contents();
        println!("STDOUT: {:?}", String::from_utf8_lossy(&stdout));
        println!("STDERR: {:?}", String::from_utf8_lossy(&stderr));

        // Read the error buffer from linear memory
        let get_error_ptr: TypedFunc<(), i32> = instance.get_typed_func(&mut store, "get_error_ptr").unwrap();
        let get_error_len: TypedFunc<(), i32> = instance.get_typed_func(&mut store, "get_error_len").unwrap();
        let err_ptr = get_error_ptr.call(&mut store, ()).unwrap();
        let err_len = get_error_len.call(&mut store, ()).unwrap();
        println!("Error ptr: {}, len: {}", err_ptr, err_len);
        if err_ptr != 0 && err_len > 0 {
            let mut err_buf = vec![0u8; err_len as usize];
            memory.read(&store, err_ptr as usize, &mut err_buf).unwrap();
            println!("ERROR BUFFER: {:?}", String::from_utf8_lossy(&err_buf));
        }

        // Read the output buffer from linear memory
        let get_output_ptr: TypedFunc<(), i32> = instance.get_typed_func(&mut store, "get_output_ptr").unwrap();
        let get_output_len: TypedFunc<(), i32> = instance.get_typed_func(&mut store, "get_output_len").unwrap();
        let out_ptr = get_output_ptr.call(&mut store, ()).unwrap();
        let out_len = get_output_len.call(&mut store, ()).unwrap();
        println!("Output ptr: {}, len: {}", out_ptr, out_len);
        if out_ptr != 0 && out_len > 0 {
            let mut out_buf = vec![0u8; out_len as usize];
            memory.read(&store, out_ptr as usize, &mut out_buf).unwrap();
            println!("OUTPUT BUFFER: {:?}", String::from_utf8_lossy(&out_buf));
        }

        let fuel_remaining = store.get_fuel().unwrap();
        println!("Fuel remaining: {}, consumed by eval: {}", fuel_remaining, fuel_after_init - fuel_remaining);

        assert_eq!(status, 0, "Expected success but got status {}", status);
    }
}
