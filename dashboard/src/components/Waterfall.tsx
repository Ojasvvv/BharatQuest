import React from 'react';
import type { StreamEvent } from '../hooks/useTelemetry';

interface WaterfallProps {
  events: StreamEvent[];
}

export const Waterfall: React.FC<WaterfallProps> = ({ events }) => {
  // Show up to 6 recent items
  const recentEvents = events.slice(0, 6);

  const RUNTIME_BADGE_STYLES: Record<string, { bg: string; text: string; label: string }> = {
    javascript: { bg: '#166534', text: '#4ade80', label: 'JS' },
    python:     { bg: '#1e3a5f', text: '#60a5fa', label: 'PY' },
  };

  return (
    <div className="panel">
      <div className="panel-head">
        <div>
          <div className="panel-title">Execution waterfall</div>
          <div className="panel-title-sub">Last 6 requests · proportional timing</div>
        </div>
        <div className="panel-badge">live</div>
      </div>
      <div className="panel-body">
        {recentEvents.length === 0 ? (
          <div style={{ padding: '20px 0', color: 'var(--text-tertiary)', fontSize: '11px', textAlign: 'center' }}>
            Waiting for data...
          </div>
        ) : (
          recentEvents.map((evt, idx) => {
            const { clone, eval: evalTime, marshal, total } = {
              clone: evt.metrics.instance_clone_time_us || 0,
              eval: evt.metrics.execution_time_us || 0,
              marshal: evt.metrics.memory_marshal_us || 0,
              total: evt.metrics.total_time_us || 1, // avoid div by 0
            };

            const clonePct = (clone / total) * 100;
            const evalPct = (evalTime / total) * 100;
            const marshalPct = (marshal / total) * 100;

            const isTrapped = evt.status === 'rejected';
            const isErr = evt.status === 'runtime_error';
            const isOk = evt.status === 'success';

            const badge = RUNTIME_BADGE_STYLES[evt.language] ?? { bg: '#374151', text: '#9ca3af', label: '??' };

            return (
              <div className="waterfall-row" key={evt.request_id + idx}>
                <div className="wf-id" style={{ display: 'flex', alignItems: 'center' }}>
                  <span style={{ 
                    background: badge.bg, 
                    color: badge.text,
                    padding: '1px 5px',
                    borderRadius: '3px',
                    fontSize: '10px',
                    fontFamily: 'monospace',
                    marginRight: '8px'
                  }}>
                    {badge.label}
                  </span>
                  {evt.request_id.split('-').pop()?.substring(0, 4) || 'req'}
                </div>
                <div className="wf-bar-track">
                  {isTrapped ? (
                    <div className="wf-seg-fail" style={{ width: '100%' }}></div>
                  ) : (
                    <>
                      <div className="wf-seg-clone" style={{ width: `${clonePct}%` }}></div>
                      <div className="wf-seg-eval" style={{ width: `${evalPct}%` }}></div>
                      <div className="wf-seg-marshal" style={{ width: `${marshalPct}%` }}></div>
                    </>
                  )}
                </div>
                <div className="wf-status">
                  {isTrapped && <div className="dot-rej"></div>}
                  {isErr && <div className="dot-err"></div>}
                  {isOk && <div className="dot-ok"></div>}
                </div>
                <div className="wf-total">{(total / 1000).toLocaleString(undefined, { maximumFractionDigits: 3 })}ms</div>
              </div>
            );
          })
        )}
      </div>
      <div className="legend">
        <div className="legend-item">
          <div className="legend-swatch" style={{ background: 'var(--blue)' }}></div>clone
        </div>
        <div className="legend-item">
          <div className="legend-swatch" style={{ background: 'var(--mint)' }}></div>eval
        </div>
        <div className="legend-item">
          <div className="legend-swatch" style={{ background: 'var(--purple)' }}></div>marshal
        </div>
        <div className="legend-item">
          <div className="legend-swatch" style={{ background: 'var(--red)', opacity: 0.7 }}></div>trapped / rejected
        </div>
      </div>
    </div>
  );
};
