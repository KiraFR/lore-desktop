/** A confirmation dialog that works both in Tauri (native dialog) and the browser
 *  mock (`window.confirm`). Returns true when the user confirms. */
export async function confirmAction(message: string, title = 'Confirm'): Promise<boolean> {
  if ('__TAURI_INTERNALS__' in window) {
    const { ask } = await import('@tauri-apps/plugin-dialog')
    return ask(message, { title, kind: 'warning' })
  }
  return window.confirm(message)
}
