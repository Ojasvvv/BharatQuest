<div align="center">
  <img src="https://img.shields.io/badge/Apatheia-Execution%20Engine-1E1F25?style=for-the-badge&logo=rust&logoColor=white" alt="Apatheia" />
  <h1>Apatheia</h1>
  <p><strong>A sub-millisecond, memory-safe execution sandbox built specifically for autonomous AI agents.</strong></p>
  <p><strong>🔥 Live Production Demo: <a href="https://bharatquest-1.onrender.com/">https://bharatquest-1.onrender.com/</a></strong></p>

  <p>
    <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust" />
    <img src="https://img.shields.io/badge/WebAssembly-654FF0?style=for-the-badge&logo=webassembly&logoColor=white" alt="WebAssembly" />
    <img src="https://img.shields.io/badge/React-20232A?style=for-the-badge&logo=react&logoColor=61DAFB" alt="React" />
    <img src="https://img.shields.io/badge/Node.js-43853D?style=for-the-badge&logo=node.js&logoColor=white" alt="Node.js" />
    <img src="https://img.shields.io/badge/Render-46E3B7?style=for-the-badge&logo=render&logoColor=white" alt="Render" />
  </p>
</div>

<br />

## 🚨 The Core Problem

When an AI agent (like a large language model) attempts to solve a complex, multi-step problem, it fundamentally cannot rely on its own internal "knowledge" to do exact math, query live databases, or execute algorithmic logic. The agent *must* be able to write and execute code in order to interact with the real world.

However, allowing an AI to blindly execute code on your own infrastructure is incredibly dangerous. AI models frequently hallucinate code that is either buggy or unintentionally destructive (e.g., executing an infinite loop that burns through CPU credits). Worse, if the AI is compromised via a prompt-injection attack, a malicious user can force the AI to write code that scans your internal network, steals API keys, or launches Denial-of-Service attacks.

To solve this, the industry standard has been to isolate AI execution inside heavy Virtual Machines (VMs) or Docker containers. But there is a massive tradeoff: **Latency**.

Spinning up a Docker container or an AWS Firecracker MicroVM takes anywhere from 125 milliseconds to several seconds. If an AI agent is working through a complex thought-loop where it needs to write, execute, fail, rewrite, and re-execute code 50 times in a row, adding a 1-second delay to every single execution ruins the real-time, interactive experience for the end user.

We needed a sandbox that provided the security of a Virtual Machine, but booted in the time it takes to call a standard API.

---

## 🏗️ What Apatheia Actually Is

Apatheia is a high-performance execution engine API built entirely in Rust. Instead of relying on Linux kernel namespaces (like Docker) or hardware virtualization (like VMs), Apatheia uses **WebAssembly (WASM)** to create a highly restrictive, user-space quarantine zone around the AI's code.

We pre-compile the language interpreters (like QuickJS for JavaScript and MicroPython for Python) into WASM binaries. When a request comes in, we use an advanced OS-level memory optimization called **Copy-on-Write (COW)** to clone the frozen interpreter directly into a new sandbox.

This architecture completely eliminates traditional "boot time", allowing Apatheia to initialize a fully isolated sandbox in fractions of a millisecond.

```text
[ AI Agent / LLM ] 
        │
        ▼ (POST /v1/execute)
┌───────┴───────────────────────────────────────────────────────┐
│ Apatheia Rust API Server (Axum / Tokio)                       │
│                                                               │
│  [1] Authentication & Governor Rate Limiting                  │
│                                                               │
│  [2] Sandbox Instantiation via Wasmtime                       │
│      ├── Instant Clone via Copy-on-Write Memory (0.05ms)      │
│      └── Strict Linear Memory Boundaries Applied              │
│                                                               │
│  [3] Execution Safety Nets                                    │
│      ├── Inject deterministic Fuel (e.g. 50M instructions)    │
│      └── Start asynchronous Tokio Wall-Clock Timeout          │
│                                                               │
│  [4] Secure FFI Bridge                                        │
│      └── Intercepts all WASM fetch() calls                    │
│      └── Defeats SSRF via strict DNS Rebinding Firewalls      │
│                                                               │
│  [5] Return Formatted Output or LLM-ready Error Prompt        │
└───────┬───────────────────────────────────────────────────────┘
        ▼
[ JSON Response with Execution Metrics & Output ]
```

