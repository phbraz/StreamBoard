import { useState } from 'react'
import { useSoundboard, type ConnectionStatus } from './hooks/useSoundboard'

// Placeholder board until per-device boards land (milestone 2).
const DEMO_BUTTONS = [
  { id: 'b1', label: 'Sound 1', kind: 'sound', targetId: 'demo-1' },
  { id: 'b2', label: 'Sound 2', kind: 'sound', targetId: 'demo-2' },
  { id: 'b3', label: 'Sound 3', kind: 'sound', targetId: 'demo-3' },
  { id: 'b4', label: 'Mic', kind: 'action', targetId: 'mic-toggle' },
  { id: 'b5', label: 'Action 2', kind: 'action', targetId: 'demo-action' },
] as const

const STATUS_COLOR: Record<ConnectionStatus, string> = {
  open: 'bg-ok',
  connecting: 'bg-warn',
  closed: 'bg-accent',
}

export default function App() {
  const sb = useSoundboard('dev')
  const [lastResult, setLastResult] = useState<string | null>(null)

  const press = async (button: (typeof DEMO_BUTTONS)[number]) => {
    const ack =
      button.kind === 'sound' ? await sb.play(button.targetId) : await sb.runAction(button.targetId)
    setLastResult(ack.ok ? `${button.label}: ok` : `${button.label}: ${ack.error ?? 'failed'}`)
  }

  return (
    <main className="mx-auto flex min-h-full max-w-3xl flex-col gap-6 p-4">
      <header className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">Soundboard</h1>
        <div className="flex items-center gap-2 text-sm text-board-muted">
          <span className={`size-2.5 rounded-full ${STATUS_COLOR[sb.status]}`} />
          {sb.status}
          {sb.deviceId && <span>· {sb.deviceId}</span>}
          <span>· mic {sb.state.micMuted ? 'muted' : 'live'}</span>
        </div>
      </header>

      <section className="grid grid-cols-3 gap-board-gap sm:grid-cols-4">
        {DEMO_BUTTONS.map((button) => (
          <button
            key={button.id}
            type="button"
            onClick={() => press(button)}
            className="aspect-square rounded-button border border-button-border bg-button p-2 text-sm font-medium transition hover:bg-button-hover active:scale-95"
          >
            {button.label}
          </button>
        ))}
      </section>

      <footer className="mt-auto flex items-center justify-between gap-4">
        <p className="truncate text-sm text-board-muted">{lastResult ?? 'Press a button'}</p>
        <button
          type="button"
          onClick={() => sb.stopAll()}
          className="rounded-button bg-accent px-4 py-2 text-sm font-semibold text-white active:scale-95"
        >
          Stop all
        </button>
      </footer>
    </main>
  )
}
