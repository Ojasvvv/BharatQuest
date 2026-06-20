import React from 'react';

interface LatencyDistributionProps {
  stats: {
    p50: number;
    p90: number;
    p99: number;
    max: number;
  };
}

export const LatencyDistribution: React.FC<LatencyDistributionProps> = ({ stats }) => {
  // Use a sensible upper bound for the scale, e.g. 10000us
  const MAX_SCALE = Math.max(stats.max || 1000, 5000);

  const getWidth = (val: number) => {
    if (!val) return '0%';
    return Math.min(100, Math.max(1, (val / MAX_SCALE) * 100)) + '%';
  };

  return (
    <div className="panel">
      <div className="panel-head">
        <div>
          <div className="panel-title">Latency distribution</div>
          <div className="panel-title-sub">Total request time · n=200</div>
        </div>
      </div>
      <div className="panel-body">
        <div className="pctl-list">
          <div className="pctl-row">
            <div className="pctl-left">
              <div className="pctl-label">p50</div>
              <div className="pctl-track">
                <div className="pctl-fill" style={{ width: getWidth(stats.p50) }}></div>
              </div>
            </div>
            <div className="pctl-value">{stats.p50 ? stats.p50.toLocaleString() : '-'}<span className="pctl-unit">µs</span></div>
          </div>
          <div className="pctl-row">
            <div className="pctl-left">
              <div className="pctl-label">p90</div>
              <div className="pctl-track">
                <div className="pctl-fill" style={{ width: getWidth(stats.p90) }}></div>
              </div>
            </div>
            <div className="pctl-value">{stats.p90 ? stats.p90.toLocaleString() : '-'}<span className="pctl-unit">µs</span></div>
          </div>
          <div className="pctl-row">
            <div className="pctl-left">
              <div className="pctl-label">p99</div>
              <div className="pctl-track">
                <div className="pctl-fill warn" style={{ width: getWidth(stats.p99) }}></div>
              </div>
            </div>
            <div className="pctl-value">{stats.p99 ? stats.p99.toLocaleString() : '-'}<span className="pctl-unit">µs</span></div>
          </div>
          <div className="pctl-row">
            <div className="pctl-left">
              <div className="pctl-label">max</div>
              <div className="pctl-track">
                <div className="pctl-fill crit" style={{ width: getWidth(stats.max) }}></div>
              </div>
            </div>
            <div className="pctl-value">{stats.max ? stats.max.toLocaleString() : '-'}<span className="pctl-unit">µs</span></div>
          </div>
        </div>
        <div className="pctl-spark-area">
          <div className="pctl-spark-label">Request latency · live sparkline placeholder</div>
          <svg className="spark-svg" height="38" viewBox="0 0 240 38" xmlns="http://www.w3.org/2000/svg" preserveAspectRatio="none">
            <defs>
              <linearGradient id="sg" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor="#5FE3A8" stopOpacity="0.15" />
                <stop offset="100%" stopColor="#5FE3A8" stopOpacity="0" />
              </linearGradient>
            </defs>
            <path d="M0,32 L8,29 L16,31 L24,26 L32,33 L40,28 L48,30 L56,27 L64,34 L72,12 L80,29 L88,31 L96,26 L104,28 L112,30 L120,25 L128,27 L136,31 L144,8 L152,28 L160,30 L168,26 L176,31 L184,28 L192,29 L200,26 L208,30 L216,27 L224,29 L232,28 L240,26 L240,38 L0,38 Z" fill="url(#sg)" />
            <path d="M0,32 L8,29 L16,31 L24,26 L32,33 L40,28 L48,30 L56,27 L64,34 L72,12 L80,29 L88,31 L96,26 L104,28 L112,30 L120,25 L128,27 L136,31 L144,8 L152,28 L160,30 L168,26 L176,31 L184,28 L192,29 L200,26 L208,30 L216,27 L224,29 L232,28 L240,26" fill="none" stroke="#5FE3A8" strokeWidth="1" strokeOpacity="0.45" />
            <circle cx="72" cy="12" r="2" fill="#D4974A" opacity="0.8" />
            <circle cx="144" cy="8" r="2" fill="#E0625B" opacity="0.8" />
          </svg>
        </div>
      </div>
    </div>
  );
};
