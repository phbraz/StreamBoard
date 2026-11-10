import { useCallback, useEffect, useRef, useState } from 'react'
import type { ClientMessage, ServerMessage, ServerState } from '../protocol'

export type ConnectionStatus = 'connecting' | 'open' | 'closed'

type Ack = Extract<ServerMessage, { type: 'ack' }>
type Request = Exclude<ClientMessage, { type: 'hello' } | { type: 'ping' }>
type RequestBody = Request extends infer R ? (R extends Request ? Omit<R, 'requestId'> : never) : never

const MAX_BACKOFF_MS = 10_000
const ACK_TIMEOUT_MS = 5_000

function wsUrl() {
  const proto = location.protocol === 'https:' ? 'wss:' : 'ws:'
  return `${proto}//${location.host}/ws`
}

export function useSoundboard(token: string) {
  const [status, setStatus] = useState<ConnectionStatus>('connecting')
  const [state, setState] = useState<ServerState>({ micMuted: false, playing: [] })
  const [deviceId, setDeviceId] = useState<string | null>(null)
  // Bumped whenever the board should be refetched (connect, or another device edited it).
  const [boardVersion, setBoardVersion] = useState(0)
  // When the newest playing instance of each sound started (client clock), for progress bars.
  const [startedAt, setStartedAt] = useState<Record<string, number>>({})
  const playCounts = useRef(new Map<string, number>())

  const socket = useRef<WebSocket | null>(null)
  const pending = useRef(new Map<string, (ack: Ack) => void>())
  const nextId = useRef(0)

  useEffect(() => {
    let backoff = 500
    let retry: ReturnType<typeof setTimeout> | undefined
    let disposed = false

    const applyState = (next: ServerState) => {
      const counts = new Map<string, number>()
      for (const p of next.playing) counts.set(p.soundId, (counts.get(p.soundId) ?? 0) + 1)
      const prevCounts = playCounts.current
      playCounts.current = counts
      const now = Date.now()
      setStartedAt((prev) => {
        const out: Record<string, number> = {}
        for (const [id, n] of counts) {
          out[id] = n > (prevCounts.get(id) ?? 0) || !(id in prev) ? now : prev[id]
        }
        return out
      })
      setState(next)
    }

    const connect = () => {
      setStatus('connecting')
      const ws = new WebSocket(wsUrl())
      socket.current = ws

      ws.onopen = () => {
        backoff = 500
        ws.send(JSON.stringify({ type: 'hello', token } satisfies ClientMessage))
      }

      ws.onmessage = (event) => {
        const msg = JSON.parse(event.data) as ServerMessage
        switch (msg.type) {
          case 'welcome':
            setDeviceId(msg.deviceId)
            applyState(msg.state)
            setStatus('open')
            setBoardVersion((v) => v + 1)
            break
          case 'boardUpdated':
            setBoardVersion((v) => v + 1)
            break
          case 'state':
            applyState(msg.state)
            break
          case 'ack':
            pending.current.get(msg.requestId)?.(msg)
            pending.current.delete(msg.requestId)
            break
          case 'error':
            console.warn('server error:', msg.message)
            break
        }
      }

      ws.onclose = () => {
        socket.current = null
        setStatus('closed')
        for (const resolve of pending.current.values()) {
          resolve({ type: 'ack', requestId: '', ok: false, error: 'disconnected' })
        }
        pending.current.clear()
        if (!disposed) {
          retry = setTimeout(connect, backoff)
          backoff = Math.min(backoff * 2, MAX_BACKOFF_MS)
        }
      }
    }

    connect()
    return () => {
      disposed = true
      clearTimeout(retry)
      const ws = socket.current
      socket.current = null
      if (ws) {
        // The close event fires later; it must not clobber the next effect's socket.
        ws.onopen = ws.onmessage = ws.onclose = null
        ws.close()
      }
    }
  }, [token])

  const request = useCallback((body: RequestBody): Promise<Ack> => {
    const ws = socket.current
    const requestId = `r${++nextId.current}`
    if (!ws || ws.readyState !== WebSocket.OPEN) {
      return Promise.resolve({ type: 'ack', requestId, ok: false, error: 'not connected' })
    }
    return new Promise((resolve) => {
      const timer = setTimeout(() => {
        pending.current.delete(requestId)
        resolve({ type: 'ack', requestId, ok: false, error: 'timed out' })
      }, ACK_TIMEOUT_MS)
      pending.current.set(requestId, (ack) => {
        clearTimeout(timer)
        resolve(ack)
      })
      ws.send(JSON.stringify({ ...body, requestId }))
    })
  }, [])

  return {
    status,
    state,
    deviceId,
    boardVersion,
    startedAt,
    press: (buttonId: string) => request({ type: 'press', buttonId }),
    play: (soundId: string) => request({ type: 'play', soundId }),
    stop: (soundId: string) => request({ type: 'stop', soundId }),
    stopAll: () => request({ type: 'stopAll' }),
    runAction: (actionId: string) => request({ type: 'runAction', actionId }),
  }
}
