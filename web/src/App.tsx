import { useEffect, useState } from 'react'
import { useSoundboard, type ConnectionStatus } from './hooks/useSoundboard'
import type { SoundInfo, SoundList } from './protocol'

const STATUS_COLOR: Record<ConnectionStatus, string> = {
  open: 'bg-ok',
  connecting: 'bg-warn',
  closed: 'bg-accent',
}

export default function App() {
  const sb = useSoundboard('dev')
  const [library, setLibrary] = useState<SoundList | null>(null)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [lastResult, setLastResult] = useState<string | null>(null)

  useEffect(() => {
    if (sb.status !== 'open') return
    fetch('/api/sounds')
      .then((res) => (res.ok ? res.json() : Promise.reject(new Error(`HTTP ${res.status}`))))
      .then((list: SoundList) => {
        setLibrary(list)
        setLoadError(null)
      })
      .catch((e: Error) => setLoadError(e.message))
  }, [sb.status])

  const playing = new Set(sb.state.playing.map((p) => p.soundId))

  const press = async (sound: SoundInfo) => {
    const ack = await sb.play(sound.id)
    setLastResult(ack.ok ? `Playing ${sound.label}` : `${sound.label}: ${ack.error ?? 'failed'}`)
  }

  return (
    <main className="mx-auto flex min-h-full max-w-3xl flex-col gap-6 p-4">
      <header className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">Soundboard</h1>
        <div className="flex items-center gap-2 text-sm text-board-muted">
          <span className={`size-2.5 rounded-full ${STATUS_COLOR[sb.status]}`} />
          {sb.status}
          {sb.deviceId && <span>· {sb.deviceId}</span>}
        </div>
      </header>

      {loadError && <p className="text-sm text-accent">Couldn't load sounds: {loadError}</p>}

      {library && library.sounds.length === 0 && (
        <p className="text-sm text-board-muted">
          No sounds yet. Copy mp3, ogg, wav or flac files into{' '}
          <code className="text-board-fg">{library.libraryDir}</code> and restart the server.
        </p>
      )}

      <section className="grid grid-cols-3 gap-board-gap sm:grid-cols-4">
        {library?.sounds.map((sound) => (
          <button
            key={sound.id}
            type="button"
            onClick={() => press(sound)}
            className={`aspect-square rounded-button border bg-button p-2 text-sm font-medium capitalize transition hover:bg-button-hover active:scale-95 ${
              playing.has(sound.id) ? 'border-accent ring-2 ring-accent' : 'border-button-border'
            }`}
          >
            {sound.label}
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
