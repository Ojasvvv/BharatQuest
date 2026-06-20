import React, { useState } from 'react';

export const AgentSimulator: React.FC = () => {
  const [code, setCode] = useState('const data = [1, 2];\nconsole.log(data.mapIsCool(x => x));');
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<any>(null);

  const handleRun = async () => {
    setLoading(true);
    setResult(null);
    try {
      const reqId = 'req-' + Math.random().toString(36).substring(2, 6);
      const res = await fetch('http://127.0.0.1:3000/v1/execute', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          request_id: reqId,
          language: 'javascript',
          code,
          timeout_ms: 1000,
          memory_limit_mb: 128
        })
      });
      const data = await res.json();
      setResult(data);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="sim-panel">
      <div className="panel-head">
        <div>
          <div className="panel-title">Agent simulator</div>
          <div className="panel-title-sub">Run JS through the sandbox and watch the self-healing loop</div>
        </div>
        <div className="panel-badge">interactive</div>
      </div>
      <div className="sim-body">
        <div className="sim-input-col">
          <div className="input-label">Input</div>
          <textarea 
            className="code-input" 
            spellCheck="false" 
            value={code} 
            onChange={(e) => setCode(e.target.value)}
          />
          <button className="run-btn" onClick={handleRun} disabled={loading}>
            {loading ? 'Running...' : 'Run in sandbox'}
          </button>
        </div>
        <div className="sim-trace-col">
          <div className="input-label">Execution trace</div>
          {!result && <div style={{ color: 'var(--text-tertiary)', fontSize: '11px' }}>No execution yet</div>}
          
          {result && result.status === 'success' && (
            <div className="loop-trace">
              <div className="loop-step">
                <div className="loop-marker marker-ok">1</div>
                <div className="loop-content">
                  <div className="loop-title">Execution succeeded</div>
                  <div className="loop-detail">completed in {result.metrics.total_time_us}µs<br/>{result.stdout && `stdout: ${result.stdout}`}</div>
                </div>
              </div>
            </div>
          )}

          {result && result.status === 'runtime_error' && (
            <div className="loop-trace">
              <div className="loop-step">
                <div className="loop-marker marker-fail">1</div>
                <div className="loop-content">
                  <div className="loop-title">Execution failed — {result.error_telemetry.type}</div>
                  <div className="loop-detail">{result.error_telemetry.message}<br/>trapped in {result.metrics.total_time_us}µs</div>
                </div>
              </div>
              <div className="loop-step">
                <div className="loop-marker marker-fail">2</div>
                <div className="loop-content">
                  <div className="loop-title">Feedback returned to model</div>
                  <div className="loop-detail">llm_feedback_prompt injected into context</div>
                </div>
              </div>
            </div>
          )}

          {result && result.status === 'rejected' && (
            <div className="loop-trace">
              <div className="loop-step">
                <div className="loop-marker marker-fail">1</div>
                <div className="loop-content">
                  <div className="loop-title">Execution Rejected</div>
                  <div className="loop-detail">reason: {result.reason}</div>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
      
      {result && result.status === 'rejected' && (
        <div className="rejected-banner">
          <div className="rejected-left">
            <div className="rejected-icon">!</div>
            <div className="rejected-text">Last <b>rejected</b> request — {result.reason}</div>
          </div>
          <div className="rejected-fuel">
            {result.metrics ? `fuel_consumed: ${result.metrics.fuel_consumed.toLocaleString()}` : 'Missing metrics'}
          </div>
        </div>
      )}
    </div>
  );
};
