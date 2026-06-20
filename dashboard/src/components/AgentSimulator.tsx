import React, { useState, useEffect } from 'react';
import { Runtime } from '../hooks/useRuntimes';

interface AgentSimulatorProps {
  runtimes?: Runtime[];
}

const EXAMPLE_SNIPPETS: Record<string, string> = {
  javascript: `const data = [1, 2, 3, 4, 5];\nconsole.log(data.reduce((a, b) => a + b, 0));`,
  python: `data = [1, 2, 3, 4, 5]\nprint(sum(data))`,
};

export const AgentSimulator: React.FC<AgentSimulatorProps> = ({ runtimes = [] }) => {
  const [selectedLanguage, setSelectedLanguage] = useState<string>('');
  const [code, setCode] = useState('');
  
  useEffect(() => {
    if (runtimes.length > 0 && !selectedLanguage) {
      const first = runtimes.find(r => r.status === 'ready');
      if (first) setSelectedLanguage(first.id);
    }
  }, [runtimes, selectedLanguage]);

  useEffect(() => {
    if (!selectedLanguage) return;
    const currentIsExample = Object.values(EXAMPLE_SNIPPETS).includes(code.trim());
    if (!code.trim() || currentIsExample) {
      setCode(EXAMPLE_SNIPPETS[selectedLanguage] ?? '');
    }
  }, [selectedLanguage]);

  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<any>(null);

  const [compareMode, setCompareMode] = useState(false);
  const [compareCode, setCompareCode] = useState('data = [1, 2, 3, 4, 5]\nprint(sum(data))');
  const [compareResults, setCompareResults] = useState<{ javascript: any | null, python: any | null }>({ javascript: null, python: null });
  const [compareLoading, setCompareLoading] = useState<{ javascript: boolean, python: boolean }>({ javascript: false, python: false });

  const runCompare = async () => {
    const codeToRun = compareCode || 'data = [1, 2, 3, 4, 5]\nprint(sum(data))';
    const jsCode = "const sum = (arr) => arr.reduce((a, b) => a + b, 0);\n" + codeToRun.replace(/^print\(/gm, 'console.log(').replace(/^#/gm, '//');
    const pyCode = codeToRun;

    setCompareLoading({ javascript: true, python: true });
    setCompareResults({ javascript: null, python: null });

    const jsPromise = fetch('http://127.0.0.1:8080/v1/execute', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        request_id: `compare-js-${Date.now()}`,
        language: 'javascript',
        code: jsCode,
        timeout_ms: 5000,
        memory_limit_mb: 64,
      })
    }).then(r => r.json()).finally(() => setCompareLoading(prev => ({ ...prev, javascript: false })));

    const pyPromise = fetch('http://127.0.0.1:8080/v1/execute', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        request_id: `compare-py-${Date.now()}`,
        language: 'python',
        code: pyCode,
        timeout_ms: 5000,
        memory_limit_mb: 64,
      })
    }).then(r => r.json()).finally(() => setCompareLoading(prev => ({ ...prev, python: false })));

    jsPromise.then(res => setCompareResults(prev => ({ ...prev, javascript: res })));
    pyPromise.then(res => setCompareResults(prev => ({ ...prev, python: res })));
  };

  const handleRun = async () => {
    setLoading(true);
    setResult(null);
    try {
      const reqId = 'req-' + Math.random().toString(36).substring(2, 6);
      const res = await fetch('http://127.0.0.1:8080/v1/execute', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          request_id: reqId,
          language: selectedLanguage,
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
      <div className="panel-head" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div>
          <div className="panel-title">Agent simulator</div>
          <div className="panel-title-sub">Run code through the sandbox and watch the self-healing loop</div>
        </div>
        <div style={{ display: 'flex', gap: '8px' }}>
          <button 
            onClick={() => setCompareMode(!compareMode)}
            style={{ fontSize: '11px', padding: '2px 8px', borderRadius: '4px', background: compareMode ? '#166534' : '#374151', color: compareMode ? '#4ade80' : '#d1d5db', cursor: 'pointer', border: 'none' }}
          >
            {compareMode ? 'Exit Compare' : 'Compare'}
          </button>
          <div className="panel-badge">interactive</div>
        </div>
      </div>
      <div className="sim-body">
        {compareMode ? (
          <div style={{ width: '100%', padding: '16px' }}>
            <div className="mb-2 text-xs text-gray-500" style={{ marginBottom: '8px', fontSize: '12px', color: '#6b7280' }}>
              Python syntax · JavaScript column adapts console output
            </div>
            <textarea
              value={compareCode}
              onChange={e => setCompareCode(e.target.value)}
              style={{ width: '100%', fontFamily: 'monospace', fontSize: '12px', backgroundColor: '#111827', color: '#86efac', border: '1px solid #374151', borderRadius: '4px', padding: '8px', height: '96px', marginBottom: '8px' }}
            />
            <button 
              onClick={runCompare} 
              style={{ width: '100%', padding: '8px', background: '#3b82f6', color: 'white', borderRadius: '4px', cursor: 'pointer', border: 'none' }}
            >
              Run in both sandboxes
            </button>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '12px', marginTop: '12px' }}>
              {['javascript', 'python'].map(lang => {
                const runtime = runtimes.find(r => r.id === lang);
                const result = compareResults[lang as 'javascript' | 'python'];
                const isLoading = compareLoading[lang as 'javascript' | 'python'];
                return (
                  <div key={lang} style={{ border: '1px solid #374151', borderRadius: '4px', padding: '8px' }}>
                    <div style={{ fontSize: '12px', fontFamily: 'monospace', color: '#9ca3af', marginBottom: '8px' }}>
                      {runtime?.label ?? lang}
                    </div>
                    {isLoading && (
                      <div style={{ fontSize: '12px', color: '#6b7280', animation: 'pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite' }}>
                        Executing...
                      </div>
                    )}
                    {result && !isLoading && (
                      <div>
                        <div style={{ fontSize: '12px', color: '#4ade80', fontFamily: 'monospace' }}>
                          {result.stdout}
                        </div>
                        <div style={{ fontSize: '12px', color: '#6b7280', marginTop: '4px' }}>
                          {result.metrics?.total_time_us}µs total
                        </div>
                        {result.metrics?.execution_time_us && (
                          <div style={{ fontSize: '12px', color: '#4b5563' }}>
                            {result.metrics.execution_time_us}µs exec
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        ) : (
          <>
            <div className="sim-input-col">
              <div className="input-label" style={{ display: 'flex', gap: '8px', marginBottom: '8px' }}>
                {runtimes.map(runtime => {
                  const isUnavailable = runtime.status === 'unavailable';
                  return (
                    <button
                      key={runtime.id}
                      onClick={() => !isUnavailable && setSelectedLanguage(runtime.id)}
                      disabled={isUnavailable}
                      title={isUnavailable ? 'Runtime unavailable' : undefined}
                      className={[
                        'px-3 py-1 text-xs font-mono rounded',
                        selectedLanguage === runtime.id
                          ? 'bg-green-900 text-green-300 border border-green-600'
                          : 'bg-gray-800 text-gray-400 border border-gray-700',
                        isUnavailable
                          ? 'opacity-40 cursor-not-allowed'
                          : 'cursor-pointer hover:border-green-700'
                      ].join(' ')}
                      style={{
                        padding: '4px 12px', fontSize: '12px', fontFamily: 'monospace', borderRadius: '4px',
                        background: selectedLanguage === runtime.id ? '#14532d' : '#1f2937',
                        color: selectedLanguage === runtime.id ? '#86efac' : '#9ca3af',
                        border: selectedLanguage === runtime.id ? '1px solid #16a34a' : '1px solid #374151',
                        opacity: isUnavailable ? 0.4 : 1,
                        cursor: isUnavailable ? 'not-allowed' : 'pointer',
                      }}
                    >
                      {runtime.label.split(' ')[0]}
                    </button>
                  );
                })}
              </div>
              
              {(() => {
                const selected = runtimes.find(r => r.id === selectedLanguage);
                return selected?.runtime_notes ? (
                  <div style={{ fontSize: '11px', color: '#60a5fa', marginTop: '4px', marginBottom: '8px', display: 'flex', alignItems: 'center', gap: '4px' }}>
                    <span>ℹ</span>
                    <span>{selected.runtime_notes}</span>
                  </div>
                ) : null;
              })()}

              <textarea 
                className="code-input" 
                spellCheck="false" 
                value={code} 
                onChange={(e) => setCode(e.target.value)}
              />
              <button className="run-btn" onClick={handleRun} disabled={loading || !selectedLanguage}>
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
          </>
        )}
      </div>
      
      {result && result.status === 'rejected' && !compareMode && (
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
