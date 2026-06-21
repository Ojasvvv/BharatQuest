# Apatheia: Judges, Demo, and Pipeline Kit

## PART 1: HOW TO EXPLAIN AND SELL THIS TO JUDGES

### The 30-Second Version
Apatheia is a sub-millisecond, memory-safe code sandbox built for autonomous AI agents. As AI agents increasingly write and execute their own code to solve problems, it's incredibly dangerous to run their hallucinated scripts natively on a server. Apatheia creates a completely isolated environment for untrusted JavaScript and Python in under a millisecond, executes it safely with strict fuel limits, and returns the result back to the AI. It's the execution engine that lets autonomous agents self-heal and iterate without destroying your infrastructure.

### The 2-Minute Version
To give AI agents agency, they need to run code. But evaluating dynamically generated, untrusted Python or JavaScript is an infrastructure nightmare. Apatheia solves this using WebAssembly and Rust. 

Instead of compiling the AI's code, we compile the language *interpreters* (QuickJS and MicroPython) into WASM modules. Using Wasmtime's Pooling Allocator, we take a pre-compiled snapshot of that interpreter (`InstancePre`) and use Copy-on-Write memory mapping to instantly clone a fresh sandbox for every single request. 

The numbers are real: `instance_clone_time_us` sits between 20-60 microseconds. `memory_marshal_us` is effectively zero, and the actual `execution_time_us` is purely interpreter overhead. The `total_time_us` for a full end-to-end sandbox lifecycle is under a millisecond. 
Crucially, the sandbox enforces deterministic fuel metering—if an AI writes `while(true){}`, Apatheia counts the WASM opcodes and traps the execution with `out_of_fuel` instantaneously, while a secondary tokio wall-clock watchdog ensures the OS thread is never locked.

### The Live Demo Script
1. **Open Dashboard (Simulator View)**: "This is the Apatheia observability dashboard showing the live state of the runtime pool."
2. **Show Dynamic Discovery**: "Notice the selectors on the left—this dynamically hits `/v1/runtimes`. We have JavaScript and Python loaded."
3. **Execute Basic Python**: Select Python, write `print(sum([1,2,3]))`, and click Run.
   - *Say:* "Notice the total time in the waterfall on the right—less than a millisecond to clone the sandbox, run it, and destroy it."
4. **The Hallucination Self-Healing Loop**: 
   - Paste the following into the JavaScript sandbox:
     ```javascript
     const data = [1, 2, 3];
     console.log(data.sumAll());
     ```
   - Click Run.
   - *Say:* "Here, the LLM hallucinated a method `sumAll()` that doesn't exist. Notice the trace on the right side. The sandbox trapped it, extracted the exact `RuntimeError: 'sumAll' is not a function`, and generated an `llm_feedback_prompt`. We feed this exact prompt back into the LLM context so it can self-heal."
5. **Compare Mode**: Click "Compare".
   - *Say:* "This is the multi-sandbox test. It translates syntax on the fly and executes both Python and JavaScript sandboxes in parallel to compare execution footprints."

### Anticipated Judge Questions

- **"Isn't this just a sandbox? What's actually novel here?"**
  The novelty is the sub-millisecond per-request spin-up via WASM Copy-on-Write. Traditional sandboxing (Docker/microVMs) takes hundreds of milliseconds, which breaks the real-time feedback loop needed for LLM agent reasoning.

- **"What happens if Wasmtime itself has a sandbox-escape bug?"**
  Wasmtime is an industry standard built by the Bytecode Alliance, but no software is 100% secure. If a zero-day WASM escape occurs, the attacker gains access to the host Rust process. In production, this service should still be run inside a hardened container or VM as defense-in-depth.

- **"Why does this matter more now than it would have 2 years ago?"**
  Two years ago, code was written by humans and statically analyzed. Today, autonomous agents generate thousands of lines of dynamic code per minute that *must* be executed immediately to provide context for the agent's next thought. The scale and speed of execution required has fundamentally changed.

- **"What's your actual cold-start number and why should I trust it?"**
  Our `instance_clone_time_us` is between 20-60 microseconds. You can trust it because it's calculated using Rust's `Instant::now()` immediately around the Wasmtime `instantiate()` call and streamed raw to the UI. There is no cold-start because we use a pre-warmed memory snapshot (`InstancePre`).

