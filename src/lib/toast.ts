import { writable } from 'svelte/store'

export interface Toast {
  id: number
  title: string
  /** Optional detail (the underlying error text); '' when absent. */
  message: string
}

/** Active toasts, newest last. A plain writable store so it is reactive in
 *  components and unit-testable without the Svelte compiler. */
export const toasts = writable<Toast[]>([])

let nextId = 1
const TOAST_TTL = 6000

/** Push a red error toast: a short title plus optional detail from `err`.
 *  Returns the toast id so callers/tests can dismiss it early. */
export function toastError(title: string, err?: unknown): number {
  const message = err === undefined ? '' : err instanceof Error ? err.message : String(err)
  const id = nextId++
  toasts.update((list) => [...list, { id, title, message }])
  setTimeout(() => dismissToast(id), TOAST_TTL)
  return id
}

export function dismissToast(id: number): void {
  toasts.update((list) => list.filter((t) => t.id !== id))
}
