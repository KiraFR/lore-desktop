import { api } from './api'
import { errorMessage } from './pushErrors'
import { toastError } from './toast'
import { CHECK_INTERVAL_MS, reduce, shouldCheck, type UpdateState } from './updater'

// In-app update cycle, shared by the StatusBar banner and Preferences ▸ Support.
// All transitions go through the pure machine in updater.ts.
export const updates = $state({
  state: { kind: 'idle' } as UpdateState,
  /** The running app's version, for the Preferences row (loaded once at start). */
  appVersion: null as string | null,
})

/** Short delay before the first silent check so startup work isn't contended. */
const BOOT_DELAY_MS = 4_000

let lastCheck: number | null = null
let started = false

async function runCheck(manual: boolean) {
  const next = reduce(updates.state, { type: 'check', manual })
  if (next.kind !== 'checking') return // the machine refused (busy, or banner already up)
  updates.state = next
  lastCheck = Date.now()
  try {
    const info = await api.checkForUpdate()
    updates.state = reduce(updates.state, info ? { type: 'found', version: info.version, notes: info.notes } : { type: 'none' })
  } catch (e) {
    // The machine routes it: auto → silently back to idle; manual → visible error.
    updates.state = reduce(updates.state, { type: 'failed', message: errorMessage(e) })
  }
}

/** Manual "Check for updates" (Preferences) — errors become visible there. */
export const checkNow = () => runCheck(true)

/** Download + install the available update; the real app relaunches at the end. */
export async function install() {
  const next = reduce(updates.state, { type: 'install' })
  if (next.kind !== 'downloading') return
  updates.state = next
  try {
    await api.installUpdate((pct) => {
      updates.state = reduce(updates.state, { type: 'progress', pct })
    })
    updates.state = reduce(updates.state, { type: 'installed' })
    // The real implementation relaunches inside installUpdate, so resolving
    // means we're in the mock — show "Restarting…" briefly, then reset so the
    // dev cycle can be replayed.
    setTimeout(() => { if (updates.state.kind === 'ready') updates.state = { kind: 'idle' } }, 1500)
  } catch (e) {
    updates.state = reduce(updates.state, { type: 'failed', message: errorMessage(e) })
    toastError("Couldn't install the update", e)
  }
}

/** Start the background cycle: version fetch, delayed boot check, then a 4 h
 *  timer plus a focus re-check once the interval has elapsed. Idempotent —
 *  App calls it once at mount. */
export function startUpdateCycle() {
  if (started) return
  started = true
  api.getAppVersion().then((v) => { updates.appVersion = v }).catch(() => { /* row stays blank */ })
  setTimeout(() => { void runCheck(false) }, BOOT_DELAY_MS)
  setInterval(() => { void runCheck(false) }, CHECK_INTERVAL_MS)
  window.addEventListener('focus', () => {
    if (shouldCheck(Date.now(), lastCheck)) void runCheck(false)
  })
}
