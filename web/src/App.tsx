import { useCallback, useEffect, useRef, useState, type ChangeEvent } from 'react'
import { AppDialog } from './components/AppDialog'
import { BoardButton, type Feedback } from './components/BoardButton'
import { ButtonDialog } from './components/ButtonDialog'
import { ObsDialog } from './components/ObsDialog'
import { ObsSettingsDialog } from './components/ObsSettingsDialog'
import { NowPlaying } from './components/NowPlaying'
import { useSoundboard, type ConnectionStatus } from './hooks/useSoundboard'
import { addApp, addHotkey, addObsScene, deleteButton, getBoard, updateButton, uploadSound } from './lib/api'
import type { BoardView, Button } from './protocol'

const STATUS_STYLE: Record<ConnectionStatus, string> = {
  open: 'bg-ok shadow-[0_0_8px] shadow-ok',
  connecting: 'bg-warn animate-pulse',
  closed: 'bg-accent',
}

const ACCEPT = 'audio/*,.mp3,.ogg,.oga,.wav,.flac,.m4a'
const TOAST_MS = 2500
const ERROR_TOAST_MS = 5000
const FEEDBACK_MS = 700

type Dialog =
  | { mode: 'newHotkey' }
  | { mode: 'newObs' }
  | { mode: 'newApp' }
  | { mode: 'obsSettings' }
  | { mode: 'edit'; button: Button }
type Toast = { id: number; text: string; error: boolean }

const addTile =
  'flex aspect-square w-full flex-col items-center justify-center gap-1 rounded-button border-2 border-dashed border-button-border text-sm font-medium text-board-muted transition hover:border-board-muted hover:text-board-fg active:scale-95'

