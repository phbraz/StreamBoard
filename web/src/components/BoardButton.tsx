import { prettyCombo } from '../lib/keys'
import type { Button } from '../protocol'
import { ProgressBar } from './ProgressBar'

// Full class strings so Tailwind can see them at build time.
const GRADIENTS = [
  'from-rose-500 to-orange-400',
  'from-fuchsia-500 to-pink-500',
  'from-violet-500 to-indigo-500',
  'from-sky-500 to-cyan-400',
  'from-emerald-500 to-teal-400',
  'from-amber-400 to-orange-500',
  'from-blue-600 to-violet-500',
  'from-pink-500 to-rose-400',
]

function gradientFor(id: string) {
  let hash = 0
  for (const c of id) hash = (hash * 31 + c.charCodeAt(0)) | 0
  return GRADIENTS[Math.abs(hash) % GRADIENTS.length]
}

export type Feedback = { kind: 'ok' | 'error'; n: number }

type Props = {
  button: Button
  editing: boolean
  playing: boolean
  /** Sound length and client-side start time, for the progress bar. */
  durationMs?: number
  startedAt?: number
  feedback?: Feedback
  serverOs?: string
  onPress: () => void
}

export function BoardButton({
  button,
  editing,
  playing,
  durationMs,
  startedAt,
  feedback,
  serverOs,
  onPress,
}: Props) {
  const tone =
    button.kind === 'sound'
      ? `bg-linear-to-br ${gradientFor(button.id)}`
      : button.kind === 'obs'
        ? 'border border-white/10 bg-linear-to-br from-indigo-900 to-zinc-900'
        : button.kind === 'app'
          ? 'border border-white/10 bg-linear-to-br from-emerald-900 to-zinc-900'
          : 'border border-white/10 bg-linear-to-br from-zinc-700 to-zinc-900'

  // Each animate-* class sets `animation`, so the one-off pop-in lives on the wrapper
  // and doesn't replay when the glow/shake on the button change.
  return (
    <div className="animate-pop">
      <button
        type="button"
        onClick={onPress}
        className={`relative flex aspect-square w-full flex-col items-center justify-center gap-1.5 overflow-hidden rounded-button p-2 text-sm font-semibold text-white shadow-lg shadow-black/40 transition duration-150 select-none hover:-translate-y-0.5 hover:brightness-110 active:translate-y-0 active:scale-95 ${tone} ${playing ? 'animate-glow' : ''} ${feedback?.kind === 'error' ? 'animate-shake' : ''} ${
          editing ? 'outline-2 outline-offset-2 outline-white/40 outline-dashed' : ''
        }`}
      >
        <span className="pointer-events-none absolute inset-x-0 top-0 h-1/2 bg-linear-to-b from-white/20 to-transparent" />

        {playing && durationMs && startedAt ? (
          <ProgressBar key={startedAt} durationMs={durationMs} startedAt={startedAt} className="h-1.5 bg-white/80" />
        ) : null}

        {feedback && (
          <span
            key={feedback.n}
            className={`pointer-events-none absolute inset-0 animate-flash ${
              feedback.kind === 'ok' ? 'bg-white' : 'bg-red-600'
            }`}
          />
        )}

        {playing && (
          <span className="absolute top-2 right-2 flex h-3 items-end gap-0.5" aria-hidden>
            {[0, 150, 300].map((delay) => (
              <span
                key={delay}
                className="h-full w-0.5 origin-bottom animate-eq rounded-full bg-white"
                style={{ animationDelay: `${delay}ms` }}
              />
            ))}
          </span>
        )}

        {editing && (
          <span className="absolute top-2 left-2 rounded-full bg-black/40 px-1.5 text-[0.6rem] font-medium">
            edit
          </span>
        )}

        <span className="relative line-clamp-2 text-center break-words drop-shadow">{button.label}</span>
        {button.kind === 'hotkey' && (
          <span className="relative rounded-md border border-white/15 bg-black/30 px-1.5 py-0.5 font-mono text-[0.65rem] font-normal text-white/80">
            {prettyCombo(button.keys, serverOs)}
          </span>
        )}
        {button.kind === 'obs' && (
          <span className="relative max-w-full truncate rounded-md border border-white/15 bg-black/30 px-1.5 py-0.5 text-[0.65rem] font-normal text-white/80">
            {button.scene}
          </span>
        )}
        {button.kind === 'app' && (
          <span className="relative max-w-full truncate rounded-md border border-white/15 bg-black/30 px-1.5 py-0.5 text-[0.65rem] font-normal text-white/80">
            {button.path.split(/[/\\]/).pop()}
          </span>
        )}
      </button>
    </div>
  )
}
