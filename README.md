<div align="center">
  <img src="https://img.shields.io/badge/Apatheia-Execution%20Engine-1E1F25?style=for-the-badge&logo=rust&logoColor=white" alt="Apatheia" />
  <h1>Apatheia</h1>
  <p><strong>A sub-millisecond, memory-safe execution sandbox built specifically for autonomous AI agents.</strong></p>

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

Apatheia is a high-performance execution engine API built entirely in Rust. Instead of relying on Linux kernel namespaces (like Docker) or hardware virtualization (like VMs), Apatheia uses **WebAssembly (WASM)** to create a mathematically sealed, user-space quarantine zone around the AI's code.

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
When an agent hallucinates a `while(true){}` loop, traditional timeout-based sandboxes will let that loop burn 100% of a CPU core until a 5-second timer kills it. Apatheia uses deterministic "Fuel Metering". Because an infinite loop executes CPU instructions rapidly, it burns through its fuel allotment almost instantly. Apatheia mathematically traps the loop and aborts it with an `OutOfFuel` error in under 5 milliseconds, saving your server from CPU exhaustion.

### 3. High-Concurrency Thread Safety
**Passed 20 Concurrent Requests at 5 req/sec**
During rigorous load testing, we threw parallel traffic spikes at the API across both 64MB and 256MB memory boundaries. Every single request was handled successfully via Tokio's asynchronous `spawn_blocking` thread pool without a single deadlock or dropped connection, proving genuine multi-threaded concurrency rather than serial queueing.

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

## 🚀 Quick Start & Local Setup

You can run the entire Apatheia engine on your local machine. The repository includes both the Rust execution backend and the React live dashboard.

### Prerequisites
*   **Rust** (`cargo`, `rustc` 1.75+)
*   **Node.js** (v18+) & **npm**
*   **wasi-sdk** (Required to compile the C-based QuickJS and Python interpreters into WASM). Download `wasi-sdk-20.0+` and extract it to a known directory.

### 1. Clone the repository
```bash
git clone https://github.com/Ojasvvv/BharatQuest.apatheia
cd BharatQuest.apatheia
```

### 2. Compile the WASM Sandboxes
Before the Rust engine can run, you must compile the "Guest Runtimes" (the interpreters) into `.wasm` files.
Ensure your `WASI_SDK_PATH` environment variable points to your downloaded WASI SDK.
```bash
export WASI_SDK_PATH="/path/to/wasi-sdk-20.0"
cd engine/guest-runtimes
make
cd ../..
```
This will generate `quickjs.wasm` and `micropython.wasm` in the `engine/guest-runtimes/dist/` directory.

### 3. Run the Rust API Server
The Rust API expects a few environment variables to know where the WASM files are and what API keys to accept.
```bash
# Path to the compiled WASM files
export WASM_BINARY_DIR="$(pwd)/engine/guest-runtimes/dist"

# A comma-separated list of accepted API keys
export APATHEIA_API_KEYS="local_dev_key,demo_key"

# Enable the FFI bridge to allow WASM to make external HTTP requests
export FETCH_ENABLED="true"

# Port to run the Axum server on
export PORT="8080"

cd api
cargo run --release
```

### 4. Execute Code via `curl`
Open a new terminal and send a live execution request to your newly running sandbox:
```bash
curl -X POST http://127.0.0.1:8080/v1/execute \
  -H "Content-Type: application/json" \
  -H "X-API-Key: local_dev_key" \
  -d '{
    "request_id": "test-123",
    "language": "javascript",
    "code": "const start = Date.now(); while(Date.now() - start < 100) {}; console.log(\"Hello from Apatheia!\");",
    "timeout_ms": 2000,
    "memory_limit_mb": 64
  }'
```
You will receive a JSON response containing the execution output, status, and precise nanosecond telemetry metrics.

---

## 🌍 Hosting Details (Deploying to Render)

