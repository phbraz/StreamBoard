import { Component, type ReactNode } from 'react'

type State = { error: Error | null }

/** Keeps a render bug from blanking the whole board. */
export class ErrorBoundary extends Component<{ children: ReactNode }, State> {
  state: State = { error: null }

  static getDerivedStateFromError(error: Error): State {
    return { error }
  }

  render() {
    if (!this.state.error) return this.props.children
    return (
      <div className="flex min-h-full flex-col items-center justify-center gap-4 p-6 text-center">
        <p className="text-lg font-semibold">Something went wrong</p>
        <p className="max-w-md text-sm break-words text-board-muted">{this.state.error.message}</p>
        <button
          type="button"
          onClick={() => location.reload()}
          className="rounded-full bg-linear-to-r from-rose-500 to-orange-400 px-4 py-2 text-sm font-semibold text-white"
        >
          Reload
        </button>
      </div>
    )
  }
}
