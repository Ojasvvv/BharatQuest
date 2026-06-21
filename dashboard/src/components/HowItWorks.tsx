import React from 'react';

export const HowItWorks: React.FC = () => {
  return (
    <div style={{ width: '100%', maxWidth: '1200px', margin: '0 auto', display: 'flex', flexDirection: 'column', gap: '16px' }}>
      
      {/* Top Banner Panel */}
      <div className="panel">
        <div className="panel-head">
          <div>
            <div className="panel-title">🚨 The Core Problem</div>
            <div className="panel-title-sub">Why Virtual Machines are too slow for AI Agents</div>
          </div>
        </div>
        <div className="panel-body" style={{ padding: '20px', color: 'var(--text-secondary)', lineHeight: '1.6' }}>
          <p>
            When an AI agent attempts to solve a complex problem, it cannot rely on its internal "knowledge" for exact math, live database queries, or algorithmic logic. It <strong>must</strong> be able to write and execute code. However, executing AI-generated code directly on your infrastructure is dangerous (e.g., infinite loops, SSRF attacks). Traditional isolation methods like Docker or AWS Firecracker take hundreds of milliseconds to boot. <strong>Apatheia bridges this gap: Virtual Machine security with API-level latency.</strong>
          </p>
        </div>
      </div>

      <div className="grid-main" style={{ gridTemplateColumns: '1fr 1fr', marginBottom: 0 }}>
        <div className="panel">
          <div className="panel-head">
            <div className="panel-title">🏗️ What Apatheia Actually Is</div>
          </div>
          <div className="panel-body" style={{ padding: '20px', color: 'var(--text-secondary)', lineHeight: '1.6' }}>
             <p>Apatheia is a high-performance execution engine built entirely in Rust. Instead of relying on Linux kernel namespaces, it uses <strong>WebAssembly (WASM)</strong> to create a mathematically sealed, user-space quarantine zone. We pre-compile language interpreters (like QuickJS) into WASM binaries. When a request arrives, Wasmtime instantiates this module instantly.</p>
          </div>
        </div>

        <div className="panel">
          <div className="panel-head">
             <div className="panel-title">🔬 Copy-on-Write Boot Times</div>
          </div>
          <div className="panel-body" style={{ padding: '20px', color: 'var(--text-secondary)', lineHeight: '1.6' }}>
             <p>Wasmtime uses <strong>Copy-on-Write (COW)</strong> to map new sandbox memory directly to a pre-compiled, frozen interpreter in physical RAM. The Linux Kernel only makes a physical copy of a memory page if the new sandbox modifies it. The result: Apatheia spins up isolated sandboxes sharing the same baseline RAM footprint in a fraction of a millisecond (~50µs).</p>
          </div>
        </div>
      </div>

      <div className="panel">
         <div className="panel-head">
           <div>
             <div className="panel-title">🛡️ Security Model (Deep Dive)</div>
             <div className="panel-title-sub">Multi-layered defense-in-depth strategy</div>
           </div>
         </div>
         <div className="panel-body" style={{ padding: '20px' }}>
            <div className="metric-row" style={{ gridTemplateColumns: '1fr 1fr', gap: '1px', background: 'var(--border)', margin: 0 }}>
               
               <div className="metric" style={{ background: 'var(--surface)', padding: '20px' }}>
                  <div className="metric-label" style={{ color: 'var(--mint)', marginBottom: '12px' }}>Linear Memory Isolation</div>
                  <div style={{ color: 'var(--text-secondary)', fontSize: '12px', lineHeight: '1.6' }}>
                    Code runs in a strict WASM memory block with no physical access to the host machine. It structurally cannot read <code>.env</code> files or network states outside its sandbox.
                  </div>
               </div>

               <div className="metric" style={{ background: 'var(--surface)', padding: '20px' }}>
                  <div className="metric-label" style={{ color: 'var(--amber)', marginBottom: '12px' }}>Deterministic Fuel Metering</div>
                  <div style={{ color: 'var(--text-secondary)', fontSize: '12px', lineHeight: '1.6' }}>
                    Executions burn "fuel" (opcodes). Infinite loops exhaust fuel almost instantly, allowing Wasmtime to safely abort the execution without tying up your CPU core.
                  </div>
               </div>

               <div className="metric" style={{ background: 'var(--surface)', padding: '20px' }}>
                  <div className="metric-label" style={{ color: 'var(--red)', marginBottom: '12px' }}>Wall-Clock Watchdog</div>
                  <div style={{ color: 'var(--text-secondary)', fontSize: '12px', lineHeight: '1.6' }}>
                    Network timeouts are caught by a strict Tokio <code>timeout</code> wrapper, ensuring deadlocks inside the sandbox never hang the Rust backend.
                  </div>
               </div>

               <div className="metric" style={{ background: 'var(--surface)', padding: '20px' }}>
                  <div className="metric-label" style={{ color: 'var(--blue)', marginBottom: '12px' }}>The Ironclad SSRF Firewall</div>
                  <div style={{ color: 'var(--text-secondary)', fontSize: '12px', lineHeight: '1.6' }}>
                    An FFI-bridge intercepts WASM network calls, blocking private IPs, disabling auto-redirects, and resolving DNS manually to prevent DNS rebinding attacks.
                  </div>
               </div>

            </div>
         </div>
      </div>

      {/* Performance Numbers */}
      <div className="panel" style={{ marginTop: '16px' }}>
         <div className="panel-head">
           <div>
             <div className="panel-title">⚡ Real Performance Numbers</div>
             <div className="panel-title-sub">Measured live on Render Free Tier (Linux container)</div>
           </div>
         </div>
         <div className="panel-body" style={{ padding: '20px' }}>
            <div className="metric-row" style={{ gridTemplateColumns: '1fr 1fr 1fr', gap: '1px', background: 'var(--border)', margin: 0 }}>
               <div className="metric" style={{ background: 'var(--surface)', padding: '20px' }}>
                  <div className="metric-label" style={{ color: 'var(--mint)', marginBottom: '12px' }}>Sandbox Cold Start</div>
                  <div className="metric-value-row" style={{ marginBottom: '8px' }}>
                     <span className="metric-value">~50</span>
                     <span className="metric-unit">µs</span>
                  </div>
                  <div style={{ color: 'var(--text-secondary)', fontSize: '12px', lineHeight: '1.6' }}>
                    0.05 milliseconds. Thanks to the Pooling Allocator and <code>memory_init_cow</code>, virtual memory pages point directly to pre-existing QuickJS binaries without allocating new physical RAM.
                  </div>
               </div>
               
               <div className="metric" style={{ background: 'var(--surface)', padding: '20px' }}>
                  <div className="metric-label" style={{ color: 'var(--amber)', marginBottom: '12px' }}>Infinite Loop Rejection</div>
                  <div className="metric-value-row" style={{ marginBottom: '8px' }}>
                     <span className="metric-value">~4.4</span>
                     <span className="metric-unit">ms</span>
                  </div>
                  <div style={{ color: 'var(--text-secondary)', fontSize: '12px', lineHeight: '1.6' }}>
                    Unlike traditional 5-second timeouts that let <code>while(true)</code> burn 100% of a CPU core, Apatheia traps and aborts infinite loops almost instantly via deterministic fuel exhaustion.
                  </div>
               </div>

               <div className="metric" style={{ background: 'var(--surface)', padding: '20px' }}>
                  <div className="metric-label" style={{ color: 'var(--blue)', marginBottom: '12px' }}>High-Concurrency</div>
                  <div className="metric-value-row" style={{ marginBottom: '8px' }}>
                     <span className="metric-value">20</span>
                     <span className="metric-unit">req/sec</span>
                  </div>
                  <div style={{ color: 'var(--text-secondary)', fontSize: '12px', lineHeight: '1.6' }}>
                    Handled successfully across 64MB and 256MB boundaries during rigorous load testing via Tokio's asynchronous <code>spawn_blocking</code> thread pool.
                  </div>
               </div>
            </div>
         </div>
      </div>

      {/* Comparison Table */}
      <div className="panel">
        <div className="panel-head">
          <div className="panel-title">📊 Cold-Start Comparison Table</div>
        </div>
        <div className="panel-body" style={{ padding: 0 }}>
          <table style={{ width: '100%', borderCollapse: 'collapse', textAlign: 'left', fontSize: '13px' }}>
            <thead>
              <tr style={{ background: 'var(--surface-2)', borderBottom: '1px solid var(--border)' }}>
                <th style={{ padding: '12px 20px', color: 'var(--text-tertiary)', fontWeight: 500, textTransform: 'uppercase', fontSize: '10px', letterSpacing: '0.05em' }}>Platform / Technology</th>
                <th style={{ padding: '12px 20px', color: 'var(--text-tertiary)', fontWeight: 500, textTransform: 'uppercase', fontSize: '10px', letterSpacing: '0.05em' }}>Cold Start Time</th>
                <th style={{ padding: '12px 20px', color: 'var(--text-tertiary)', fontWeight: 500, textTransform: 'uppercase', fontSize: '10px', letterSpacing: '0.05em' }}>Measurement Source</th>
              </tr>
            </thead>
            <tbody>
              <tr style={{ borderBottom: '1px solid var(--border)' }}>
                <td style={{ padding: '16px 20px', color: 'var(--mint)', fontWeight: 600 }}>Apatheia (WASM + COW)</td>
                <td style={{ padding: '16px 20px', fontFamily: 'monospace' }}>~0.06 ms</td>
                <td style={{ padding: '16px 20px', color: 'var(--text-secondary)', fontSize: '12px' }}>Self-measured (emitted from Rust backend)</td>
              </tr>
              <tr style={{ borderBottom: '1px solid var(--border)' }}>
                <td style={{ padding: '16px 20px', color: 'var(--text)' }}>Native Subprocess (fork)</td>
                <td style={{ padding: '16px 20px', fontFamily: 'monospace' }}>~1 - 5 ms</td>
                <td style={{ padding: '16px 20px', color: 'var(--text-secondary)', fontSize: '12px' }}>Standard Linux fork() and execve() baseline</td>
              </tr>
              <tr style={{ borderBottom: '1px solid var(--border)' }}>
                <td style={{ padding: '16px 20px', color: 'var(--text)' }}>AWS Firecracker (MicroVM)</td>
                <td style={{ padding: '16px 20px', fontFamily: 'monospace' }}>≤ 125 ms</td>
                <td style={{ padding: '16px 20px', color: 'var(--text-secondary)', fontSize: '12px' }}>AWS Official Documentation</td>
              </tr>
              <tr style={{ borderBottom: '1px solid var(--border)' }}>
                <td style={{ padding: '16px 20px', color: 'var(--text)' }}>E2B (AI Sandbox API)</td>
                <td style={{ padding: '16px 20px', fontFamily: 'monospace' }}>~150 - 500 ms</td>
                <td style={{ padding: '16px 20px', color: 'var(--text-secondary)', fontSize: '12px' }}>Self-reported / Unverified</td>
              </tr>
              <tr>
                <td style={{ padding: '16px 20px', color: 'var(--text)' }}>Docker Container</td>
                <td style={{ padding: '16px 20px', fontFamily: 'monospace' }}>~500 ms - 1.5 s</td>
                <td style={{ padding: '16px 20px', color: 'var(--text-secondary)', fontSize: '12px' }}>General Industry Benchmarks</td>
              </tr>
            </tbody>
          </table>
          <div style={{ padding: '16px 20px', background: 'var(--surface-2)', borderTop: '1px solid var(--border)', fontSize: '12px', color: 'var(--text-secondary)' }}>
             <strong>Why this matters:</strong> If an AI agent does 10 code executions to solve a problem, Docker adds 10+ seconds of pure "waiting to boot" latency. Apatheia adds 0.0006 seconds.
          </div>
        </div>
      </div>

      {/* FAQ */}
      <div className="panel" style={{ marginBottom: '40px' }}>
        <div className="panel-head">
          <div className="panel-title">🤔 Exhaustive FAQ</div>
        </div>
        <div className="panel-body" style={{ padding: '24px', color: 'var(--text-secondary)', lineHeight: '1.6' }}>
          
          <div style={{ marginBottom: '24px' }}>
            <h4 style={{ color: 'var(--text)', fontSize: '14px', marginBottom: '8px' }}>Q: What is Apatheia, in one sentence?</h4>
            <p>A: Apatheia is a high-performance execution engine that allows AI agents to write and run untrusted code safely in microseconds using WebAssembly isolation.</p>
          </div>

          <div style={{ marginBottom: '24px' }}>
            <h4 style={{ color: 'var(--text)', fontSize: '14px', marginBottom: '8px' }}>Q: Why not just use Docker?</h4>
            <p>A: Docker is an incredible tool for deploying long-running microservices, but it is too slow for real-time AI thought loops. Docker relies on Linux kernel namespaces, cgroups, and filesystem overlays. Creating these structures takes the Linux kernel anywhere from 500ms to 2 seconds. Apatheia clones sandboxes entirely in user-space memory in ~0.06ms. However, Docker is vastly superior if the code your AI generates requires complex native operating system dependencies, package managers (like apt-get), or access to heavy C-libraries.</p>
          </div>

          <div style={{ marginBottom: '24px' }}>
            <h4 style={{ color: 'var(--text)', fontSize: '14px', marginBottom: '8px' }}>Q: Why not use AWS Firecracker or a microVM?</h4>
            <p>A: Firecracker (which powers AWS Lambda) is the industry standard for secure, multi-tenant serverless execution, boasting boot times of ≤ 125ms. However, Firecracker still boots an entire minimized Linux kernel. Apatheia skips the Linux kernel entirely. We run the language interpreter directly inside a WASM linear memory block on the host OS. Firecracker provides stronger isolation against CPU-level vulnerabilities (like Spectre/Meltdown), while Apatheia provides vastly superior latency.</p>
          </div>

          <div style={{ marginBottom: '24px' }}>
            <h4 style={{ color: 'var(--text)', fontSize: '14px', marginBottom: '8px' }}>Q: Is Apatheia safe from SSRF (Server-Side Request Forgery) attacks?</h4>
            <p>A: Yes. Because we use WASI (WebAssembly System Interface), the untrusted code has absolutely no direct access to the host's networking stack. The code must ask the host server to fetch URLs via our FFI bridge. Our bridge manually resolves the DNS of every URL, blocks all private IP ranges, blocks Cloud Metadata endpoints, disables automatic redirects, and forces the HTTP client to use the pre-validated IP to defeat DNS Rebinding attacks.</p>
          </div>

          <div>
            <h4 style={{ color: 'var(--text)', fontSize: '14px', marginBottom: '8px' }}>Q: Did anything go wrong while building this? Be honest.</h4>
            <p>A: Yes. During Phase 6 load testing on a live Render deployment, we discovered a catastrophic deadlock. Our DashMap rate-limiter was holding a synchronous OS-level lock across an asynchronous Tokio .await point. When a single execution request blocked waiting for a simulated 4-second HTTP network fetch, it held the lock. The next 19 requests queued up waiting for the lock, freezing the entire Toko worker pool and crashing the server. We caught it via rigorous stress testing, isolated the lexical scope to force the lock to drop early, and proved the fix under repeated load.</p>
          </div>

        </div>
      </div>

    </div>
  );
};