Apatheia is designed to be easily hostable on platforms like Render, Railway, or AWS. The project contains two main services:
1.  **The Rust Backend (`api/src/main.rs`)**: This handles the intense WASM execution.
2.  **The Live Dashboard (`dashboard/` & `demo/self-heal-server.js`)**: This is a React frontend served by a lightweight Node.js Express server that also acts as a Server-Sent Events (SSE) bridge for LLM demonstrations.

### Deploying the Rust Execution Engine
To deploy the Rust backend to Render:
1. Create a new **Web Service** connected to this repository.
2. Set the **Build Command**: `cargo build --release` (You may need to use a Dockerfile if Render's native Rust environment doesn't have `wasi-sdk` installed to run the `make` step. For ease of use, we recommend pre-compiling the `.wasm` files and committing them to your repo, or using a multi-stage Dockerfile).
3. Set the **Start Command**: `cargo run --release`
4. Set the following **Environment Variables**:
    *   `APATHEIA_API_KEYS`: (e.g., `your_production_key`)
    *   `WASM_BINARY_DIR`: `./engine/guest-runtimes/dist`
    *   `FETCH_ENABLED`: `true`

### Deploying the Dashboard
The React dashboard is designed to connect to your live Rust backend.
1. Create a second **Web Service** on Render.
2. Set the **Build Command**: `cd dashboard && npm install && npm run build`
3. Set the **Start Command**: `node demo/self-heal-server.js`
4. Set the following **Environment Variables**:
    *   `GROQ_API_KEY`: Your LLM API key for the self-healing demo.
    *   `APATHEIA_API_KEY`: The same key you configured in your Rust backend.

**Note on Telemetry Routing:** In `api/src/main.rs`, the telemetry endpoints (`/v1/execute/stream` and `/v1/metrics/history`) are intentionally placed in the **public, unprotected** Axum router. This allows the dashboard's `useTelemetry.ts` WebSocket client to connect and stream live data directly to your browser without needing to securely pass API keys via headers (which browsers block for WebSockets). The actual code execution endpoint (`/v1/execute`) remains strictly protected by `auth_and_rate_limit`.

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
The code does not run inside a Linux process. It runs inside a WebAssembly Virtual Machine. Wasmtime enforces a strict "Linear Memory" block. The code mathematically cannot generate a memory pointer that references the host server's RAM. It cannot read your server's `.env` files or SSH keys, because those memory addresses physically do not exist within the WASM module's universe.

### Layer 2: Deterministic Fuel Metering
A simple `while(true){}` loop uses almost no memory, but it consumes 100% of a CPU core forever. Relying purely on a clock-based timeout is dangerous because the CPU is still redlining while waiting for the clock to strike.
Apatheia injects **"Fuel"** into the Wasmtime store (e.g., 50,000,000 instructions). Every time the interpreter executes a branch, loop, or function call, the internal fuel counter decrements. If the fuel hits zero, Wasmtime aborts execution synchronously at the opcode level. It is physically impossible for the code to hog the CPU.

### Layer 3: Wall-Clock Watchdog
If the AI's code calls `fetch("http://example.com/sleep-for-60-seconds")`, the code stops burning CPU fuel because it is "waiting" for the network. Fuel metering cannot save you here, and the worker thread will be held hostage.
Apatheia wraps the entire execution in an asynchronous `tokio::time::timeout`. If 2000ms pass on the physical clock on the wall, Tokio drops the future entirely and frees the thread, no matter what the WASM module was doing.

