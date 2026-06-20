import React from 'react';

export const HowItWorks: React.FC = () => {
  return (
    <div className="panel" style={{ maxWidth: '850px', margin: '0 auto', marginBottom: '40px' }}>
      <div className="panel-head">
        <div>
          <div className="panel-title">Apatheia Architecture</div>
          <div className="panel-title-sub">How the sub-millisecond WASM sandbox works</div>
        </div>
      </div>
      <div className="panel-body" style={{ padding: '24px', color: 'var(--text-secondary)', lineHeight: '1.6', fontSize: '13px' }}>
        <h3 style={{ color: 'var(--text)', marginBottom: '10px', fontSize: '14px', fontWeight: 600 }}>1. The Sandbox Engine (Rust + Wasmtime)</h3>
        <p style={{ marginBottom: '24px' }}>
          At the core of Apatheia is a custom engine built in Rust using Wasmtime. Instead of spinning up heavy Docker containers or V8 isolates, Apatheia uses WebAssembly (WASM) for absolute memory safety and near-instant cold starts.
        </p>

        <h3 style={{ color: 'var(--text)', marginBottom: '10px', fontSize: '14px', fontWeight: 600 }}>2. QuickJS Cross-Compilation</h3>
        <p style={{ marginBottom: '24px' }}>
          To execute JavaScript, the engine doesn't rely on Node.js or V8. Instead, the lightweight QuickJS C engine is cross-compiled entirely into a single <code>.wasm</code> binary using the WASI SDK. When an API request comes in, Wasmtime instantiates this pre-compiled module in under 40 microseconds.
        </p>

        <h3 style={{ color: 'var(--text)', marginBottom: '10px', fontSize: '14px', fontWeight: 600 }}>3. Deterministic Safety Constraints</h3>
        <p style={{ marginBottom: '14px' }}>Code execution is strictly sandboxed by the host. Apatheia prevents malicious or runaway code through three distinct mechanisms:</p>
        <ul style={{ paddingLeft: '24px', marginBottom: '24px', listStyleType: 'disc' }}>
          <li style={{ marginBottom: '6px' }}><strong style={{ color: 'var(--text)' }}>Memory Limits:</strong> The linear memory of the WASM module is hard-capped (e.g., 256MB). If the JS code tries to allocate more, the WASM host traps the OOM error instantly.</li>
          <li style={{ marginBottom: '6px' }}><strong style={{ color: 'var(--text)' }}>Fuel Metering:</strong> To prevent infinite loops (like <code>while(true)</code>), every opcode executed consumes "fuel". If the script exhausts its fuel limit, execution is immediately trapped and safely halted.</li>
          <li><strong style={{ color: 'var(--text)' }}>Wall-clock Watchdog:</strong> A Tokio-based async timeout ensures that even if fuel metering misses something, the thread is forcefully terminated after a strict wall-clock duration.</li>
        </ul>

        <h3 style={{ color: 'var(--text)', marginBottom: '10px', fontSize: '14px', fontWeight: 600 }}>4. The Self-Healing Loop for AI Agents</h3>
        <p style={{ marginBottom: '24px' }}>
          When code fails (such as attempting to call a hallucinated method that doesn't exist), the engine doesn't just crash. It traps the <code>RuntimeError</code>, captures the exact execution trace, and constructs an <code>llm_feedback_prompt</code>. This structured feedback is instantly sent back to the autonomous AI agent, allowing it to learn from its syntax error and generate corrected code.
        </p>

        <h3 style={{ color: 'var(--text)', marginBottom: '10px', fontSize: '14px', fontWeight: 600 }}>5. Live Telemetry Stream</h3>
        <p>
          Every execution phase (instance cloning, JS evaluation, memory marshaling) is tracked by the engine in exact microseconds. These latency metrics are broadcast live over a WebSocket stream from the Axum API to this React dashboard, ensuring complete transparency into the system's low-latency performance characteristics.
        </p>
      </div>
    </div>
  );
};
