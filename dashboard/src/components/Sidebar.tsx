import React from 'react';

import { Runtime } from '../hooks/useRuntimes';

interface SidebarProps {
  activeTab: string;
  setActiveTab: (tab: string) => void;
  runtimes?: Runtime[];
  loading?: boolean;
  error?: string | null;
}

export const Sidebar: React.FC<SidebarProps> = ({ activeTab, setActiveTab, runtimes = [], loading, error }) => {
  return (
    <aside className="sidebar">
      <div className="sidebar-brand">
        <div className="brand-mark"></div>
        <div className="brand-name">Apatheia</div>
      </div>

      <div className="env-badge">
        <div className="env-dot"></div>
        <div className="env-text">localhost / dev</div>
      </div>
      <div className="px-3 py-2" style={{ padding: '8px 12px', borderBottom: '1px solid #1f2937' }}>
        <div className="text-xs text-gray-500 uppercase tracking-wider mb-1" style={{ fontSize: '10px', color: '#6b7280', textTransform: 'uppercase', letterSpacing: '0.05em', marginBottom: '8px' }}>
          Runtimes
        </div>
        {loading ? (
          <div className="text-xs text-gray-500" style={{ fontSize: '12px', color: '#6b7280' }}>Loading...</div>
        ) : error ? (
          <div className="text-xs text-red-400" style={{ fontSize: '12px', color: '#f87171' }}>Could not reach backend</div>
        ) : (
          runtimes.map(runtime => (
            <div key={runtime.id} className="flex items-center gap-2 py-0.5" style={{ display: 'flex', alignItems: 'center', gap: '8px', padding: '2px 0' }}>
              <span style={{
                width: 6, height: 6, borderRadius: '50%',
                background: runtime.status === 'ready' ? '#4ade80' : '#ef4444',
                display: 'inline-block',
                animation: runtime.status === 'ready' ? 'pulse 2s infinite' : 'none'
              }} />
              <span className="text-xs text-gray-300 font-mono" style={{ fontSize: '11px', color: '#d1d5db', fontFamily: 'monospace' }}>
                {runtime.label}
              </span>
              <span className={`text-xs ml-auto ${
                runtime.status === 'ready' ? 'text-green-500' : 'text-red-400'
              }`} style={{ fontSize: '11px', marginLeft: 'auto', color: runtime.status === 'ready' ? '#22c55e' : '#f87171' }}>
                {runtime.status}
              </span>
            </div>
          ))
        )}
      </div>

      <nav className="nav-section">
        <div className="nav-label">Monitoring</div>
        <div 
          className={`nav-item ${activeTab === 'telemetry' ? 'active' : ''}`}
          onClick={() => setActiveTab('telemetry')}
        >
          <svg className="nav-icon" viewBox="0 0 14 14" fill="none" xmlns="http://www.w3.org/2000/svg">
            <rect x="1" y="7" width="3" height="6" rx="0.5" fill="currentColor" />
            <rect x="5.5" y="4" width="3" height="9" rx="0.5" fill="currentColor" />
            <rect x="10" y="1" width="3" height="12" rx="0.5" fill="currentColor" />
          </svg>
          Telemetry
        </div>
        <div 
          className={`nav-item ${activeTab === 'how-it-works' ? 'active' : ''}`}
          onClick={() => setActiveTab('how-it-works')}
        >
          <svg className="nav-icon" viewBox="0 0 14 14" fill="none" xmlns="http://www.w3.org/2000/svg">
            <circle cx="7" cy="7" r="5.5" stroke="currentColor" strokeWidth="1.2" />
            <path d="M7 4v3" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
            <circle cx="7" cy="10" r="0.5" fill="currentColor" stroke="currentColor" />
          </svg>
          How It Works
        </div>
      </nav>

      <nav className="nav-section">
        <div className="nav-label">Connect</div>
        <a href="https://github.com/Ojasvvv/BharatQuest" target="_blank" rel="noreferrer" className="nav-item" style={{textDecoration: 'none'}}>
          <svg className="nav-icon" viewBox="0 0 24 24" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
            <path d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12"/>
          </svg>
          GitHub
        </a>
        <a href="https://linkedin.com/in/ojasvkushwah" target="_blank" rel="noreferrer" className="nav-item" style={{textDecoration: 'none'}}>
          <svg className="nav-icon" viewBox="0 0 24 24" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
            <path d="M20.447 20.452h-3.554v-5.569c0-1.328-.027-3.037-1.852-3.037-1.853 0-2.136 1.445-2.136 2.939v5.667H9.351V9h3.414v1.561h.046c.477-.9 1.637-1.85 3.37-1.85 3.601 0 4.267 2.37 4.267 5.455v6.286zM5.337 7.433c-1.144 0-2.063-.926-2.063-2.065 0-1.138.92-2.063 2.063-2.063 1.14 0 2.064.925 2.064 2.063 0 1.139-.925 2.065-2.064 2.065zm1.782 13.019H3.555V9h3.564v11.452zM22.225 0H1.771C.792 0 0 .774 0 1.729v20.542C0 23.227.792 24 1.771 24h20.451C23.2 24 24 23.227 24 22.271V1.729C24 .774 23.2 0 22.222 0h.003z"/>
          </svg>
          LinkedIn
        </a>
      </nav>

      <div className="sidebar-vitals">
        <div className="vitals-label">Sandbox Configuration</div>
        <div className="vital-row">
          <div className="vital-top">
            <span className="vital-name">Fuel Limit</span>
            <span className="vital-val">50M opcodes</span>
          </div>
          <div className="vital-track">
            <div className="vital-fill" style={{ width: '100%', opacity: 0.2 }}></div>
          </div>
        </div>
        <div className="vital-row">
          <div className="vital-top">
            <span className="vital-name">Wall Clock</span>
            <span className="vital-val">5,000ms</span>
          </div>
          <div className="vital-track">
            <div className="vital-fill med" style={{ width: '100%', opacity: 0.2 }}></div>
          </div>
        </div>
        <div className="vital-row">
          <div className="vital-top">
            <span className="vital-name">Memory Cap</span>
            <span className="vital-val">256MB</span>
          </div>
          <div className="vital-track">
            <div className="vital-fill hi" style={{ width: '100%', opacity: 0.2 }}></div>
          </div>
        </div>
      </div>
    </aside>
  );
};