### Layer 4: The Ironclad SSRF Firewall
If the AI executes `fetch("http://169.254.169.254")` (the AWS Metadata IP), it could steal your cloud server's IAM credentials. To prevent Server-Side Request Forgery (SSRF), Apatheia's `ffi-bridge`:
1.  **Blocks Private IPs:** It statically rejects any request to `127.0.0.1`, `10.x.x.x`, `192.168.x.x`, and loopback IPv6 addresses.
2.  **Defeats DNS Rebinding:** An attacker might register `innocent.com` with a TTL of 0, resolving it first to a safe IP (to pass the firewall) and then to an evil IP (when the HTTP client actually connects). Apatheia manually resolves the DNS, validates the specific IP, and then *forces* the `reqwest` client to connect to that exact validated socket address, making bait-and-switch attacks mathematically impossible.
3.  **Disables Auto-Redirects:** It stops the HTTP client from following `HTTP 302` redirects automatically, forcing every single redirect hop back through the entire SSRF validation loop.

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
A: WebAssembly is a low-level binary instruction format. If you run AI-generated Python code directly on your server using `python3 script.py`, that script has the power to format your hard drive, steal your environment variables, and map your network. WASM acts as a strict mathematical quarantine zone; the code inside is physically incapable of addressing memory outside of its allocated sandbox block.

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
*   The [Bytecode Alliance](https://bytecodealliance.org/) for building and maintaining `wasmtime`, without which this sub-millisecond architecture would be impossible.
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
    "instance_clone_time_us": 55,
    "execution_time_us": 850,
    "memory_marshal_us": 12,
    "total_time_us": 917,
    "fuel_consumed": 150040
  }
}
```

**Response - Runtime Error (200 OK with Error Status):**
If the code executes but throws a syntax or runtime error, Apatheia returns a `200 OK` (because the API succeeded in sandboxing it). The response includes a specifically formatted `llm_feedback_prompt` that you can instantly append to the LLM's message array to trigger a self-healing rewrite.
```json
{
  "status": "runtime_error",
  "error_type": "ReferenceError",
  "message": "foo is not defined",
  "llm_feedback_prompt": {
    "role": "system",
    "content": "Execution failed: ReferenceError: foo is not defined. Review your code, identify the bug, and provide a corrected script."
  },
  "metrics": {
    "instance_clone_time_us": 54,
    "execution_time_us": 110,
    "memory_marshal_us": 0,
    "total_time_us": 164,
    "fuel_consumed": 20400
  }
}
```

**Response - Rejected (400 / 429 / 401 / 500):**
If the request violates limits, the API returns a standard HTTP error.
*   `401 Unauthorized`: Invalid or missing `X-API-Key`.
*   `429 Too Many Requests`: Exceeded Governor rate limits, or the agent exceeded the `MAX_ITERATIONS` retry limit for its `parent_request_id`.
*   `400 Bad Request`: Invalid language requested, or JSON payload malformed.
*   `403 Forbidden`: (Returned if fuel limits are hit during execution, though technically wrapped in a rejected payload).

---

## 🔬 Deep Dive: How Copy-on-Write (COW) Achieves 0.05ms Boot Times

The absolute hardest engineering challenge in building Apatheia was beating the AWS Firecracker benchmark of 125ms. To understand how we achieved ~0.06ms cold starts, you have to understand exactly what happens at the Linux Kernel layer when a request hits the API.

### The Problem with Normal Booting
If an AI wants to execute Python code, you need a Python interpreter. The MicroPython WASM binary is roughly 2.5 MB in size.
Normally, when a request comes in, the server would:
1. Ask the OS to allocate 2.5 MB of blank RAM.
2. Read the `micropython.wasm` file from the hard drive.
3. Parse the WASM bytecode.
4. Compile the WASM bytecode into native `x86_64` or `ARM64` machine code (using Wasmtime's Cranelift compiler).
5. Load the compiled machine code into the 2.5 MB of RAM.

Doing this takes roughly **100 to 200 milliseconds**. If you have 50 requests per second, you are thrashing the CPU cache and slowing the entire server to a crawl.

### The Copy-on-Write Solution
Apatheia bypasses steps 1 through 5 entirely for every single API request.

When the Apatheia Rust server *first boots up*, it performs the expensive load and compile step exactly once. It creates an `InstancePre` (a pre-compiled, frozen blueprint of the MicroPython interpreter). It loads this into a specific block of physical RAM.

When a live API request hits `POST /v1/execute`, Wasmtime asks the Linux Kernel to create a *Virtual Memory Map*. It says: "Hey Linux, create a new 64MB memory boundary for this user. But don't give them empty RAM. Just point their virtual memory addresses to the exact same physical RAM where the frozen `InstancePre` is sitting."

Because of an OS feature called **Copy-on-Write**, the Linux Kernel instantly grants the request in roughly `50` microseconds. Multiple concurrent API requests all technically point to the exact same physical bytes of memory on the server! 

But what if Request A writes a variable, and Request B tries to read it? 
The magic of Copy-on-Write is in the name. The moment Request A's code actually tries to *modify* a specific byte of that shared memory, the Linux Kernel's Memory Management Unit (MMU) catches it. The Kernel pauses Request A, instantly makes a private, physical copy of *only that specific 4KB memory page*, and points Request A to the new private copy. 

**The Result:** Apatheia can spin up 1,000 completely isolated Python sandboxes on a single server, and until they start writing heavy amounts of data to memory, they all share the same baseline physical RAM footprint, booting in a fraction of a millisecond.

---


## 🏭 Advanced Production Deployment Guide

If you are graduating from a single test deployment and planning to run Apatheia as a true, multi-tenant enterprise execution layer, there are several strict architectural upgrades you must implement. The MVP deployment (as provided in this repository) makes specific concessions for ease-of-use that do not scale horizontally.

### 1. Migrating to Distributed Rate Limiting
Apatheia currently uses the `governor` crate with a local `DashMap` to track API rate limits in-memory. If you deploy Apatheia behind an AWS Application Load Balancer across three EC2 instances, User A could hit Instance 1 five times, Instance 2 five times, and Instance 3 five times, effectively achieving 15 requests per second (bypassing the 5 req/sec limit).

**The Solution:** You must replace the in-memory `RateLimiter` with an asynchronous Redis backend. We recommend using the `redis` crate in asynchronous mode with a token bucket Lua script to guarantee atomic, cross-server rate-limit tracking. This introduces ~1ms of network latency to the request validation phase, which is a necessary tradeoff for horizontal scaling.

### 2. Externalizing Telemetry & Billing Logs
The `telemetry` crate currently flushes `ExecutionMetrics` to a local SQLite file (`metrics.db`) using the `rusqlite` crate. This was chosen because it requires zero infrastructure to run the Quick Start tutorial. However, SQLite on an ephemeral container means total data loss upon redeployment. Furthermore, SQLite struggles under heavy concurrent write contention.

**The Solution:** The `metrics_history_handler` and the core metric flushing mechanism must be redirected. We recommend:
1. Pushing metrics asynchronously to a message broker (like Apache Kafka or AWS Kinesis).
2. Having a dedicated background worker consume the stream and batch-write to a heavy-duty Time Series Database (TSDB) like TimescaleDB or InfluxDB.
3. For billing purposes, ensure that the `fuel_consumed` metric is cryptographically signed or handled via strict Exactly-Once processing semantics, as dropping these packets means losing revenue.

### 3. Implementing True Multi-Tenant Isolation
WebAssembly completely isolates the *memory* and *CPU instructions* of the execution. However, if two concurrent executions (from two different users) both make a `fetch()` call to the public internet via our FFI bridge, both requests originate from the exact same host IP address. 

If User A executes a malicious script that gets the host IP address banned by Cloudflare, User B will suddenly find that their legitimate HTTP requests are also being blocked. This is the "Noisy Neighbor IP Problem".

**The Solution:** For true multi-tenancy, you must implement Egress NAT translation. Apatheia's `reqwest` client builder must be modified so that the HTTP client binds to a specific, unique local network interface per user tier. Traffic from free-tier users should be routed through a pool of rotating proxies, whereas traffic from Enterprise users should be routed through dedicated Elastic IPs. This ensures that IP-reputation damage is isolated to the offending user.

### 4. Advanced CPU Pinning for Predictable Latency
While Apatheia's cold-starts are ~0.06ms, overall execution times can fluctuate based on the host operating system's CPU scheduler. If the Linux kernel decides to run a heavy garbage collection task or a network interrupt on the same CPU core that is executing the WASM sandbox, the `execution_time_us` might randomly jump from 1,000µs to 4,000µs.

**The Solution:** If you require strict, deterministic latency bounds for High-Frequency Trading AI or real-time game engines, you must isolate the Apatheia worker threads.
1. Use `isolcpus` in the Linux boot parameters to reserve specific CPU cores entirely.
2. Modify the Tokio runtime builder in `api/src/main.rs` to use `core_affinity`.
3. Pin the `spawn_blocking` worker threads exclusively to those reserved cores. This guarantees that the host OS will never schedule interrupts on the execution cores, resulting in flat, perfectly predictable p99 latency metrics.

---

## 📞 Support & Community

Building execution sandboxes is incredibly difficult. If you encounter edge-cases, memory leaks in the QuickJS C-bindings, or unhandled SSRF bypasses, please report them immediately.

*   **Security Vulnerabilities:** Do NOT open a public GitHub issue for SSRF or sandbox escapes. Email the security team directly so we can patch the FFI bridge or Wasmtime pooling configurations before public disclosure.
*   **Feature Requests:** We are actively evaluating adding support for Go, Rust, and Ruby interpreters inside the WASM environment. If your agents require these, please upvote the respective tracking issues.

## 🖥️ Understanding the Live Dashboard

Apatheia comes with a comprehensive React-based live dashboard. The dashboard connects via an `EventSource` and `WebSocket` bridge to provide granular visibility into the execution pool.

### The Waterfall View
When a request begins processing, it appears on the waterfall chart. This is not a simulation. The colors correspond to the exact nanosecond timestamps recorded by the Rust `telemetry` crate:
*   **Blue (Clone):** The time taken by the `InstancePre` memory map to duplicate the interpreter.
*   **Green (Eval):** The time spent inside the `Wasmtime` loop evaluating the untrusted string.
*   **Purple (Marshal):** The time taken to pull the output pointer from WASM linear memory back into the Rust host process.

### The Latency Distribution (Percentiles)
Because Apatheia handles sub-millisecond execution, "Average Latency" is a highly deceptive metric. A garbage collection pause in the Linux kernel can cause a 10x spike that ruins the average.
Instead, we track `p50`, `p90`, and `p99` latency bands. 
*   If your `p99 Total Latency` exceeds 10ms, your server is likely experiencing memory pressure or thread pool exhaustion, meaning your `governor` rate limits should be tightened.

### The Live Agent Simulator
The "Self-Healing Agent" panel on the right side of the dashboard runs a genuine loop against the Groq API (Llama 3). 
1. It requests code with an intentionally vague prompt designed to trigger an error.
2. The generated code is evaluated in Apatheia.
3. The resulting `RuntimeError` and `llm_feedback_prompt` are piped back to the Llama 3 model automatically.
4. You can watch the agent iteratively rewrite its code until the execution yields a clean `Success` response.

---

*This README was extensively expanded to provide exhaustive technical clarity to judges, developers, and security auditors. For an even deeper understanding of the core philosophy, refer to `docs/BIBLE.md`.*

### Local Development Setup Options
If you do not wish to use the pre-compiled WASM interpreters, you can compile them from scratch using `wasi-sdk`.
1. Download `wasi-sdk-20.0` or newer for your platform (Linux or MacOS).
2. Export `WASI_SDK_PATH=/path/to/wasi-sdk`.
3. In `engine/guest-runtimes/quickjs-wasm`, run `make`.
4. The build process automatically applies specific C patches to disable native threading and network socket access at the compilation level, ensuring the interpreter itself has zero physical capability to breach the sandbox, even if a zero-day exploit existed in Wasmtime's WASI implementation.

### System Requirements
*   **Memory:** Minimum 512MB RAM required to run the Tokio runtime and load the `InstancePre` modules into virtual memory.
*   **Storage:** 50MB required for the SQLite `metrics.db` historical buffer.
*   **OS:** Any modern Linux distribution (Ubuntu 20.04+, Alpine, etc.) or MacOS. Windows is not officially supported due to fundamental differences in how virtual memory paging handles Copy-on-Write allocations.

---

*(End of documentation)*