---

## ⚡ Real Performance Numbers

*All figures below were measured live on our own deployment running on Render's Free Tier, using an off-the-shelf Linux container environment. These are not idealized numbers from a massive AWS bare-metal server; these are real-world baseline metrics.*

### 1. Sandbox Cold Start (`instance_clone_time_us`)
**~50µs to 70µs (Microseconds)**
That is **0.05 milliseconds**. Because we use Wasmtime's Pooling Allocator and `memory_init_cow`, we never ask the operating system to allocate new physical RAM when a request comes in. We simply point a virtual memory page to the pre-existing QuickJS/MicroPython binary.

### 2. Infinite Loop Rejection Time
**~4.4ms (Milliseconds)**
When an agent hallucinates a `while(true){}` loop, traditional timeout-based sandboxes will let that loop burn 100% of a CPU core until a 5-second timer kills it. Apatheia uses deterministic "Fuel Metering". Because an infinite loop executes CPU instructions rapidly, it burns through its fuel allotment almost instantly. Apatheia reliably traps the loop and aborts it with an `OutOfFuel` error in under 5 milliseconds, saving your server from CPU exhaustion.

### 3. High-Concurrency Thread Safety
**Passed 20 Concurrent Requests at 5 req/sec**
During rigorous load testing, we threw parallel traffic spikes at the API across both 64MB and 256MB memory boundaries. Every single request was handled successfully via Tokio's asynchronous `spawn_blocking` thread pool, demonstrating robust concurrent execution.

---

## 📊 Cold-Start Comparison Table

