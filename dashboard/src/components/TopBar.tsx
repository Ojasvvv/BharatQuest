import React, { useEffect, useState } from 'react';

interface TopBarProps {
  isConnected: boolean;
}

export const TopBar: React.FC<TopBarProps> = ({ isConnected }) => {
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
    const interval = setInterval(update, 1000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="topbar">
      <div className="topbar-title">Sandbox Telemetry</div>
      <div className="topbar-right">
        <div className="topbar-ts">{timeStr}</div>
        <div className="live-pill">
          {isConnected && <div className="live-dot"></div>}
          {isConnected ? 'Connected' : 'Connecting...'}
        </div>
      </div>
    </div>
  );
};
