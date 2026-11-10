import { useState } from 'react'

type Props = {
  durationMs: number
  /** Client time the sound started; a bar mounted mid-play starts part-way through. */
  startedAt: number
  className: string
}

/** Fills left to right over `durationMs`. Remount (change `key`) to restart it. */
export function ProgressBar({ durationMs, startedAt, className }: Props) {
  const [elapsed] = useState(() => Math.max(0, Date.now() - startedAt))
  return (
    <span
      className={`pointer-events-none absolute inset-x-0 bottom-0 origin-left animate-progress ${className}`}
      style={{ animationDuration: `${durationMs}ms`, animationDelay: `-${elapsed}ms` }}
    />
  )
}
