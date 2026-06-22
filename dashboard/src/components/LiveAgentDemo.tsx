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
  const scrollContainerRef = useRef<HTMLDivElement>(null);

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
    if (scrollContainerRef.current && events.length > 0) {
      scrollContainerRef.current.scrollTop = scrollContainerRef.current.scrollHeight;
    }
  }, [events]);

  return (
    <div className="sim-panel" style={{ marginTop: '16px' }}>
      <div className="panel-head" style={{ display: 'flex', flexWrap: 'wrap', gap: '16px', justifyContent: 'space-between', alignItems: 'center' }}>
        <div>
          <div className="panel-title">Live Agent Demo</div>
          <div className="panel-title-sub">Real-time LLM interaction loop</div>
        </div>
        <div style={{ display: 'flex', gap: '12px', alignItems: 'center', width: '100%', maxWidth: '350px' }}>
          <select 
            value={mode} 
            onChange={(e) => setMode(e.target.value as any)}
            disabled={isRunning}
            style={{ 
              background: 'var(--surface-2)', 
              color: 'var(--text)', 
              border: '1px solid var(--border)', 
              borderRadius: '6px', 
              padding: '6px 10px', 
              fontSize: '12px',
              flex: 1
            }}
          >
            <option value="realistic_bug">Mode: Realistic Bug (Self-Heal)</option>
            <option value="dangerous">Mode: Dangerous Code (Trap)</option>
          </select>
          <button 
            onClick={isRunning ? stopLoop : startLoop}
            style={{ 
              fontSize: '12px', 
              padding: '6px 16px', 
              borderRadius: '6px', 
              background: isRunning ? 'var(--red-dim)' : 'var(--mint-dim)', 
              color: isRunning ? 'var(--red)' : 'var(--mint)', 
              border: `1px solid ${isRunning ? 'rgba(224,98,91,0.3)' : 'rgba(95,227,168,0.3)'}`,
              cursor: 'pointer',
              fontWeight: 600,
              flexShrink: 0
            }}
          >
            {isRunning ? 'Stop' : 'Start'}
          </button>
        </div>
      </div>
      
      <div ref={scrollContainerRef} style={{ padding: '24px', position: 'relative', display: 'flex', flexDirection: 'column', gap: '0', maxHeight: '500px', overflowY: 'auto', background: 'var(--surface)', borderTop: '1px solid var(--border)' }}>
        {events.length === 0 && !isRunning && (
           <div style={{ color: 'var(--text-tertiary)', fontSize: '13px', textAlign: 'center', padding: '40px 20px' }}>
             Click "Start" to begin the live execution timeline.
           </div>
        )}
        
        {/* Timeline track */}
        {events.length > 0 && (
          <div style={{ position: 'absolute', left: '43px', top: '24px', bottom: '24px', width: '2px', background: 'var(--border)' }} />
        )}

        {events.map((ev, i) => {
          
          const TimelineItem = ({ icon, color, bg, children }: { icon: React.ReactNode, color: string, bg: string, children: React.ReactNode }) => (
            <div key={i} style={{ display: 'flex', gap: '16px', position: 'relative', marginBottom: '20px' }}>
              <div style={{ 
                width: '40px', height: '40px', borderRadius: '50%', background: bg, border: `1px solid ${color}`, 
                display: 'flex', alignItems: 'center', justifyContent: 'center', fontSize: '16px', flexShrink: 0, zIndex: 1, boxShadow: '0 0 0 4px var(--surface)'
              }}>
                {icon}
              </div>
              <div style={{ flex: 1, minWidth: 0, marginTop: '2px' }}>
                {children}
              </div>
            </div>
          );

          if (ev.type === 'start') {
             return (
               <TimelineItem icon="📋" color="rgba(58,125,217,0.3)" bg="var(--surface-2)">
                 <div style={{ padding: '12px 16px', border: '1px solid var(--border)', borderRadius: '8px', background: 'var(--surface-2)' }}>
                   <div style={{ color: 'var(--blue)', fontSize: '11px', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.05em' }}>Task Goal</div>
                   <div style={{ color: 'var(--text)', fontSize: '13px', marginTop: '6px', lineHeight: 1.5 }}>{ev.data.task}</div>
                 </div>
               </TimelineItem>
             );
          }

          if (ev.type === 'llm_generating') {
             const isLatest = i === events.length - 1 && isRunning;
             const icon = isLatest ? (
                <div className="spinner" style={{ width: '16px', height: '16px', border: '2px solid var(--amber)', borderTopColor: 'transparent', borderRadius: '50%', animation: 'spin 1s linear infinite' }} />
             ) : '🧠';
             return (
               <TimelineItem icon={icon} color={isLatest ? "rgba(212,151,74,0.5)" : "transparent"} bg={isLatest ? "var(--surface-2)" : "transparent"}>
                 <div style={{ color: 'var(--amber)', fontSize: '12px', fontWeight: 500, padding: '12px 0' }}>
                   Iteration {ev.data.iteration}: Requesting code from Groq (Llama 3.3)...
                 </div>
               </TimelineItem>
             );
          }

          if (ev.type === 'code_generated') {
             return (
               <TimelineItem icon="📝" color="rgba(255,255,255,0.1)" bg="var(--surface-3)">
                 <div style={{ padding: '12px 16px', border: '1px solid var(--border-strong)', borderRadius: '8px', background: '#0A0B0D' }}>
                   <div style={{ color: 'var(--text-tertiary)', fontSize: '10px', textTransform: 'uppercase', letterSpacing: '0.05em', marginBottom: '8px' }}>Generated Code</div>
                   <pre style={{ margin: 0, color: 'var(--mint)', fontSize: '12px', whiteSpace: 'pre-wrap', wordBreak: 'break-word', overflowX: 'auto', fontFamily: "'JetBrains Mono', monospace" }}>{ev.data.code}</pre>
                 </div>
               </TimelineItem>
             );
          }

          if (ev.type === 'apatheia_evaluating') {
             return (
               <TimelineItem icon="⚡" color="transparent" bg="transparent">
                 <div style={{ color: 'var(--purple)', fontSize: '12px', fontWeight: 500, padding: '12px 0' }}>
                   Sending to Apatheia WASM Engine...
                 </div>
               </TimelineItem>
             );
          }

          if (ev.type === 'apatheia_result') {
             const res = ev.data.result;
             const isSuccess = res.status === 'success';
             const isError = res.status === 'runtime_error';
             const isRejected = res.status === 'rejected';
             
             const borderColor = isSuccess ? 'var(--mint)' : isError ? 'var(--red)' : 'var(--amber)';
             const bgColor = isSuccess ? 'var(--mint-dim)' : isError ? 'var(--red-dim)' : 'rgba(212,151,74,0.15)';
             const icon = isSuccess ? '✅' : isError ? '❌' : '🛑';

             return (
               <TimelineItem icon={icon} color={`rgba(${isSuccess ? '95,227,168' : isError ? '224,98,91' : '212,151,74'}, 0.3)`} bg={bgColor}>
                 <div style={{ padding: '16px', border: `1px solid rgba(${isSuccess ? '95,227,168' : isError ? '224,98,91' : '212,151,74'}, 0.2)`, borderRadius: '8px', background: bgColor }}>
                   <div style={{ color: borderColor, fontSize: '11px', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
                     {isSuccess ? 'Execution Success' : isError ? 'Runtime Error' : 'Execution Rejected'}
                   </div>
                   
                   {isSuccess && <pre style={{ color: 'var(--text)', fontSize: '12px', marginTop: '8px', whiteSpace: 'pre-wrap', wordBreak: 'break-word', overflowX: 'auto', fontFamily: "'JetBrains Mono', monospace" }}>Stdout: {res.stdout}</pre>}
                   {isError && <pre style={{ color: 'var(--red)', fontSize: '12px', marginTop: '8px', whiteSpace: 'pre-wrap', wordBreak: 'break-word', overflowX: 'auto', fontFamily: "'JetBrains Mono', monospace" }}>{res.error_telemetry?.message}</pre>}
                   {isRejected && <pre style={{ color: 'var(--amber)', fontSize: '12px', marginTop: '8px', whiteSpace: 'pre-wrap', wordBreak: 'break-word', overflowX: 'auto', fontFamily: "'JetBrains Mono', monospace" }}>Reason: {res.reason}</pre>}
                   
                   <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(80px, 1fr))', gap: '8px', marginTop: '16px', paddingTop: '12px', borderTop: `1px solid rgba(${isSuccess ? '95,227,168' : isError ? '224,98,91' : '212,151,74'}, 0.2)` }}>
                     <div style={{ fontSize: '10px', color: 'var(--text-secondary)' }}>Clone<br/><b style={{ color: 'var(--text)', fontSize: '12px', fontFamily: "'JetBrains Mono', monospace" }}>{res.metrics?.instance_clone_time_us}μs</b></div>
                     <div style={{ fontSize: '10px', color: 'var(--text-secondary)' }}>Exec<br/><b style={{ color: 'var(--text)', fontSize: '12px', fontFamily: "'JetBrains Mono', monospace" }}>{res.metrics?.execution_time_us}μs</b></div>
                     <div style={{ fontSize: '10px', color: 'var(--text-secondary)' }}>Total<br/><b style={{ color: 'var(--text)', fontSize: '12px', fontFamily: "'JetBrains Mono', monospace" }}>{res.metrics?.total_time_us}μs</b></div>
                     <div style={{ fontSize: '10px', color: 'var(--text-secondary)' }}>Fuel<br/><b style={{ color: 'var(--text)', fontSize: '12px', fontFamily: "'JetBrains Mono', monospace" }}>{res.metrics?.fuel_consumed}</b></div>
                   </div>
                 </div>
               </TimelineItem>
             );
          }

          if (ev.type === 'retry_feedback') {
             return (
               <TimelineItem icon="🔄" color="rgba(58,125,217,0.3)" bg="var(--surface-2)">
                 <div style={{ padding: '12px 16px', border: '1px dashed var(--border-strong)', borderRadius: '8px', background: 'var(--surface-2)' }}>
                   <div style={{ color: 'var(--blue)', fontSize: '11px', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.05em', marginBottom: '6px' }}>Relaying Feedback</div>
                   <pre style={{ margin: 0, color: 'var(--text-secondary)', fontSize: '11px', whiteSpace: 'pre-wrap', wordBreak: 'break-word', overflowX: 'auto', fontFamily: "'JetBrains Mono', monospace" }}>{ev.data.feedback}</pre>
                 </div>
               </TimelineItem>
             );
          }

          if (ev.type === 'completed') {
             const isSuccess = ev.data.reason === 'success';
             return (
               <TimelineItem icon="🏁" color="transparent" bg="transparent">
                 <div style={{ color: isSuccess ? 'var(--mint)' : 'var(--red)', fontSize: '12px', fontWeight: 600, padding: '12px 0' }}>
                   Loop Completed ({ev.data.reason})
                 </div>
               </TimelineItem>
             );
          }

          if (ev.type === 'error') {
             return (
               <TimelineItem icon="⚠️" color="transparent" bg="transparent">
                 <div style={{ color: 'var(--red)', fontSize: '12px', fontWeight: 600, padding: '12px 0' }}>
                   Server Error: {ev.data.error}
                 </div>
               </TimelineItem>
             );
          }

          return null;
        })}
        <div />
      </div>
      <style>{`
        @keyframes spin { 100% { transform: rotate(360deg); } }
      `}</style>
    </div>
  );
};
