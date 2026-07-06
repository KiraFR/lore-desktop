import { writable } from 'svelte/store'

export interface ToastAction {
  label: string
  run: () => void
}

export interface Toast {
  id: number
  title: string
  /** Optional detail (the underlying error text); '' when absent. */
  message: string
  /** 'error' = red (default), 'info' = accent (used for actionable prompts). */
  variant: 'error' | 'info'
  /** Optional action button (e.g. "Release locks"). */
  action?: ToastAction
}

/** Active toasts, newest last. A plain writable store so it is reactive in
 *  components and unit-testable without the Svelte compiler. */
export const toasts = writable<Toast[]>([])

let nextId = 1
const ERROR_TTL = 6000
const ACTION_TTL = 15000

/** Push a red error toast: a short title plus optional detail from `err`.
 *  Returns the toast id so callers/tests can dismiss it early. */
export function toastError(title: string, err?: unknown): number {
  const message = err === undefined ? '' : err instanceof Error ? err.message : String(err)
  const id = nextId++
  toasts.update((list) => [...list, { id, title, message, variant: 'error' }])
  setTimeout(() => dismissToast(id), ERROR_TTL)
  return id
}

/** Push a neutral (accent) toast with an action button. Longer-lived than an
 *  error toast so the user has time to act. */
export function toastAction(title: string, action: ToastAction): number {
  const id = nextId++
  toasts.update((list) => [...list, { id, title, message: '', variant: 'info', action }])
  setTimeout(() => dismissToast(id), ACTION_TTL)
  return id
}

export function dismissToast(id: number): void {
  toasts.update((list) => list.filter((t) => t.id !== id))
}
