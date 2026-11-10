import { useEffect, useState, type FormEvent } from 'react'
import { comboFromEvent, prettyCombo } from '../lib/keys'

type Props = {
  title: string
  initialLabel?: string
  initialKeys?: string
  withKeys: boolean
  serverOs?: string
  deleteWarning?: string
  onSave: (label: string, keys: string) => Promise<void>
  onDelete?: () => Promise<void>
  onClose: () => void
}

const input =
  'w-full rounded-lg border border-button-border bg-board-bg px-3 py-2 text-board-fg outline-none focus:border-board-fg'

export function ButtonDialog({
  title,
  initialLabel = '',
  initialKeys = '',
  withKeys,
  serverOs,
  deleteWarning,
  onSave,
  onDelete,
  onClose,
}: Props) {
  const [label, setLabel] = useState(initialLabel)
  const [keys, setKeys] = useState(initialKeys)
  const [recording, setRecording] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!recording) return
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault()
      e.stopPropagation()
      if (e.code === 'Escape' && !e.ctrlKey && !e.altKey && !e.shiftKey && !e.metaKey) {
        setRecording(false)
        return
      }
      const combo = comboFromEvent(e)
      if (combo) {
        setKeys(combo)
        setRecording(false)
      }
    }
    window.addEventListener('keydown', onKey, { capture: true })
    return () => window.removeEventListener('keydown', onKey, { capture: true })
  }, [recording])

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
    run(() => onSave(label, keys))
  }

  const remove = () => {
    if (deleteWarning && !confirm(deleteWarning)) return
    if (onDelete) run(onDelete)
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
            placeholder={withKeys ? 'e.g. Browser' : undefined}
            onChange={(e) => setLabel(e.target.value)}
            autoFocus={!withKeys}
          />
        </label>

        {withKeys && (
          <div className="flex flex-col gap-1 text-sm text-board-muted">
            Keys
            <div className="flex gap-2">
              <input
                className={input}
                value={keys}
                placeholder={serverOs === 'windows' ? 'win+shift+b' : 'super+shift+b'}
                onChange={(e) => setKeys(e.target.value)}
                autoFocus
              />
              <button
                type="button"
                onClick={() => setRecording((r) => !r)}
                className={`shrink-0 rounded-lg border px-3 text-board-fg ${
                  recording ? 'border-accent bg-accent/20' : 'border-button-border'
                }`}
              >
                {recording ? 'Press keys…' : 'Record'}
              </button>
            </div>
            <span className="text-xs">
              {keys ? prettyCombo(keys, serverOs) : 'Type a combo or record it on a keyboard.'} Shortcuts your
              desktop already uses can't be recorded on that same PC, so type those.
            </span>
          </div>
        )}

        {error && <p className="text-sm text-accent">{error}</p>}

        <div className="flex items-center gap-2">
          {onDelete && (
            <button
              type="button"
              disabled={busy}
              onClick={remove}
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
            disabled={busy || (withKeys && !keys.trim())}
            className="rounded-lg bg-linear-to-r from-rose-500 to-orange-400 px-4 py-2 text-sm font-semibold text-white transition active:scale-95 disabled:opacity-40"
          >
            {busy ? 'Working…' : 'Save'}
          </button>
        </div>
      </form>
    </div>
  )
}
