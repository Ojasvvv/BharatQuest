# Apatheia: The Why-Not-That Log

This document interrogates every consequential engineering decision made in the Apatheia codebase.

### Decision: Interpreter-in-WASM vs JS-to-WASM Compiler
**What we did:** Compiled the QuickJS and MicroPython interpreters into WebAssembly, and fed the AI's dynamically generated string code to those interpreters at runtime.
**What we considered instead:** Using a JS-to-WASM or Python-to-WASM compiler to compile the AI-generated code directly into a brand new `.wasm` binary on every request.
**Why we rejected the alternative(s):** Compiling source code directly to WASM is extremely slow (taking seconds), which breaks the sub-millisecond execution constraint required for high-throughput AI agent pipelines. It also requires heavy compiler toolchains to be present on the host server. 
**What we'd reconsider if constraints changed:** If latency was not the primary constraint, and absolute execution performance of the script was paramount (e.g., heavy math computation), we would reconsider direct compilation.
**Risk this decision carries:** The interpreter itself incurs performance overhead, so AI-generated scripts run significantly slower than native machine code.

### Decision: QuickJS vs Other JS Engines
**What we did:** Chose QuickJS.
**What we considered instead:** V8, Boa, deno_core, Hermes, SpiderMonkey, JerryScript.
**Why we rejected the alternative(s):** 
- *V8 / deno_core*: Far too massive to compile into a lightweight WASM module. V8's JIT compiler does not map well to WASM linear memory and violates the sub-millisecond clone constraint.
- *Boa*: Written in Rust, which could theoretically compile to WASM easily, but its performance and standard library completeness were lower than QuickJS at the time of evaluation.
- *Hermes*: Geared entirely toward React Native.
- *SpiderMonkey*: Too heavy, similar to V8.
**What we'd reconsider if constraints changed:** If we wanted full Node.js API compatibility, we would be forced to look at something heavier like deno_core, sacrificing startup speed.
**Risk this decision carries:** QuickJS is not fully V8-compatible in terms of modern web APIs, so LLMs trained on Node.js code might hallucinate APIs that don't exist.

### Decision: Wasmtime vs Other WASM Runtimes
**What we did:** Used Wasmtime with its Pooling Allocator.
**What we considered instead:** Wasmer, WasmEdge, wasm3.
**Why we rejected the alternative(s):** Wasmtime is the reference implementation of WASI in Rust, and its Pooling Allocator specifically enables the Copy-on-Write sub-millisecond initialization that this project demands. Wasmer and WasmEdge are strong, but Wasmtime's Fuel API and `InstancePre` ergonomics were exactly aligned with our requirements.
**What we'd reconsider if constraints changed:** Nothing. Wasmtime is the industry standard for this use case.
**Risk this decision carries:** We are heavily tied to Bytecode Alliance's ecosystem and updates.

### Decision: Rust as Host Language
**What we did:** Wrote the API, Engine, and Bridge in Rust.
**What we considered instead:** Go, Node.js/TypeScript, Python, C++.
**Why we rejected the alternative(s):** Rust provides zero-cost abstractions, memory safety without garbage collection latency spikes, and first-class integration with the `wasmtime` crate (which is written in Rust). Using Node.js or Python would introduce GC pauses into our sub-millisecond latency measurements.
**What we'd reconsider if constraints changed:** Go would be a viable alternative for the web server layer due to its fantastic concurrency, but bridging to `wasmtime` (C API) is clunkier than Rust.
**Risk this decision carries:** Steeper learning curve for contributors.

### Decision: Axum vs Other Web Frameworks
**What we did:** Chose Axum.
**What we considered instead:** Actix-web, Warp, Rocket.
**Why we rejected the alternative(s):** Axum is built natively on top of `tokio` and `tower`. It handles WebSockets flawlessly (required for the dashboard), integrates perfectly with `tracing`, and has no macros obscuring control flow unlike Rocket.
**What we'd reconsider if constraints changed:** N/A. Axum is the modern standard.

