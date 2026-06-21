import React, { useState, useEffect, useRef } from 'react';

interface SseEvent {
  type: string;
  data: any;
  timestamp: number;
}

export const LiveAgentDemo: React.FC = () => {
  const [mode, setMode] = useState<'realistic_bug' | 'dangerous'>('realistic_bug');
  const [isRunning, setIsRunning] = useState(false);
  const [events, setEvents] = useState<SseEvent[]>([]);
  const eventSourceRef = useRef<EventSource | null>(null);
  const logsEndRef = useRef<HTMLDivElement>(null);

  const startLoop = () => {
    if (isRunning) return;
    setIsRunning(true);
    setEvents([]);

    const es = new EventSource(`/start?mode=${mode}`);
    eventSourceRef.current = es;

    const addEvent = (type: string, data: any) => {
      setEvents(prev => [...prev, { type, data, timestamp: Date.now() }]);
    };

    es.addEventListener('start', (e) => addEvent('start', JSON.parse(e.data)));
    es.addEventListener('llm_generating', (e) => addEvent('llm_generating', JSON.parse(e.data)));
    es.addEventListener('code_generated', (e) => addEvent('code_generated', JSON.parse(e.data)));
    es.addEventListener('apatheia_evaluating', (e) => addEvent('apatheia_evaluating', JSON.parse(e.data)));
    es.addEventListener('apatheia_result', (e) => addEvent('apatheia_result', JSON.parse(e.data)));
    es.addEventListener('retry_feedback', (e) => addEvent('retry_feedback', JSON.parse(e.data)));
    es.addEventListener('completed', (e) => {
        addEvent('completed', JSON.parse(e.data));
        es.close();
        setIsRunning(false);
    });
    es.addEventListener('error', (e: any) => {
      if (e.data) {
        addEvent('error', JSON.parse(e.data));
      }
      es.close();
      setIsRunning(false);
    });
  };

  const stopLoop = () => {
    if (eventSourceRef.current) {
      eventSourceRef.current.close();
    }
    setIsRunning(false);
  };

  useEffect(() => {
    return () => stopLoop();
  }, []);

  useEffect(() => {
    logsEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [events]);

  return (
    <div className="sim-panel" style={{ marginTop: '16px' }}>
      <div className="panel-head" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div>
          <div className="panel-title">Live Agent Demo (Real LLM Integration)</div>
          <div className="panel-title-sub">Streams real Groq + Apatheia interaction via SSE</div>
        </div>
        <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
          <select 
            value={mode} 
            onChange={(e) => setMode(e.target.value as any)}
            disabled={isRunning}
            style={{ 
              background: '#374151', 
              color: '#fff', 
              border: 'none', 
              borderRadius: '4px', 
              padding: '4px 8px', 
              fontSize: '12px' 
            }}
          >
            <option value="realistic_bug">Mode 1: Realistic Bug (Self-Heal Loop)</option>
            <option value="dangerous">Mode 2: Dangerous Code (Fuel Trap)</option>
          </select>
          <button 
            onClick={isRunning ? stopLoop : startLoop}
            style={{ 
              fontSize: '12px', 
              padding: '4px 12px', 
              borderRadius: '4px', 
              background: isRunning ? '#991b1b' : '#166534', 
              color: isRunning ? '#fca5a5' : '#4ade80', 
              cursor: 'pointer', 
              border: 'none' 
            }}
          >
            {isRunning ? 'Stop' : 'Start Agent'}
          </button>
        </div>
      </div>
      
      <div style={{ padding: '16px', display: 'flex', flexDirection: 'column', gap: '16px', maxHeight: '400px', overflowY: 'auto', background: '#0f172a' }}>
        {events.length === 0 && !isRunning && (
           <div style={{ color: '#64748b', fontSize: '12px', textAlign: 'center', padding: '20px' }}>
             Click "Start Agent" to begin live streaming. Ensure self-heal-server is running on port 3001.
           </div>
        )}
        
        {events.map((ev, i) => {
          if (ev.type === 'start') {
             return (
               <div key={i} style={{ padding: '12px', border: '1px solid #1e293b', borderRadius: '6px', background: '#1e293b' }}>
                 <div style={{ color: '#38bdf8', fontSize: '12px', fontWeight: 'bold' }}>📋 Task Goal</div>
                 <div style={{ color: '#cbd5e1', fontSize: '13px', marginTop: '4px', fontFamily: 'monospace' }}>{ev.data.task}</div>
               </div>
             );
          }

          if (ev.type === 'llm_generating') {
             return (
               <div key={i} style={{ color: '#eab308', fontSize: '12px', display: 'flex', alignItems: 'center', gap: '8px' }}>
                 <div className="spinner" style={{ width: '12px', height: '12px', border: '2px solid #eab308', borderTopColor: 'transparent', borderRadius: '50%', animation: 'spin 1s linear infinite' }} />
                 🧠 Iteration {ev.data.iteration}: Asking Groq (Llama 3.3)...
               </div>
             );
          }

          if (ev.type === 'code_generated') {
             return (
               <div key={i} style={{ padding: '12px', border: '1px solid #334155', borderRadius: '6px', background: '#020617' }}>
                 <div style={{ color: '#94a3b8', fontSize: '11px', marginBottom: '8px' }}>📝 Generated Code (Iteration {ev.data.iteration})</div>
                 <pre style={{ margin: 0, color: '#86efac', fontSize: '12px', whiteSpace: 'pre-wrap' }}>{ev.data.code}</pre>
               </div>
             );
          }

          if (ev.type === 'apatheia_evaluating') {
             return (
               <div key={i} style={{ color: '#a855f7', fontSize: '12px' }}>
                 ⚡ Sending to Apatheia Execution Engine...
               </div>
             );
          }

          if (ev.type === 'apatheia_result') {
             const res = ev.data.result;
             const isSuccess = res.status === 'success';
             const isError = res.status === 'runtime_error';
             const isRejected = res.status === 'rejected';
             
             return (
               <div key={i} style={{ padding: '12px', border: `1px solid ${isSuccess ? '#166534' : isError ? '#991b1b' : '#b45309'}`, borderRadius: '6px', background: isSuccess ? '#052e16' : isError ? '#450a0a' : '#451a03' }}>
                 <div style={{ color: isSuccess ? '#4ade80' : isError ? '#f87171' : '#fbbf24', fontSize: '13px', fontWeight: 'bold' }}>
                   {isSuccess ? '✅ SUCCESS' : isError ? '❌ RUNTIME ERROR' : '🛑 REJECTED'}
                 </div>
                 {isSuccess && <pre style={{ color: '#a7f3d0', fontSize: '12px', marginTop: '8px' }}>Stdout: {res.stdout}</pre>}
                 {isError && <pre style={{ color: '#fca5a5', fontSize: '12px', marginTop: '8px' }}>{res.error_telemetry?.message}</pre>}
                 {isRejected && <pre style={{ color: '#fcd34d', fontSize: '12px', marginTop: '8px' }}>Reason: {res.reason}</pre>}
                 
                 <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '8px', marginTop: '12px', paddingTop: '12px', borderTop: '1px solid rgba(255,255,255,0.1)' }}>
                   <div style={{ fontSize: '10px', color: '#94a3b8' }}>Clone Time<br/><b style={{ color: '#fff'}}>{res.metrics?.instance_clone_time_us}μs</b></div>
                   <div style={{ fontSize: '10px', color: '#94a3b8' }}>Exec Time<br/><b style={{ color: '#fff'}}>{res.metrics?.execution_time_us}μs</b></div>
                   <div style={{ fontSize: '10px', color: '#94a3b8' }}>Total Time<br/><b style={{ color: '#fff'}}>{res.metrics?.total_time_us}μs</b></div>
                   <div style={{ fontSize: '10px', color: '#94a3b8' }}>Fuel Consumed<br/><b style={{ color: '#fff'}}>{res.metrics?.fuel_consumed}</b></div>
                 </div>
               </div>
             );
          }

          if (ev.type === 'retry_feedback') {
             return (
               <div key={i} style={{ padding: '12px', border: '1px dashed #64748b', borderRadius: '6px', background: '#1e293b' }}>
                 <div style={{ color: '#fb923c', fontSize: '12px', fontWeight: 'bold', marginBottom: '8px' }}>🔄 Relaying Feedback to Groq...</div>
                 <pre style={{ margin: 0, color: '#cbd5e1', fontSize: '11px', whiteSpace: 'pre-wrap' }}>{ev.data.feedback}</pre>
               </div>
             );
          }

          if (ev.type === 'completed') {
             return (
               <div key={i} style={{ color: ev.data.reason === 'success' ? '#4ade80' : '#f87171', fontSize: '12px', fontWeight: 'bold' }}>
                 🏁 Loop Completed ({ev.data.reason})
               </div>
             );
          }

          if (ev.type === 'error') {
             return (
               <div key={i} style={{ color: '#f87171', fontSize: '12px', fontWeight: 'bold' }}>
                 ⚠️ Server Error: {ev.data.error}
               </div>
             );
          }

          return null;
        })}
        <div ref={logsEndRef} />
      </div>
      <style>{`
        @keyframes spin { 100% { transform: rotate(360deg); } }
      `}</style>
    </div>
  );
};
