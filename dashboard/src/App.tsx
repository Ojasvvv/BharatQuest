import { Sidebar } from './components/Sidebar';
import { TopBar } from './components/TopBar';
import { MetricRow } from './components/MetricRow';
import { Waterfall } from './components/Waterfall';
import { LatencyDistribution } from './components/LatencyDistribution';
import { LiveAgentDemo } from './components/LiveAgentDemo';
import { HowItWorks } from './components/HowItWorks';
import { useTelemetry } from './hooks/useTelemetry';
import { useState } from 'react';

import { useRuntimes } from './hooks/useRuntimes';

function App() {
  const { events, connectionStatus, lastMessageTime, stats } = useTelemetry('wss://bharatquest.onrender.com/v1/execute/stream');
  const { runtimes, loading: runtimesLoading, error: runtimesError } = useRuntimes();
  const [activeTab, setActiveTab] = useState('telemetry');

  return (
    <>
      <Sidebar activeTab={activeTab} setActiveTab={setActiveTab} runtimes={runtimes} loading={runtimesLoading} error={runtimesError} />
      <div className="main">
        <TopBar connectionStatus={connectionStatus} lastMessageTime={lastMessageTime} />
        <div className="content">
          {activeTab === 'telemetry' ? (
            <>
              <MetricRow stats={stats.p50} connectionStatus={connectionStatus} />
              <div className="grid-main">
                <Waterfall events={events} />
                <LatencyDistribution stats={stats.totalStats} />
              </div>
              <LiveAgentDemo />
            </>
          ) : activeTab === 'how-it-works' ? (
            <HowItWorks />
          ) : null}
        </div>
      </div>
    </>
  );
}

export default App;
