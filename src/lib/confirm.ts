import { toastError } from './toast'

/** A confirmation dialog that works both in Tauri (native dialog) and the browser
 *  mock (`window.confirm`). Returns true when the user confirms. */
export async function confirmAction(message: string, title = 'Confirm'): Promise<boolean> {
  if ('__TAURI_INTERNALS__' in window) {
    try {
      const { ask } = await import('@tauri-apps/plugin-dialog')
      return await ask(message, { title, kind: 'warning' })
    } catch (e) {
      // A missing dialog capability must never silently swallow the action.
      toastError('Confirmation dialog failed', e)
      return false
    }
  }
  return window.confirm(message)
}
