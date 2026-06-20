import React from 'react';
import { Sidebar } from './components/Sidebar';
import { TopBar } from './components/TopBar';
import { MetricRow } from './components/MetricRow';
import { Waterfall } from './components/Waterfall';
import { LatencyDistribution } from './components/LatencyDistribution';
import { AgentSimulator } from './components/AgentSimulator';
import { HowItWorks } from './components/HowItWorks';
import { useTelemetry } from './hooks/useTelemetry';
import { useState } from 'react';

function App() {
  const { events, isConnected, stats } = useTelemetry('ws://127.0.0.1:3000/v1/execute/stream');
  const [activeTab, setActiveTab] = useState('telemetry');

  return (
    <>
      <Sidebar activeTab={activeTab} setActiveTab={setActiveTab} />
      <div className="main">
        <TopBar isConnected={isConnected} />
        <div className="content">
          {activeTab === 'telemetry' ? (
            <>
              <MetricRow stats={stats.p50} />
              <div className="grid-main">
                <Waterfall events={events} />
                <LatencyDistribution stats={stats.totalStats} />
              </div>
              <AgentSimulator />
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