### Decision: InstancePre + Clone-per-request vs Other Models
**What we did:** Used Wasmtime's `InstancePre` and cloned a fresh sandbox instance for every single request.
**What we considered instead:** Re-using a single long-lived WASM instance and wiping its state, or spawning a fresh OS process/container per request.
**Why we rejected the alternative(s):** Re-using a single WASM instance creates massive security risks (state leaking between AI agents) and requires building complex memory wiping logic. Spawning a fresh OS process or container takes hundreds of milliseconds to seconds. InstancePre + CoW allows the security of fresh instances with the speed of reuse.
**What we'd reconsider if constraints changed:** If Wasmtime didn't exist, we'd have to use a pool of pre-forked OS processes (microVMs).
**Risk this decision carries:** High initial memory footprint (256MB allocated per instance in the pool limits concurrency based on available RAM).

### Decision: Pooling Allocator
**What we did:** Enabled Wasmtime's Pooling Allocator for QuickJS.
**What we considered instead:** Standard on-demand memory allocation for instances.
**Why we rejected the alternative(s):** Standard allocation requires the OS to map new memory and copy data on every instantiation. The Pooling Allocator sets up `mmap` limits upfront and uses Copy-on-Write (CoW), which is the entire basis of the "sub-millisecond" claim. Without it, instantiation takes orders of magnitude longer.

### Decision: Fuel Metering AND Wall-clock Watchdog
**What we did:** Implemented both instruction-based fuel metering and a `tokio` wall-clock timeout.
**What we considered instead:** Using only one or the other.
**Why we rejected the alternative(s):** Fuel alone doesn't protect against the WASM guest calling a blocking host function (like sleeping or waiting for an HTTP response). Wall-clock timeout alone doesn't protect against deterministic infinite loops locking up CPU cores indefinitely. You need both to protect CPU and Time.

### Decision: SSRF Defense
**What we did:** **GAP:** It was originally planned to use resolve-then-validate, manual redirect handling. However, the codebase reveals this is completely unimplemented (descoped).
**What we considered instead:** An egress proxy or routing all `fetch` calls through a third-party service.
**Why we rejected the alternative(s):** Due to hackathon constraints, the entire network bridge feature was descoped and left as a stub in `ffi-bridge/src/lib.rs`.

### Decision: Async/Sync Bridge Solution
**What we did:** Used `tokio::task::spawn_blocking` to wrap the synchronous WASM execution.
**What we considered instead:** Implementing a microtask pumping queue inside WASM to yield control back to the Rust async executor.
**Why we rejected the alternative(s):** Implementing async/await across the FFI boundary into a synchronous C interpreter is incredibly difficult and requires rewriting the QuickJS event loop integration. `spawn_blocking` was chosen for speed of delivery (hackathon constraints).
**Risk this decision carries:** `spawn_blocking` consumes an entire OS thread from the tokio blocking pool for the duration of the request. High concurrency will exhaust the blocking pool thread limit and degrade performance.

### Decision: Dashboard Tech Stack
**What we did:** React, Vite, Tailwind.
**What we considered instead:** Server-rendered templates, Grafana.
**Why we rejected the alternative(s):** We needed dynamic WebSocket visualization (waterfalls) and an interactive code simulator. Server-rendered HTML is too slow for sub-millisecond metrics visualization. Grafana requires a separate heavy stack. 

### Decision: WebSocket Streaming
**What we did:** Pushed metrics over WebSockets.
**What we considered instead:** Polling or Server-Sent Events (SSE).
**Why we rejected the alternative(s):** WebSockets are bidirectional and have lower latency overhead per message than SSE or HTTP polling.

### Decision: MAX_ITERATIONS = 3
**What we did:** Limited the self-healing loop to 3 retries based on `parent_request_id`.
**What we considered instead:** Unlimited retries or higher numbers.
**Why we rejected the alternative(s):** If an LLM cannot fix a syntax or logic error within 3 attempts given direct traceback feedback, it has likely entered an unrecoverable hallucination loop. 

### Decision: JS & Python Scope
**What we did:** Supported only JavaScript and Python.
**What we considered instead:** Supporting Go, Rust, Java.
**Why we rejected the alternative(s):** Hackathon time constraints. Additionally, Python and JS are the dominant languages output by autonomous AI agents today.

### Decision: Cargo Workspace Structure
**What we did:** Split into `api`, `engine`, `ffi-bridge`, `quickjs-wasm`, `telemetry`.
**What we considered instead:** A monolithic crate.
**Why we rejected the alternative(s):** We needed strong boundaries to ensure the API layer didn't accidentally import unsafe C bindings, and to isolate unit tests cleanly (especially for the WASM compiler toolchain).
