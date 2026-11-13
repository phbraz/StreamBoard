import type { AppList, BoardView, Button, ButtonPatch, NewApp, NewHotkey, NewObsScene, ObsSceneList } from '../protocol'

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(path, init)
  if (!res.ok) {
    const body = await res.json().catch(() => null)
    throw new Error(body?.error ?? (res.status === 413 ? 'file is too large' : `HTTP ${res.status}`))
  }
  return (res.status === 204 ? undefined : await res.json()) as T
}

const json = (method: string, body: unknown): RequestInit => ({
  method,
  headers: { 'content-type': 'application/json' },
  body: JSON.stringify(body),
})

export const getBoard = () => request<BoardView>('/api/board')

export function uploadSound(file: File, label?: string) {
  const form = new FormData()
  form.append('file', file)
  if (label) form.append('label', label)
  return request<Button>('/api/sounds', { method: 'POST', body: form })
}

export const addHotkey = (hotkey: NewHotkey) => request<Button>('/api/hotkeys', json('POST', hotkey))

export const getObsScenes = () => request<ObsSceneList>('/api/obs/scenes')

export const addObsScene = (scene: NewObsScene) => request<Button>('/api/obs/scenes', json('POST', scene))

export const getApps = () => request<AppList>('/api/apps')

export const addApp = (app: NewApp) => request<Button>('/api/apps', json('POST', app))

export const updateButton = (id: string, patch: ButtonPatch) =>
  request<Button>(`/api/buttons/${encodeURIComponent(id)}`, json('PATCH', patch))

export const deleteButton = (id: string) =>
  request<void>(`/api/buttons/${encodeURIComponent(id)}`, { method: 'DELETE' })
