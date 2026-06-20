# Apatheia

**Sub-millisecond, WASM-based code sandbox for autonomous AI agents.**

Apatheia executes untrusted JavaScript in a sandboxed QuickJS interpreter compiled to
WebAssembly (`wasm32-wasi`), providing memory-safe isolation by construction via WASM
linear memory. The sandbox relies on [Wasmtime](https://wasmtime.dev/)'s correctness
for its isolation guarantees.

## Runtime Support

| Runtime    | Interpreter          | Cold Start (p50) | Exec Time (p50) |
|------------|----------------------|------------------|-----------------|
| JavaScript | QuickJS/WASM         | 379µs | 2091µs |
| Python     | MicroPython/WASM     | 66µs | 1330µs |

*p50 over last 200 runs. Cold start = WASM instance setup time.
Exec time = interpreter evaluation time. Measured separately.*

## Language Scope

- **JavaScript**: ES2020 via QuickJS. `fetch()` available with SSRF firewall.
  Full ES2020 feature set. No `node:` modules.
- **Python**: MicroPython subset of CPython 3. No third-party packages
  (`requests`, `numpy`, `pandas` not available). Standard builtins only.
  Fuel metering and wall-clock watchdog applied identically to JavaScript.

## Architecture

One WASM host (Wasmtime + Rust). Two interpreter binaries:
- `quickjs.wasm` — Reactor module, InstancePre + CoW cloning
- `micropython-wasi.wasm` — Command module, fresh instantiation per request

Adding a language: compile its interpreter to `wasm32-wasi`, expose
`alloc_buffer` + `eval_code` + `read_output` exports (Reactor) or use
`_start` + WASI stdio (Command). No host changes required.

## Security

Memory-safe isolation via WASM linear memory, enforced by Wasmtime.
- Fuel metering: configurable opcode budget per execution
- Wall-clock timeout: `tokio::time::timeout` enforced independently  
- Memory cap: WASM linear memory limit per instance
- SSRF firewall: `fetch()` bridge blocks RFC1918 and metadata endpoints
- Self-healing cap: MAX_ITERATIONS=3 enforced server-side

*Relies on Wasmtime's correctness guarantees. Not formally verified.*
