# Apatheia

**Sub-millisecond, WASM-based code sandbox for autonomous AI agents.**

Apatheia executes untrusted JavaScript in a sandboxed QuickJS interpreter compiled to
WebAssembly (`wasm32-wasi`), providing memory-safe isolation by construction via WASM
linear memory. The sandbox relies on [Wasmtime](https://wasmtime.dev/)'s correctness
for its isolation guarantees.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                    AI Agent                          │
│              (sends JS to execute)                   │
└──────────────────────┬──────────────────────────────┘
                       │ POST /execute
                       ▼
┌─────────────────────────────────────────────────────┐
│                  API Server (axum)                    │
│         REST + WebSocket • JSON schema               │
└──────────────────────┬──────────────────────────────┘
                       │
          ┌────────────┼────────────┐
          ▼            ▼            ▼
   ┌────────────┐ ┌─────────┐ ┌──────────────┐
   │   Engine   │ │Telemetry│ │ FFI Bridge   │
   │            │ │         │ │              │
   │ wasmtime   │ │ tracing │ │ SSRF-firewalled
   │ InstancePre│ │ metrics │ │ fetch() host │
   │ fuel+watch │ │         │ │ function     │
   └─────┬──────┘ └─────────┘ └──────────────┘
         │
         ▼
   ┌────────────┐
   │ QuickJS    │
   │ .wasm      │
   │ (wasm32-   │
   │  wasi)     │
   └────────────┘
```

### Execution Model

1. **Startup**: Load `quickjs.wasm` into a Wasmtime `Engine` → compile to an
   `InstancePre` snapshot (one-time cost).
2. **Per-request**: Clone a fresh `Instance` from `InstancePre` via Wasmtime's
   pooling allocator / CoW semantics (microsecond-scale).
3. **Execute**: Write the JS string into WASM linear memory → call the exported
   `eval` function → QuickJS interprets the JS inside the sandbox.
4. **Collect**: Read stdout/stderr from linear memory.
5. **Teardown**: Drop the `Instance` entirely. No GC, no reuse, no state pollution.

### Safety Mechanisms

- **Fuel metering**: Wasmtime `Fuel` limits instruction count per execution.
- **Wall-clock watchdog**: `tokio::time::timeout` wraps every execution — fuel alone
  is NOT sufficient since instruction count doesn't bound wall-clock time evenly.
- **SSRF firewall**: Outbound `fetch()` calls are validated against a deny-list of
  private/loopback addresses before the HTTP request is made.
- **Memory limits**: WASM linear memory is bounded; response sizes from `fetch()` are
  capped.

## Project Structure

```
apatheia/
├── Cargo.toml           # Workspace root
├── engine/              # WASM engine: wasmtime, InstancePre, fuel/watchdog
├── api/                 # Axum REST + WebSocket server
├── ffi-bridge/          # SSRF-firewalled fetch() bridge (independently testable)
├── quickjs-wasm/        # Build scripts + vendored QuickJS for wasm32-wasi
├── telemetry/           # Shared tracing/metrics infrastructure
└── dashboard/           # React + Vite + TailwindCSS + Recharts frontend
```

## Current Status

### Phase 1: Scaffold ✅
- [x] Cargo workspace with 5 Rust crates
- [x] Dependency declarations (wasmtime, tokio, axum, serde, tracing, reqwest, etc.)
- [x] Stub implementations — `cargo build` succeeds
- [x] Git initialized
- [x] README with architecture documentation

### Phase 2: QuickJS WASM Build ✅
- [x] Vendor QuickJS sources
- [x] `build.rs` cross-compilation to wasm32-wasi via WASI SDK
- [x] Reproducible build producing `quickjs.wasm`

### Phase 3: Engine Core ✅
- [x] InstancePre snapshot loading at startup
- [x] Per-request Instance cloning with CoW
- [x] Memory marshaling (JS string → linear memory → eval → read output)
- [x] Fuel metering + wall-clock watchdog
- [x] Telemetry: separate cold-instantiation vs total latency measurements
- **Baseline numbers locked in**:
  - `instance_clone_time_us`: ~20-40µs warm / ~270-490µs cold-cache
  - `execution_time_us`: ~650-950µs for trivial scripts (dominated by QuickJS context init)
  - *(Note: A context-prewarming optimization is possible and parked on a branch for later if time allows)*
- **API Metrics Contract (Phase 4 Dashboard target)**:
  - `instance_clone_time_us`, `execution_time_us`, `memory_marshal_us`, `total_time_us`, `fuel_consumed`

### Phase 4: FFI Bridge (Pending)
- [ ] SSRF-firewalled fetch() host function
- [ ] Async-correct design (no `block_on` on tokio executor)
- [ ] Response size limits
- [ ] Unit tests for SSRF deny-list

### Phase 5: API + Dashboard (Pending)
- [ ] POST /execute endpoint with JSON schema
- [ ] WebSocket streaming
- [ ] React dashboard with Recharts metrics visualization
- [ ] Live latency display (from actual telemetry, never hardcoded)

## License

MIT
