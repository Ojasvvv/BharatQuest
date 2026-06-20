import React from 'react';

interface MetricRowProps {
  stats: {
    clone: number;
    eval: number;
    marshal: number;
    total: number;
  };
}

export const MetricRow: React.FC<MetricRowProps> = ({ stats }) => {
  // Max possible times to calculate proportional bar width
  const MAX_CLONE = 1000;
  const MAX_EVAL = 5000;
  const MAX_MARSHAL = 500;
  
  const getWidth = (val: number, max: number) => {
    return Math.min(100, Math.max(1, (val / max) * 100)) + '%';
  };

  return (
    <div className="metric-row">
      <div className="metric">
        <div className="metric-label">Instance clone</div>
        <div className="metric-value-row">
          <div className="metric-value">{stats.clone || '-'}</div>
          <div className="metric-unit">µs</div>
        </div>
        <div className="metric-bar">
          <div className="metric-bar-fill" style={{ width: stats.clone ? getWidth(stats.clone, MAX_CLONE) : '0%' }}></div>
        </div>
        <div className="metric-delta">p50 · last 200 runs</div>
      </div>
      
      <div className="metric">
        <div className="metric-label">JS evaluation</div>
        <div className="metric-value-row">
          <div className="metric-value">{stats.eval || '-'}</div>
          <div className="metric-unit">µs</div>
        </div>
        <div className="metric-bar">
          <div className="metric-bar-fill" style={{ width: stats.eval ? getWidth(stats.eval, MAX_EVAL) : '0%' }}></div>
        </div>
        <div className="metric-delta">p50 · last 200 runs</div>
      </div>
      
      <div className="metric">
        <div className="metric-label">Memory marshal</div>
        <div className="metric-value-row">
          <div className="metric-value">{stats.marshal || '-'}</div>
          <div className="metric-unit">µs</div>
        </div>
        <div className="metric-bar">
          <div className="metric-bar-fill" style={{ width: stats.marshal ? getWidth(stats.marshal, MAX_MARSHAL) : '0%' }}></div>
        </div>
        <div className="metric-delta">p50 · last 200 runs</div>
      </div>
      
      <div className="metric">
        <div className="metric-label">Total request</div>
        <div className="metric-value-row">
          <div className="metric-value">{stats.total || '-'}</div>
          <div className="metric-unit">µs</div>
        </div>
        <div className="metric-bar" style={{ background: 'var(--mint-dim)' }}>
          <div className="metric-bar-fill" style={{ width: '100%', background: 'var(--mint)', opacity: 1 }}></div>
        </div>
        <div className="metric-delta highlight">Live metrics derived from stream</div>
      </div>
    </div>
  );
};
