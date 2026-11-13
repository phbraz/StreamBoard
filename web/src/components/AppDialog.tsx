import { useEffect, useMemo, useState, type FormEvent } from 'react'
import { getApps } from '../lib/api'
import type { AppInfo } from '../protocol'

type Props = {
  title: string
  initialLabel?: string
  initialPath?: string
  onSave: (label: string, path: string) => Promise<void>
  onDelete?: () => Promise<void>
  onClose: () => void
}

const input =
  'w-full rounded-lg border border-button-border bg-board-bg px-3 py-2 text-board-fg outline-none focus:border-board-fg'

export function AppDialog({ title, initialLabel = '', initialPath = '', onSave, onDelete, onClose }: Props) {
  const [label, setLabel] = useState(initialLabel)
  const [path, setPath] = useState(initialPath)
  const [query, setQuery] = useState('')
  const [apps, setApps] = useState<AppInfo[] | null>(null)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    getApps()
      .then((list) => {
        if (!cancelled) setApps(list.apps)
      })
      .catch((e: Error) => {
        if (!cancelled) setLoadError(e.message)
      })
    return () => {
      cancelled = true
    }
  }, [])

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase()
    const list = apps ?? []
    const matched = q
      ? list.filter((app) => app.name.toLowerCase().includes(q) || app.path.toLowerCase().includes(q))
      : list
    return matched.slice(0, 80)
  }, [apps, query])

  const choose = (app: AppInfo) => {
    setLabel((current) => {
      const previous = apps?.find((item) => item.path === path)
      if (current.trim() === '' || current === previous?.name || current === labelFromPath(path)) return app.name
      return current
    })
    setPath(app.path)
  }

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
    run(() => onSave(label, path))
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
            placeholder="e.g. Discord"
            onChange={(e) => setLabel(e.target.value)}
            autoFocus
          />
        </label>

        <div className="flex flex-col gap-1 text-sm text-board-muted">
          Apps on this PC
          <input
            className={input}
            value={query}
            placeholder={apps ? `Search ${apps.length} apps` : 'Loading apps…'}
            onChange={(e) => setQuery(e.target.value)}
          />
          <div className="max-h-40 overflow-auto rounded-lg border border-button-border">
            {filtered.length === 0 ? (
              <p className="px-3 py-2 text-xs">{loadError ?? (apps ? 'No matching apps.' : 'Loading…')}</p>
            ) : (
              filtered.map((app) => (
                <button
                  key={app.path}
                  type="button"
                  onClick={() => choose(app)}
                  className={`block w-full truncate px-3 py-1.5 text-left text-sm ${
                    app.path === path ? 'bg-emerald-500/20 text-board-fg' : 'text-board-muted hover:bg-white/5 hover:text-board-fg'
                  }`}
                >
                  {app.name}
                </button>
              ))
            )}
          </div>
        </div>

        <label className="flex flex-col gap-1 text-sm text-board-muted">
          Path
          <input
            className={input}
            value={path}
            placeholder="C:\Path\App.exe"
            onChange={(e) => setPath(e.target.value)}
          />
          <span className="text-xs">
            Pick a Start menu shortcut, or paste an .exe or .lnk path. The server starts it directly. Shells and
            scripts are refused.
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
            disabled={busy || !path.trim()}
            className="rounded-lg bg-linear-to-r from-emerald-500 to-teal-400 px-4 py-2 text-sm font-semibold text-white transition active:scale-95 disabled:opacity-40"
          >
            {busy ? 'Working…' : 'Save'}
          </button>
        </div>
      </form>
    </div>
  )
}

function labelFromPath(path: string) {
  const name = path.split(/[/\\]/).pop() ?? ''
  return name.replace(/\.(exe|lnk)$/i, '')
}
