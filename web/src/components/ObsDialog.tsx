import { useEffect, useState, type FormEvent } from 'react'
import { getObsScenes } from '../lib/api'
import type { ObsSceneInfo } from '../protocol'

type Props = {
  title: string
  initialLabel?: string
  initialScene?: string
  onSave: (label: string, scene: string) => Promise<void>
  onDelete?: () => Promise<void>
  onClose: () => void
}

const input =
  'w-full rounded-lg border border-button-border bg-board-bg px-3 py-2 text-board-fg outline-none focus:border-board-fg'

export function ObsDialog({ title, initialLabel = '', initialScene = '', onSave, onDelete, onClose }: Props) {
  const [label, setLabel] = useState(initialLabel)
  const [scene, setScene] = useState(initialScene)
  const [scenes, setScenes] = useState<ObsSceneInfo[] | null>(null)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    getObsScenes()
      .then((list) => {
        if (cancelled) return
        setScenes(list.scenes)
        setScene((current) => {
          if (current && list.scenes.some((s) => s.name === current)) return current
          return list.scenes.find((s) => s.current)?.name ?? list.scenes[0]?.name ?? current
        })
        setLabel((current) => {
          if (current.trim()) return current
          const picked =
            (initialScene && list.scenes.find((s) => s.name === initialScene)?.name) ||
            list.scenes.find((s) => s.current)?.name ||
            list.scenes[0]?.name
          return picked ?? current
        })
      })
      .catch((e: Error) => {
        if (!cancelled) setLoadError(e.message)
      })
    return () => {
      cancelled = true
    }
  }, [initialScene])

  const run = async (action: () => Promise<void>) => {
    setBusy(true)
    setError(null)
    try {
      await action()
      onClose()
    } catch (e) {
      setError((e as Error).message)
    } finally {
      setBusy(false)
    }
  }

  const submit = (e: FormEvent) => {
    e.preventDefault()
    run(() => onSave(label, scene))
  }

  const chooseScene = (name: string) => {
    setLabel((current) => (current.trim() === '' || current === scene ? name : current))
    setScene(name)
  }

  return (
    <div
      className="fixed inset-0 z-20 flex animate-fade items-end justify-center bg-black/60 p-4 backdrop-blur-sm sm:items-center"
      onClick={onClose}
    >
      <form
        onSubmit={submit}
        onClick={(e) => e.stopPropagation()}
        className="flex w-full max-w-sm animate-sheet flex-col gap-4 rounded-button border border-button-border bg-button p-5 shadow-2xl shadow-black/60"
      >
        <h2 className="text-lg font-semibold">{title}</h2>

        <label className="flex flex-col gap-1 text-sm text-board-muted">
          Label
          <input
            className={input}
            value={label}
            maxLength={40}
            placeholder="e.g. Starting soon"
            onChange={(e) => setLabel(e.target.value)}
            autoFocus
          />
        </label>

        <label className="flex flex-col gap-1 text-sm text-board-muted">
          Scene
          {scenes && scenes.length > 0 ? (
            <select className={input} value={scene} onChange={(e) => chooseScene(e.target.value)}>
              {scenes.map((item) => (
                <option key={item.name} value={item.name}>
                  {item.name}
                  {item.current ? ' (live)' : ''}
                </option>
              ))}
            </select>
          ) : (
            <input
              className={input}
              value={scene}
              placeholder={scenes ? 'OBS has no scenes' : 'Loading scenes…'}
              onChange={(e) => setScene(e.target.value)}
              disabled={scenes !== null}
            />
          )}
          <span className="text-xs">
            {loadError ??
              'Pressing the button switches the program scene in OBS on this PC. Enable the websocket server in OBS if the list stays empty.'}
          </span>
        </label>

        {error && <p className="text-sm text-accent">{error}</p>}

        <div className="flex items-center gap-2">
          {onDelete && (
            <button
              type="button"
              disabled={busy}
              onClick={() => {
                if (onDelete) run(onDelete)
              }}
              className="mr-auto rounded-lg px-3 py-2 text-sm text-accent hover:bg-accent/10"
            >
              Delete
            </button>
          )}
          <button
            type="button"
            onClick={onClose}
            className="ml-auto rounded-lg px-3 py-2 text-sm text-board-muted hover:text-board-fg"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={busy || !scene.trim() || (scenes !== null && scenes.length === 0)}
            className="rounded-lg bg-linear-to-r from-indigo-500 to-violet-400 px-4 py-2 text-sm font-semibold text-white transition active:scale-95 disabled:opacity-40"
          >
            {busy ? 'Working…' : 'Save'}
          </button>
        </div>
      </form>
    </div>
  )
}
