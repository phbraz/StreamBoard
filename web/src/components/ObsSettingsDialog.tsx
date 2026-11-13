import { useEffect, useState, type FormEvent } from 'react'
import { getObsSettings, saveObsSettings } from '../lib/api'

type Props = {
  onClose: () => void
}

const input =
  'w-full rounded-lg border border-button-border bg-board-bg px-3 py-2 text-board-fg outline-none focus:border-board-fg'

export function ObsSettingsDialog({ onClose }: Props) {
  const [host, setHost] = useState('')
  const [port, setPort] = useState('4455')
  const [password, setPassword] = useState('')
  const [passwordSet, setPasswordSet] = useState(false)
  const [clearPassword, setClearPassword] = useState(false)
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [message, setMessage] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    getObsSettings()
      .then((settings) => {
        if (cancelled) return
        setHost(settings.host)
        setPort(String(settings.port))
        setPasswordSet(settings.passwordSet)
      })
      .catch((e: Error) => {
        if (!cancelled) setError(e.message)
      })
      .finally(() => {
        if (!cancelled) setLoading(false)
      })
    return () => {
      cancelled = true
    }
  }, [])

  const submit = async (e: FormEvent) => {
    e.preventDefault()
    const parsed = Number(port)
    if (!Number.isInteger(parsed) || parsed < 1 || parsed > 65535) {
      setError('Port must be a number from 1 to 65535')
      return
    }
    setBusy(true)
    setError(null)
    setMessage(null)
    try {
      const saved = await saveObsSettings({
        host,
        port: parsed,
        password: clearPassword ? '' : password === '' ? undefined : password,
      })
      setHost(saved.settings.host)
      setPort(String(saved.settings.port))
      setPasswordSet(saved.settings.passwordSet)
      setPassword('')
      setClearPassword(false)
      setMessage(saved.message)
    } catch (err) {
      setError((err as Error).message)
    } finally {
      setBusy(false)
    }
  }

  return (
    <div
      className="fixed inset-0 z-20 flex animate-fade items-end justify-center bg-black/60 p-4 backdrop-blur-sm sm:items-center"
      onClick={onClose}
    >
      <form
        className="w-full max-w-md animate-fade rounded-2xl border border-white/10 bg-board-bg p-5 shadow-2xl"
        onClick={(e) => e.stopPropagation()}
        onSubmit={submit}
      >
        <h2 className="text-lg font-semibold text-board-fg">OBS connection</h2>
        <p className="mt-1 text-sm text-board-muted">
          Host, port, and password from OBS: Tools → WebSocket Server Settings. Saved on this PC for StreamBoard.
        </p>

        <label className="mt-4 block text-sm text-board-muted">
          Server
          <input
            className={`${input} mt-1`}
            value={host}
            onChange={(e) => setHost(e.target.value)}
            placeholder="127.0.0.1"
            autoFocus
            disabled={loading || busy}
          />
        </label>
        <label className="mt-3 block text-sm text-board-muted">
          Port
          <input
            className={`${input} mt-1`}
            value={port}
            onChange={(e) => setPort(e.target.value)}
            inputMode="numeric"
            disabled={loading || busy}
          />
        </label>
        <label className="mt-3 block text-sm text-board-muted">
          Password
          <input
            className={`${input} mt-1`}
            type="password"
            value={password}
            onChange={(e) => {
              setPassword(e.target.value)
              if (e.target.value) setClearPassword(false)
            }}
            placeholder={passwordSet ? 'Saved password stays if this is blank' : 'No password'}
            disabled={loading || busy || clearPassword}
            autoComplete="off"
          />
        </label>
        {passwordSet && (
          <label className="mt-2 flex items-center gap-2 text-sm text-board-muted">
            <input
              type="checkbox"
              checked={clearPassword}
              onChange={(e) => setClearPassword(e.target.checked)}
              disabled={busy}
            />
            Remove the saved password
          </label>
        )}

        {error && <p className="mt-3 text-sm text-rose-300">{error}</p>}
        {message && <p className="mt-3 text-sm text-emerald-300">{message}</p>}

        <div className="mt-5 flex justify-end gap-2">
          <button
            type="button"
            onClick={onClose}
            className="rounded-full border border-button-border px-4 py-2 text-sm text-board-muted hover:text-board-fg"
          >
            Close
          </button>
          <button
            type="submit"
            disabled={loading || busy}
            className="rounded-full bg-indigo-500 px-4 py-2 text-sm font-semibold text-white disabled:opacity-40"
          >
            {busy ? 'Saving…' : 'Save'}
          </button>
        </div>
      </form>
    </div>
  )
}
