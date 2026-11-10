import { ProgressBar } from './ProgressBar'

type Item = { soundId: string; label: string; count: number; durationMs?: number; startedAt?: number }

type Props = {
  items: Item[]
  onStop: (soundId: string) => void
}

export function NowPlaying({ items, onStop }: Props) {
  if (items.length === 0) {
    return <p className="truncate text-sm text-board-muted">Nothing playing</p>
  }

  return (
    <ul className="flex min-w-0 gap-2 overflow-x-auto">
      {items.map((item) => {
        return (
          <li key={item.soundId} className="animate-pop">
            <button
              type="button"
              onClick={() => onStop(item.soundId)}
              title="Stop"
              className="relative flex items-center gap-2 overflow-hidden rounded-full border border-white/10 bg-white/5 py-1.5 pr-3 pl-2 text-sm whitespace-nowrap hover:bg-white/10"
            >
              <span className="flex h-3 items-end gap-0.5" aria-hidden>
                {[0, 150, 300].map((delay) => (
                  <span
                    key={delay}
                    className="h-full w-0.5 origin-bottom animate-eq rounded-full bg-accent"
                    style={{ animationDelay: `${delay}ms` }}
                  />
                ))}
              </span>
              {item.label}
              {item.count > 1 && <span className="text-board-muted">×{item.count}</span>}
              <span className="text-board-muted">■</span>
              {item.durationMs && item.startedAt ? (
                <ProgressBar
                  key={item.startedAt}
                  durationMs={item.durationMs}
                  startedAt={item.startedAt}
                  className="h-0.5 bg-accent"
                />
              ) : null}
            </button>
          </li>
        )
      })}
    </ul>
  )
}