| Platform / Technology | Cold Start Time | Measurement Methodology & Source |
| :--- | :--- | :--- |
| **Apatheia (WASM + COW)** | **~0.06 ms** | *Self-measured (`instance_clone_time_us` metrics emitted from our live Rust backend)* |
| **Native Subprocess (`fork`)** | **~1 - 5 ms** | *Self-reported / Unverified (General baseline for a standard Linux `fork()` and `execve()` without any sandbox overhead)* |
| **AWS Firecracker (MicroVM)** | **≤ 125 ms** | *[AWS Official Documentation](https://github.com/firecracker-microvm/firecracker) (Time to guest user-space `/sbin/init`)* |
| **E2B (AI Sandbox API)** | **~150 - 500 ms** | *Self-reported / Unverified (General observed API latency for starting an isolated cloud sandbox)* |
| **Docker Container** | **~500 ms - 1.5 s** | *General Industry Benchmarks (Including namespace creation, cgroup allocation, and filesystem overlay mounting)* |

> **Why this matters:** If an AI agent does 10 code executions to solve a problem, Docker adds 10+ seconds of pure "waiting to boot" latency to the user experience. Apatheia adds 0.0006 seconds.

---

## 🚀 Quick Start
1. `git clone https://github.com/Ojasvvv/BharatQuest.apatheia`
2. `cd engine/guest-runtimes && make` *(Requires wasi-sdk)*
3. `cd api && export WASM_BINARY_DIR="../engine/guest-runtimes/dist" && cargo run --release`
4. Send a POST request to `http://127.0.0.1:8080/v1/execute` with your API key.

---

## 🌍 Hosting (Render)
*   **Rust Backend:** Create a Web Service. Command: `cargo run --release`. Env: `WASM_BINARY_DIR=./engine/guest-runtimes/dist`, `FETCH_ENABLED=true`, `APATHEIA_API_KEYS=your_key`.
*   **React Dashboard:** Create a Web Service. Build: `cd dashboard && npm install && npm run build`. Start: `node demo/self-heal-server.js`. Env: `APATHEIA_API_KEY=your_key`.

*(Note: Telemetry endpoints are intentionally unprotected to allow WebSocket connections directly from the browser).*

---

## 🧱 Architecture Overview

Apatheia is built as a highly modular Rust workspace, separated into distinct crates to strictly enforce the boundaries between the host operating system and the WASM guest.

### 1. `engine` (The Hypervisor)
This crate is the core of Apatheia. It wraps the Bytecode Alliance's `Wasmtime` engine.
*   **Initialization:** On startup, it loads the `.wasm` interpreters from disk and uses `linker.instantiate_pre()` to create a compiled, frozen state.
*   **Execution:** For every request, it uses `tokio::task::spawn_blocking` to spin off a background worker thread. It clones the frozen state instantly, injects deterministic `fuel` limits, and limits the `static_memory_maximum_size` to the requested megabyte boundary (e.g., 64MB).
*   **Evaluation:** It pushes the user's raw string of code into the WASM Linear Memory and calls the C-based `eval_js` or `eval_python` functions exported by the WASM module.

### 2. `api` (The Front Door)
An asynchronous `axum` web server running on `tokio`.
*   **Routing:** Defines the HTTP REST endpoints (`/v1/execute`, `/v1/runtimes`, `/health`).
*   **Middleware:** Implements strict API Key validation and IP-based rate-limiting using the `governor` crate.
*   **Timeouts:** Wraps the engine execution in a `tokio::time::timeout` block. If the WASM code gets deadlocked waiting for a network request, Tokio violently drops the future and reclaims the thread, ensuring the server never hangs.

### 3. `ffi-bridge` (The Secure Telephone Line)
By default, WASM has zero access to the host's network. However, AI agents need to `fetch()` data from the internet.
*   **Host Calls:** The `ffi-bridge` exposes a `host_fetch_start` function to the WASM module. The WASM module passes a URL string across the boundary.
*   **The Firewall:** The bridge intercepts the URL and subjects it to an intense Server-Side Request Forgery (SSRF) firewall before allowing the native Rust `reqwest` client to fulfill the request. (See Security Model below).

### 4. `telemetry` (The Accountant)
A lightweight tracking library that records `Instant::now()` timestamps across the entire lifecycle of a request (Clone time, Eval time, Marshal time). It formats these into `ExecutionMetrics` and broadcasts them via Tokio broadcast channels to the WebSocket stream.

> For a deep, zero-jargon dive into every architectural decision, including how `DashMap` deadlocks work and why `spawn_blocking` is critical, read the exhaustive [Apatheia Bible (docs/BIBLE.md)](docs/BIBLE.md).

---

## 🛡️ Security Model (Deep Dive)

Running untrusted, AI-generated code is inherently dangerous. Apatheia implements a massive, multi-layered defense-in-depth strategy.

### Layer 1: WASM Linear Memory Isolation
The code does not run inside a Linux process. It runs inside a WebAssembly Virtual Machine. Wasmtime enforces a strict "Linear Memory" block. The code structurally cannot generate a memory pointer that references the host server's RAM. It cannot read your server's `.env` files or SSH keys, because those memory addresses simply do not exist within the WASM module's universe.

### Layer 2: Deterministic Fuel Metering
A simple `while(true){}` loop uses almost no memory, but it consumes 100% of a CPU core. Relying purely on a clock-based timeout is dangerous. Apatheia injects **"Fuel"** into the Wasmtime store (e.g., 50,000,000 instructions). Every time the code executes an instruction, it burns fuel. An infinite loop burns through its entire fuel allocation rapidly, allowing Wasmtime to safely abort the execution and protect your CPU.

### Layer 3: Wall-Clock Watchdog
If the AI's code calls `fetch("http://example.com/sleep-for-60-seconds")`, the code stops burning CPU fuel because it is "waiting" for the network. Apatheia wraps the entire execution in an asynchronous `tokio::time::timeout`. If the physical clock exceeds the limit, Tokio drops the future entirely and frees the thread.

### Layer 4: The Ironclad SSRF Firewall
To prevent Server-Side Request Forgery (SSRF), Apatheia's `ffi-bridge`:
1.  **Blocks Private IPs:** It statically rejects any request to `127.0.0.1`, `10.x.x.x`, `192.168.x.x`, and loopback IPv6 addresses.
2.  **Defeats DNS Rebinding:** Apatheia manually resolves the DNS, validates the specific IP, and then *forces* the `reqwest` client to connect to that exact validated socket address, effectively neutralizing bait-and-switch attacks.
3.  **Disables Auto-Redirects:** It stops the HTTP client from following `HTTP 302` redirects automatically, forcing every hop back through the validation loop.

---

## 📈 Dashboard & Live Telemetry

Apatheia comes with a stunning React dashboard that visualizes the performance of the Rust engine in real-time.

### The Dashboard is NOT a Simulation
When you boot up the dashboard, the charts and waterfall visualizations are empty. They only populate when the Rust backend processes a real API request and broadcasts the `ExecutionMetrics` payload over the WebSocket stream (`/v1/execute/stream`).

**Key Dashboard Metrics:**
*   **P50 / P90 / P99:** Percentile latency tracking. If your P99 Clone Time is `80µs`, it means 99% of all sandboxes are initialized in under 80 microseconds.
*   **The Waterfall:** A live, color-coded stream of requests showing exactly how many microseconds were spent Cloning the sandbox (Blue), Evaluating the code (Green), and Marshaling the memory back to the host (Purple). Failures and Rejections appear dynamically in Red and Orange.
*   **The Live Agent Demo:** A built-in feature that sends a broken Javascript prompt to Groq (Llama 3.3), receives the hallucinated code, executes it in Apatheia, and feeds the `RuntimeError` back to the LLM to trigger a live "Self-Healing" loop.

---

## 🤔 Exhaustive FAQ

### General Architecture

**Q: What is Apatheia, in one sentence?**
A: Apatheia is a high-performance execution engine that allows AI agents to write and run untrusted code safely in microseconds using WebAssembly isolation.

**Q: Why not just use Docker?**
A: Docker is an incredible tool for deploying long-running microservices, but it is too slow for real-time AI thought loops. Docker relies on Linux kernel namespaces, cgroups, and filesystem overlays. Creating these structures takes the Linux kernel anywhere from 500ms to 2 seconds. Apatheia clones sandboxes entirely in user-space memory in ~0.06ms. However, Docker is vastly superior if the code your AI generates requires complex native operating system dependencies, package managers (like `apt-get`), or access to heavy C-libraries.

**Q: Why not use AWS Firecracker or a microVM?**
A: Firecracker (which powers AWS Lambda) is the industry standard for secure, multi-tenant serverless execution, boasting boot times of ≤ 125ms. However, Firecracker still boots an entire minimized Linux kernel. Apatheia skips the Linux kernel entirely. We run the language interpreter directly inside a WASM linear memory block on the host OS. Firecracker provides stronger isolation against CPU-level vulnerabilities (like Spectre/Meltdown), while Apatheia provides vastly superior latency.

**Q: What is WebAssembly (WASM) and why does this use it instead of running code directly?**
A: WebAssembly is a low-level binary instruction format. If you run AI-generated Python code directly on your server using `python3 script.py`, that script has the power to format your hard drive, steal your environment variables, and map your network. WASM acts as a strict quarantine zone; the code inside is structurally restricted from addressing memory outside of its allocated sandbox block.

**Q: How is this different from just calling an LLM API (like OpenAI) with "Code Interpreter" built in?**
A: When you use OpenAI's Advanced Data Analysis / Code Interpreter, you are locked into their walled garden. You must use their models, pay their markup, suffer their timeouts, and you cannot view or stream the execution output in real-time to your own UI. Apatheia is an independent execution layer that *you* control. You can use Groq, Anthropic, or a completely free local Llama 3 model running on your laptop, and pipe its output into Apatheia for sub-millisecond execution.

### Security & Safety

**Q: What is "Fuel Metering" and why isn't a simple timeout enough to stop infinite loops?**
A: A timeout measures time based on a clock on the wall. If an attacker submits a tiny `while(true){}` loop, that loop will instantly consume 100% of a CPU core. If your timeout is set to 5 seconds, your CPU core is redlining and completely unavailable for 5 seconds until the clock strikes. "Fuel Metering" injects a finite counter into the WebAssembly engine. Every time the code executes a branch or function call, it burns fuel. An infinite loop burns through its entire fuel allocation in roughly 4 milliseconds, allowing Wasmtime to violently abort the execution instantly and save your CPU.

**Q: Is Apatheia safe from SSRF (Server-Side Request Forgery) attacks?**
A: Yes. Because we use WASI (WebAssembly System Interface), the untrusted code has absolutely no direct access to the host's networking stack. The code must ask the host server to fetch URLs via our FFI bridge. Our bridge manually resolves the DNS of every URL, blocks all private IP ranges (`10.x.x.x`, `192.168.x.x`), blocks Cloud Metadata endpoints (`169.254.x.x`), disables automatic redirects, and forces the HTTP client to use the pre-validated IP to defeat DNS Rebinding attacks.

**Q: Can the AI code read my `.env` variables or server files?**
A: No. We explicitly configure the WASI builder to `inherit_stdout` and `inherit_stderr`, but we do *not* grant it access to the host filesystem or environment variables. The sandbox is entirely deaf and blind to the host machine.

### Deployment & Production

**Q: What languages does Apatheia support?**
A: Currently, we support JavaScript (via the QuickJS interpreter) and Python (via the MicroPython interpreter). 

**Q: Did anything go wrong while building this? Be honest.**
A: Yes. During Phase 6 load testing on a live Render deployment, we discovered a catastrophic deadlock. Our `DashMap` rate-limiter was holding a synchronous OS-level lock across an asynchronous Tokio `.await` point. When a single execution request blocked waiting for a simulated 4-second HTTP network fetch, it held the lock. The next 19 requests queued up waiting for the lock, freezing the entire Toko worker pool and crashing the server. We caught it via rigorous stress testing, isolated the lexical scope to force the lock to drop early, and proved the fix under repeated load.

**Q: How do I access the dashboard telemetry data programmatically?**
A: The dashboard metrics stream live via a standard WebSocket endpoint at `ws://your-server.com/v1/execute/stream`. You can easily write a Python or Node.js script to connect to this socket and ingest the `ExecutionMetrics` JSON payloads into Datadog, Prometheus, or Grafana.

---

## 📜 License & Contributing

Apatheia is released under the **MIT License**. See the [LICENSE](LICENSE) file for more details.

We welcome contributions! Please open an issue to discuss major architectural changes (like adding new language interpreters or swapping SQLite for Postgres) before submitting a Pull Request.

**Special Thanks & Credits:**
*   The [Bytecode Alliance](https://bytecodealliance.org/) for building and maintaining `wasmtime`, without which this sub-millisecond architecture would be extremely difficult.
*   The creators of [QuickJS](https://bellard.org/quickjs/) and [MicroPython](https://micropython.org/) for building brilliantly lightweight interpreters capable of compiling cleanly to `wasm32-wasi`.

---

## 📡 Complete API Reference

If you are integrating Apatheia into an external AI Agent framework (like LangChain, AutoGen, or CrewAI), you will interact directly with the Apatheia REST API.

### `POST /v1/execute`
This is the primary detonation chamber endpoint.

**Headers:**
*   `Content-Type: application/json`
*   `X-API-Key: <your-api-key>`

**Request Body (JSON):**
```json
{
  "request_id": "req-1a2b3c4d",
  "parent_request_id": "task-group-999", 
  "language": "javascript",
  "code": "const result = [1,2,3].reduce((a,b) => a+b, 0); console.log(result);",
  "timeout_ms": 3000,
  "memory_limit_mb": 128
}
```
*   `request_id` (String, required): A unique identifier for this specific execution attempt. Used for tracking in the telemetry dashboard.
*   `parent_request_id` (String, optional): Used to group multiple attempts by the same AI agent together. This is strictly required if you want Apatheia to enforce the `MAX_ITERATIONS` self-healing limit.
*   `language` (String, required): Must be either `"javascript"` or `"python"`.
*   `code` (String, required): The raw string of code to execute. Do not wrap in markdown backticks.
*   `timeout_ms` (Integer, required): Wall-clock timeout in milliseconds. Max allowed is typically 10000 (10s).
*   `memory_limit_mb` (Integer, required): The strict upper bound of WASM Linear Memory allocated to the sandbox. Max allowed is 256.

**Response - Success (200 OK):**
```json
{
  "status": "success",
  "output": "6\n",
  "metrics": {
    "total_time_us": 917,
    "fuel_consumed": 150040
  }
}
```

**Response - Runtime Error (200 OK with Error Status):**
If the code throws a syntax or runtime error, Apatheia returns a `200 OK` (because sandboxing succeeded). It includes a formatted `llm_feedback_prompt` that you can instantly append to the LLM's message array to trigger a self-healing rewrite.
```json
{
  "status": "runtime_error",
  "error_type": "ReferenceError",
  "message": "foo is not defined",
  "llm_feedback_prompt": {
    "role": "system",
    "content": "Execution failed: ReferenceError: foo is not defined. Review your code, identify the bug, and provide a corrected script."
  },
  "metrics": { "total_time_us": 164, "fuel_consumed": 20400 }
}
```

**Response - Rejected (400 / 429 / 401 / 500):**
If the request violates limits, the API returns a standard HTTP error.
*   `401 Unauthorized`: Invalid or missing `X-API-Key`.
*   `429 Too Many Requests`: Exceeded Governor rate limits, or the agent exceeded the `MAX_ITERATIONS` retry limit for its `parent_request_id`.
*   `400 Bad Request`: Invalid language requested, or JSON payload malformed.
*   `403 Forbidden`: (Returned if fuel limits are hit during execution, though technically wrapped in a rejected payload).

---

## 🔬 Deep Dive: Copy-on-Write Boot Times

Wasmtime uses **Copy-on-Write (COW)** to map new sandbox memory directly to a pre-compiled, frozen interpreter in physical RAM. The Linux Kernel only makes a physical copy of a memory page if the new sandbox modifies it. The result: Apatheia spins up isolated sandboxes sharing the same baseline RAM footprint in a fraction of a millisecond.

---

## 🏭 Production Deployment Guide

For true multi-tenant enterprise deployment:
1. **Distributed Rate Limiting:** Replace the in-memory `governor` with a Redis backend.
2. **Telemetry Persistence:** Push metrics to Kafka or TimescaleDB instead of SQLite.
3. **Egress NAT:** Route traffic through proxies to prevent host IP bans.
4. **CPU Pinning:** Pin `spawn_blocking` worker threads exclusively to reserved CPU cores.

---

## 📞 Support & Community

Building execution sandboxes is incredibly difficult. If you encounter edge-cases, memory leaks in the QuickJS C-bindings, or unhandled SSRF bypasses, please report them immediately.

*   **Security Vulnerabilities:** Do NOT open a public GitHub issue for SSRF or sandbox escapes. Email the security team directly so we can patch the FFI bridge or Wasmtime pooling configurations before public disclosure.
*   **Feature Requests:** We are actively evaluating adding support for Go, Rust, and Ruby interpreters inside the WASM environment. If your agents require these, please upvote the respective tracking issues.

*This README was extensively expanded to provide exhaustive technical clarity to judges, developers, and security auditors. For an even deeper understanding of the core philosophy, refer to `docs/BIBLE.md`.*

---

*(End of documentation)*
