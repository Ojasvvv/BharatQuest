import { useState, useCallback, useEffect } from 'react';

export interface Runtime {
  id: string
  label: string
  status: 'ready' | 'unavailable'
  wasm_binary: string
  runtime_notes?: string
}

export interface UseRuntimesResult {
  runtimes: Runtime[]
  loading: boolean
  error: string | null
  refetch: () => void
}

export function useRuntimes(): UseRuntimesResult {
  const [runtimes, setRuntimes] = useState<Runtime[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const fetch_runtimes = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const res = await fetch('/v1/runtimes')
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
      const data = await res.json()
      setRuntimes(data.runtimes)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to fetch runtimes')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { fetch_runtimes() }, [fetch_runtimes])

  // Poll every 30 seconds for status changes
  useEffect(() => {
    const interval = setInterval(fetch_runtimes, 30_000)
    return () => clearInterval(interval)
    // This setInterval is legitimate: polling /v1/runtimes for status
    // changes (e.g. a runtime becoming unavailable). 30s is appropriate.
  }, [fetch_runtimes])

  return { runtimes, loading, error, refetch: fetch_runtimes }
}
