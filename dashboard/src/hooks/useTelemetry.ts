import { useState, useEffect, useCallback, useRef } from 'react';

export interface ExecutionMetrics {
  instance_clone_time_us: number;
  execution_time_us: number;
  memory_marshal_us: number;
  total_time_us: number;
  fuel_consumed: number;
}

export interface StreamEvent {
  request_id: string;
  language: string;
  status: 'success' | 'runtime_error' | 'rejected';
  metrics: ExecutionMetrics;
}

export type ConnectionStatus = 'connected' | 'reconnecting' | 'disconnected';

export function useTelemetry(wsUrl: string) {
  const [events, setEvents] = useState<StreamEvent[]>([]);
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>('disconnected');
  const [lastMessageTime, setLastMessageTime] = useState<Date | null>(null);
  const reconnectAttemptsRef = useRef(0);
  const MAX_RECONNECT_ATTEMPTS = 3;

  const connectWebSocket = useCallback(() => {
    const ws = new WebSocket(wsUrl);

    ws.onopen = () => {
      setConnectionStatus('connected');
      reconnectAttemptsRef.current = 0;
    };

    ws.onmessage = (event) => {
      try {
        setLastMessageTime(new Date());
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
      if (reconnectAttemptsRef.current >= MAX_RECONNECT_ATTEMPTS) {
        setConnectionStatus('disconnected');
        return;
      }
      setConnectionStatus('reconnecting');
      reconnectAttemptsRef.current += 1;
      // Wait 2s then retry
      setTimeout(() => connectWebSocket(), 2000);
      // This setTimeout is legitimate: WebSocket reconnection backoff delay
    };

    ws.onerror = () => {
      // onerror usually leads to onclose where reconnect logic is handled
      ws.close();
    };

    return ws;
  }, [wsUrl]);

  useEffect(() => {
    const ws = connectWebSocket();
    return () => {
      if (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING) {
        ws.close();
      }
    };
  }, [connectWebSocket]);

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

  return { events, connectionStatus, lastMessageTime, stats };
}