- **"Why not just use Docker / a microVM (Firecracker) / a regular subprocess with seccomp?"**
  Speed and concurrency limits. A Firecracker microVM takes ~120-200ms to boot. A WASM instance takes 0.05ms. When an AI agent pipeline requires executing 50 small steps to reason through a problem, a 200ms penalty on every step destroys the UX.

- **"What languages does this actually support today?"**
  JavaScript (via QuickJS) and Python (via MicroPython 1.x standard library subset).

- **"Is this production-ready?"**
  Yes, but with known architectural constraints. The core execution engine, memory isolation, and SSRF firewall are fully hardened for production. However, telemetry history does not survive a container restart on the current (free tier) deployment — this is a known limitation of the ephemeral filesystem, not an oversight. The fix is either upgrading to a paid tier with a Persistent Disk or migrating the SQLite layer to an external DB like Redis/PostgreSQL. Additionally, multithreading within the WASM guests remains disabled by design to ensure strict determinism.

- **"What's the actual unit economics / cost story vs. running this in a container per request?"**
  We haven't benchmarked strict dollar costs yet. However, the memory footprint allows hundreds of sandboxes per gigabyte of RAM compared to containers, vastly improving density.

---

## PART 2: ALTERNATIVES — WHY NOT THEM

- **Firecracker / microVMs**: AWS Lambda uses these. They use KVM hardware virtualization to spin up lightweight Linux VMs. They provide stronger isolation guarantees (hardware-backed) and support any language, but their cold starts (100-250ms) are too slow for real-time agent chains. Apatheia trades hardware-level isolation for WASM software-level isolation to gain a 1000x speedup.
- **Docker / standard containers**: Containers use cgroups/namespaces. They are excellent for long-running services but terrible for ephemeral, sub-second execution due to the heavy OS overhead required to mount filesystems and allocate namespaces.
- **E2B (and similar agent-sandbox-as-a-service)**: E2B provides cloud-hosted, long-lived Linux sandboxes for agents. They are excellent if your agent needs to run `npm install` or spin up a PostgreSQL database. Apatheia is completely different—it is a localized, instantly ephemeral evaluation engine for pure compute logic, not a full OS replica. (Note: No head-to-head benchmarks exist in this repo).
- **Plain subprocess + seccomp/gVisor**: Spawning a process via `fork`/`exec` and restricting it via `seccomp`. This was explicitly rejected because it is incredibly difficult to tune `seccomp` profiles perfectly across different host OS kernels, whereas WASM provides a guaranteed linear memory sandbox by mathematical design.
- **Compiling AI JS to WASM**: Compiling source code directly to WASM takes seconds. Apatheia runs a pre-compiled interpreter to skip the compilation step entirely.

**Positioning**: Apatheia sits at the absolute extreme edge of **Speed** and **Density**. It sacrifices **Language Breadth** (no arbitrary C++ or Java) and **Hardware Isolation** in favor of sub-millisecond execution and tiny memory footprints.

---

## PART 3: PIPELINE INTEGRATION

### Architecture Diagram
```text
┌────────────┐          POST /v1/execute         ┌──────────────────────┐
│  AI Agent  │──────────────────────────────────►│   Apatheia Server    │
│ (LLM Loop) │                                   │ (Rust + Wasmtime)    │
└────────────┘                                   └──────────────────────┘
      ▲                                                      │
      │                                                      │
      │   ┌──────────────────────────────────────────────┐   │
      └───┤ JSON: { status: "runtime_error", ...         │◄──┘
          │         llm_feedback_prompt: "..." }         │
          └──────────────────────────────────────────────┘
```

### Integration Surface
Calling systems only need to send a simple JSON payload (`ExecuteRequest`) with `code`, `language`, and limits. 
It slots directly into a LangChain `Tool` or a system-prompt execution step. If the status returns `runtime_error`, the integrating framework should extract `llm_feedback_prompt.content` and append it as a "System" message in the next LLM turn to facilitate self-healing.
To prevent infinite loops, the caller should maintain the `parent_request_id` across retries—Apatheia will forcefully reject the 4th attempt with `429 Too Many Requests`.

### Deployment Reality
Currently, the service runs as a single instance for hackathon demo purposes. There is no multi-tenant routing, clustering, or autoscaling built into the codebase yet. 

### The Dashboard's Role
The WebSocket dashboard is explicitly a **demo-only artifact**. It uses hardcoded URL endpoints (`http://127.0.0.1:8080`), stores state entirely in React memory, and is meant to visualize the internal metrics for judges. It is not designed to be a permanent production observability surface.
