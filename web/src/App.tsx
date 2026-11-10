import { useEffect, useRef, useState, type ChangeEvent } from 'react'
import { ButtonDialog } from './components/ButtonDialog'
import { useSoundboard, type ConnectionStatus } from './hooks/useSoundboard'
import { addHotkey, deleteButton, getBoard, updateButton, uploadSound } from './lib/api'
import { prettyCombo } from './lib/keys'
import type { BoardView, Button } from './protocol'

const STATUS_COLOR: Record<ConnectionStatus, string> = {
  open: 'bg-ok',
  connecting: 'bg-warn',
  closed: 'bg-accent',
}

const ACCEPT = 'audio/*,.mp3,.ogg,.oga,.wav,.flac,.m4a'

type Dialog = { mode: 'newHotkey' } | { mode: 'edit'; button: Button }

const tile =
  'flex aspect-square flex-col items-center justify-center gap-1 rounded-button border p-2 text-sm font-medium transition active:scale-95'

export default function App() {
  const sb = useSoundboard('dev')
  const [board, setBoard] = useState<BoardView | null>(null)
  const [message, setMessage] = useState<{ text: string; error?: boolean } | null>(null)
  const [editing, setEditing] = useState(false)
  const [dialog, setDialog] = useState<Dialog | null>(null)
  const fileInput = useRef<HTMLInputElement>(null)

  useEffect(() => {
    getBoard()
      .then(setBoard)
      .catch((e: Error) => setMessage({ text: `Couldn't load board: ${e.message}`, error: true }))
  }, [sb.boardVersion])

  const playing = new Set(sb.state.playing.map((p) => p.soundId))

  const press = async (button: Button) => {
    if (editing) {
      setDialog({ mode: 'edit', button })
      return
    }
    const ack = await sb.press(button.id)
    setMessage(
      ack.ok
        ? { text: button.kind === 'sound' ? `Playing ${button.label}` : `Sent ${prettyCombo(button.keys)}` }
        : { text: `${button.label}: ${ack.error ?? 'failed'}`, error: true },
    )
  }

  const onFiles = async (e: ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(e.target.files ?? [])
    e.target.value = ''
    const limit = (board?.maxUploadMb ?? 20) * 1024 * 1024
    for (const file of files) {
      if (file.size > limit) {
        setMessage({ text: `${file.name} is over ${board?.maxUploadMb} MB`, error: true })
        continue
      }
      setMessage({ text: `Uploading ${file.name}…` })
      try {
        const button = await uploadSound(file)
        setMessage({ text: `Added ${button.label}` })
      } catch (err) {
        setMessage({ text: `${file.name}: ${(err as Error).message}`, error: true })
      }
    }
  }

  return (
    <main className="mx-auto flex min-h-full max-w-3xl flex-col gap-6 p-4">
      <header className="flex items-center justify-between gap-4">
        <h1 className="text-xl font-semibold">Soundboard</h1>
        <div className="flex items-center gap-3 text-sm text-board-muted">
          <span className="flex items-center gap-2">
            <span className={`size-2.5 rounded-full ${STATUS_COLOR[sb.status]}`} />
            {sb.status}
          </span>
          <button
            type="button"
            onClick={() => setEditing((v) => !v)}
            className={`rounded-lg border px-3 py-1 ${
              editing ? 'border-board-fg text-board-fg' : 'border-button-border hover:text-board-fg'
            }`}
          >
            {editing ? 'Done' : 'Edit'}
          </button>
        </div>
      </header>

      {editing && (
        <p className="-mt-3 text-sm text-board-muted">Tap a button to rename, change or delete it.</p>
      )}

      <section className="grid grid-cols-3 gap-board-gap sm:grid-cols-4">
        {board?.buttons.map((button) => (
          <button
            key={button.id}
            type="button"
            onClick={() => press(button)}
            className={`${tile} bg-button hover:bg-button-hover ${
              button.kind === 'sound' && playing.has(button.soundId)
                ? 'border-accent ring-2 ring-accent'
                : editing
                  ? 'border-dashed border-board-muted'
                  : 'border-button-border'
            }`}
          >
            <span className="line-clamp-2 break-words text-center">{button.label}</span>
            {button.kind === 'hotkey' && (
              <span className="font-mono text-[0.65rem] text-board-muted">{prettyCombo(button.keys)}</span>
            )}
          </button>
        ))}

        <button
          type="button"
          onClick={() => fileInput.current?.click()}
          className={`${tile} border-dashed border-button-border text-board-muted hover:text-board-fg`}
        >
          <span className="text-2xl leading-none">+</span>
          Sound
        </button>
        <button
          type="button"
          onClick={() => setDialog({ mode: 'newHotkey' })}
          className={`${tile} border-dashed border-button-border text-board-muted hover:text-board-fg`}
        >
          <span className="text-2xl leading-none">+</span>
          Hotkey
        </button>
      </section>

      <input ref={fileInput} type="file" accept={ACCEPT} multiple hidden onChange={onFiles} />

      {board && board.buttons.length === 0 && (
        <p className="text-sm text-board-muted">
          Add sounds with <strong>+ Sound</strong>, or copy files into{' '}
          <code className="text-board-fg">{board.libraryDir}</code> on the server.
        </p>
      )}

      <footer className="mt-auto flex items-center justify-between gap-4">
        <p className={`truncate text-sm ${message?.error ? 'text-accent' : 'text-board-muted'}`}>
          {message?.text ?? 'Press a button'}
        </p>
        <button
          type="button"
          onClick={() => sb.stopAll()}
          className="shrink-0 rounded-button bg-accent px-4 py-2 text-sm font-semibold text-white active:scale-95"
        >
          Stop all
        </button>
      </footer>

      {dialog?.mode === 'newHotkey' && (
        <ButtonDialog
          title="New hotkey"
          withKeys
          onSave={async (label, keys) => {
            await addHotkey({ label, keys })
          }}
          onClose={() => setDialog(null)}
        />
      )}

      {dialog?.mode === 'edit' && (
        <ButtonDialog
          title={dialog.button.kind === 'sound' ? 'Edit sound' : 'Edit hotkey'}
          initialLabel={dialog.button.label}
          initialKeys={dialog.button.kind === 'hotkey' ? dialog.button.keys : undefined}
          withKeys={dialog.button.kind === 'hotkey'}
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
    </main>
  )
}
