import React, { useEffect, useState } from 'react';

import type { ConnectionStatus } from '../hooks/useTelemetry';

interface TopBarProps {
  connectionStatus: ConnectionStatus;
  lastMessageTime: Date | null;
}

export const TopBar: React.FC<TopBarProps> = ({ connectionStatus, lastMessageTime }) => {
  const [timeStr, setTimeStr] = useState('');

  useEffect(() => {
    const update = () => {
      const now = new Date();
      setTimeStr(
        now.toISOString().split('T')[0] +
          ' · ' +
          now.toISOString().split('T')[1].substring(0, 8) +
          ' UTC'
      );
    };
    update();
    const interval = setInterval(update, 1000); // Legitimate setInterval: update top bar clock every second
    return () => clearInterval(interval);
  }, []);

  const STATUS_CONFIG = {
    connected:    { color: '#4ade80', label: 'Connected',     pulse: false },
    reconnecting: { color: '#fbbf24', label: 'Reconnecting...', pulse: true },
    disconnected: { color: '#ef4444', label: 'Disconnected',  pulse: false },
  };

  const status = STATUS_CONFIG[connectionStatus];

  return (
    <>
      <div className="topbar">
        <div className="topbar-title">Sandbox Telemetry</div>
        <div className="topbar-right">
          <div className="topbar-ts">{timeStr}</div>
          <div className="flex items-center gap-1.5" style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
            <span style={{
              width: 8, height: 8, borderRadius: '50%',
              background: status.color,
              display: 'inline-block',
              animation: status.pulse ? 'pulse 1s infinite' : 'none'
            }} />
            <span className="text-xs" style={{ color: status.color, fontSize: '12px' }}>
              {status.label}
            </span>
          </div>
        </div>
      </div>
      {connectionStatus === 'disconnected' && lastMessageTime && (
        <div style={{
          backgroundColor: 'rgba(113, 63, 18, 0.3)',
          borderBottom: '1px solid rgba(161, 98, 7, 0.5)',
          padding: '4px 16px',
          fontSize: '12px',
          color: '#facc15'
        }}>
          Live data paused — last updated {lastMessageTime.toLocaleTimeString()}
        </div>
      )}
    </>
  );
};