export default function App() {
  const sb = useSoundboard('dev')
  const [board, setBoard] = useState<BoardView | null>(null)
  const [editing, setEditing] = useState(false)
  const [dialog, setDialog] = useState<Dialog | null>(null)
  const [toast, setToast] = useState<Toast | null>(null)
  const [feedback, setFeedback] = useState<Record<string, Feedback>>({})
  const fileInput = useRef<HTMLInputElement>(null)
  const seq = useRef(0)

  const notify = useCallback((text: string, error = false) => {
    const id = ++seq.current
    setToast({ id, text, error })
    setTimeout(() => setToast((t) => (t?.id === id ? null : t)), error ? ERROR_TOAST_MS : TOAST_MS)
  }, [])

  const flash = (buttonId: string, kind: Feedback['kind']) => {
    const n = ++seq.current
    setFeedback((f) => ({ ...f, [buttonId]: { kind, n } }))
    setTimeout(
      () =>
        setFeedback((f) => {
          if (f[buttonId]?.n !== n) return f
          const { [buttonId]: _, ...rest } = f
          return rest
        }),
      FEEDBACK_MS,
    )
  }

  useEffect(() => {
    const openObsSettings = () => {
      if (window.location.hash !== '#obs') return
      setDialog({ mode: 'obsSettings' })
      window.history.replaceState(null, '', window.location.pathname + window.location.search)
    }
    openObsSettings()
    window.addEventListener('hashchange', openObsSettings)
    return () => window.removeEventListener('hashchange', openObsSettings)
  }, [])

  useEffect(() => {
    getBoard()
      .then(setBoard)
      .catch((e: Error) => notify(`Couldn't load board: ${e.message}`, true))
  }, [sb.boardVersion, notify])

  const durations = new Map(board?.sounds.map((s) => [s.id, s.durationMs]))
  const playCounts = new Map<string, number>()
  for (const p of sb.state.playing) playCounts.set(p.soundId, (playCounts.get(p.soundId) ?? 0) + 1)
  const soundLabel = (soundId: string) =>
    board?.buttons.find((b) => b.kind === 'sound' && b.soundId === soundId)?.label ??
    board?.sounds.find((s) => s.id === soundId)?.label ??
    soundId

  const press = async (button: Button) => {
    if (editing) {
      setDialog({ mode: 'edit', button })
      return
    }
    const ack = await sb.press(button.id)
    if (!ack.ok) {
      flash(button.id, 'error')
      notify(`${button.label}: ${ack.error ?? 'failed'}`, true)
    } else if (button.kind === 'hotkey' || button.kind === 'obs' || button.kind === 'app') {
      flash(button.id, 'ok')
    }
  }

  const onFiles = async (e: ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(e.target.files ?? [])
    e.target.value = ''
    const limitMb = board?.maxUploadMb ?? 20
    for (const file of files) {
      if (file.size > limitMb * 1024 * 1024) {
        notify(`${file.name} is over ${limitMb} MB`, true)
        continue
      }
      notify(`Uploading ${file.name}…`)
      try {
        const button = await uploadSound(file)
        notify(`Added ${button.label}`)
      } catch (err) {
        notify(`${file.name}: ${(err as Error).message}`, true)
      }
    }
  }

  return (
    <div className="flex min-h-full flex-col">
      <header className="sticky top-0 z-10 border-b border-white/5 bg-board-bg/70 backdrop-blur-md">
        <div className="mx-auto flex max-w-3xl items-center justify-between gap-4 px-4 py-3">
          <h1 className="flex items-center gap-2 text-xl font-bold tracking-tight">
            <img src="/streamboard.png" alt="" className="size-8 rounded-lg" />
            <span className="bg-linear-to-r from-rose-400 via-fuchsia-400 to-indigo-400 bg-clip-text text-transparent">
              StreamBoard
            </span>
          </h1>
          <div className="flex items-center gap-3 text-sm text-board-muted">
            <span className="flex items-center gap-2" title={sb.status}>
              <span className={`size-2.5 rounded-full ${STATUS_STYLE[sb.status]}`} />
              <span className="hidden sm:inline">{sb.status}</span>
            </span>
            <button
              type="button"
              onClick={() => setDialog({ mode: 'obsSettings' })}
              className="rounded-full border border-button-border px-3 py-1 transition hover:border-board-muted hover:text-board-fg"
            >
              OBS
            </button>
            <button
              type="button"
              onClick={() => setEditing((v) => !v)}
              className={`rounded-full border px-3 py-1 transition ${
                editing
                  ? 'border-transparent bg-board-fg text-board-bg'
                  : 'border-button-border hover:border-board-muted hover:text-board-fg'
              }`}
            >
              {editing ? 'Done' : 'Edit'}
            </button>
          </div>
        </div>
      </header>

      <main className="mx-auto flex w-full max-w-3xl flex-1 flex-col gap-4 p-4">
        {editing && (
          <p className="animate-fade text-sm text-board-muted">Tap a button to rename, change or delete it.</p>
        )}

        <section className="grid grid-cols-3 gap-board-gap sm:grid-cols-4">
          {board?.buttons.map((button) => {
            const soundId = button.kind === 'sound' ? button.soundId : undefined
            return (
              <BoardButton
                key={button.id}
                button={button}
                editing={editing}
                playing={soundId !== undefined && playCounts.has(soundId)}
                durationMs={soundId ? durations.get(soundId) : undefined}
                startedAt={soundId ? sb.startedAt[soundId] : undefined}
                feedback={feedback[button.id]}
                serverOs={board?.serverOs}
                onPress={() => press(button)}
              />
            )
          })}

          <button type="button" onClick={() => fileInput.current?.click()} className={addTile}>
            <span className="text-2xl leading-none">+</span>
            Sound
          </button>
          <button type="button" onClick={() => setDialog({ mode: 'newHotkey' })} className={addTile}>
            <span className="text-2xl leading-none">+</span>
            Hotkey
          </button>
          <button type="button" onClick={() => setDialog({ mode: 'newObs' })} className={addTile}>
            <span className="text-2xl leading-none">+</span>
            OBS
          </button>
          <button type="button" onClick={() => setDialog({ mode: 'newApp' })} className={addTile}>
            <span className="text-2xl leading-none">+</span>
            App
          </button>
        </section>

        <input ref={fileInput} type="file" accept={ACCEPT} multiple hidden onChange={onFiles} />

        {board && board.buttons.length === 0 && (
          <p className="text-sm text-board-muted">
            Add sounds with <strong>+ Sound</strong>, key combos with <strong>+ Hotkey</strong>, OBS scenes
            with <strong>+ OBS</strong>, or programs on this PC with <strong>+ App</strong>. Copy sound files into{' '}
            <code className="text-board-fg">{board.libraryDir}</code> on the server too.
          </p>
        )}
      </main>

      <footer className="sticky bottom-0 border-t border-white/5 bg-board-bg/80 backdrop-blur-md">
        <div className="mx-auto flex max-w-3xl items-center justify-between gap-3 px-4 py-3 pb-[max(0.75rem,env(safe-area-inset-bottom))]">
          <NowPlaying
            items={[...playCounts].map(([soundId, count]) => ({
              soundId,
              count,
              label: soundLabel(soundId),
              durationMs: durations.get(soundId),
              startedAt: sb.startedAt[soundId],
            }))}
            onStop={(soundId) => sb.stop(soundId)}
          />
          <button
            type="button"
            onClick={() => sb.stopAll()}
            disabled={playCounts.size === 0}
            className="shrink-0 rounded-full bg-linear-to-r from-rose-500 to-orange-400 px-4 py-2 text-sm font-semibold text-white shadow-lg shadow-rose-500/20 transition active:scale-95 disabled:opacity-30 disabled:shadow-none"
          >
            Stop all
          </button>
        </div>
      </footer>

      {toast && (
        <div
          key={toast.id}
          role="status"
          className={`fixed bottom-20 left-1/2 z-30 max-w-[90vw] animate-toast truncate rounded-full border px-4 py-2 text-sm shadow-xl backdrop-blur-md ${
            toast.error
              ? 'border-accent/40 bg-accent/20 text-rose-100'
              : 'border-white/10 bg-white/10 text-board-fg'
          }`}
        >
          {toast.text}
        </div>
      )}

      {dialog?.mode === 'newHotkey' && (
        <ButtonDialog
          title="New hotkey"
          withKeys
          serverOs={board?.serverOs}
          onSave={async (label, keys) => {
            await addHotkey({ label, keys })
          }}
          onClose={() => setDialog(null)}
        />
      )}

      {dialog?.mode === 'obsSettings' && <ObsSettingsDialog onClose={() => setDialog(null)} />}

      {dialog?.mode === 'newObs' && (
        <ObsDialog
          title="New OBS scene"
          onSave={async (label, scene) => {
            await addObsScene({ label, scene })
          }}
          onClose={() => setDialog(null)}
        />
      )}

      {dialog?.mode === 'newApp' && (
        <AppDialog
          title="New app"
          onSave={async (label, path) => {
            await addApp({ label, path })
          }}
          onClose={() => setDialog(null)}
        />
      )}

      {dialog?.mode === 'edit' && dialog.button.kind === 'app' && (
        <AppDialog
          title="Edit app"
          initialLabel={dialog.button.label}
          initialPath={dialog.button.path}
          onSave={async (label, path) => {
            await updateButton(dialog.button.id, { label, path })
          }}
          onDelete={async () => {
            await deleteButton(dialog.button.id)
          }}
          onClose={() => setDialog(null)}
        />
      )}

      {dialog?.mode === 'edit' && dialog.button.kind === 'obs' && (
        <ObsDialog
          title="Edit OBS scene"
          initialLabel={dialog.button.label}
          initialScene={dialog.button.scene}
          onSave={async (label, scene) => {
            await updateButton(dialog.button.id, { label, scene })
          }}
          onDelete={async () => {
            await deleteButton(dialog.button.id)
          }}
          onClose={() => setDialog(null)}
        />
      )}

      {dialog?.mode === 'edit' && (dialog.button.kind === 'sound' || dialog.button.kind === 'hotkey') && (
        <ButtonDialog
          title={dialog.button.kind === 'sound' ? 'Edit sound' : 'Edit hotkey'}
          initialLabel={dialog.button.label}
          initialKeys={dialog.button.kind === 'hotkey' ? dialog.button.keys : undefined}
          withKeys={dialog.button.kind === 'hotkey'}
          serverOs={board?.serverOs}
          deleteWarning={
            dialog.button.kind === 'sound'
              ? `Delete "${dialog.button.label}"? This also deletes the sound file from the server.`
              : undefined
          }
          onSave={async (label, keys) => {
            const { button } = dialog
            await updateButton(button.id, button.kind === 'hotkey' ? { label, keys } : { label })
          }}
          onDelete={async () => {
            await deleteButton(dialog.button.id)
          }}
          onClose={() => setDialog(null)}
        />
      )}
    </div>
  )
}
