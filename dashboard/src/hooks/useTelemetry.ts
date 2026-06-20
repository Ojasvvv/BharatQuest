import { useState, useEffect, useCallback } from 'react';

export interface ExecutionMetrics {
  instance_clone_time_us: number;
  execution_time_us: number;
  memory_marshal_us: number;
  total_time_us: number;
  fuel_consumed: number;
}

export interface StreamEvent {
  request_id: string;
  status: 'success' | 'runtime_error' | 'rejected';
  metrics: ExecutionMetrics;
}

export function useTelemetry(wsUrl: string) {
  const [events, setEvents] = useState<StreamEvent[]>([]);
  const [isConnected, setIsConnected] = useState(false);

  useEffect(() => {
    let ws: WebSocket;
    let reconnectTimer: number;

    const connect = () => {
      ws = new WebSocket(wsUrl);

      ws.onopen = () => {
        setIsConnected(true);
      };

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data) as StreamEvent;
          setEvents((prev) => {
            const newEvents = [data, ...prev];
            // Keep last 200 events
            if (newEvents.length > 200) {
              return newEvents.slice(0, 200);
            }
            return newEvents;
          });
        } catch (e) {
          console.error("Failed to parse websocket message", e);
        }
      };

      ws.onclose = () => {
        setIsConnected(false);
        // Reconnect after 2 seconds
        reconnectTimer = setTimeout(connect, 2000) as unknown as number;
      };

      ws.onerror = () => {
        ws.close();
      };
    };

    connect();

    return () => {
      clearTimeout(reconnectTimer);
      if (ws) {
        ws.close();
      }
    };
  }, [wsUrl]);

  // Compute percentiles for last 200 runs
  const getPercentile = useCallback((metric: keyof ExecutionMetrics, p: number) => {
    if (events.length === 0) return 0;
    const sorted = [...events].map(e => e.metrics[metric]).sort((a, b) => a - b);
    const index = Math.ceil((p / 100) * sorted.length) - 1;
    return sorted[Math.max(0, Math.min(sorted.length - 1, index))];
  }, [events]);

  const stats = {
    p50: {
      clone: getPercentile('instance_clone_time_us', 50),
      eval: getPercentile('execution_time_us', 50),
      marshal: getPercentile('memory_marshal_us', 50),
      total: getPercentile('total_time_us', 50),
    },
    totalStats: {
      p50: getPercentile('total_time_us', 50),
      p90: getPercentile('total_time_us', 90),
      p99: getPercentile('total_time_us', 99),
      max: getPercentile('total_time_us', 100),
    }
  };

  return { events, isConnected, stats };
}
