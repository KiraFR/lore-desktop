/**
 * Pure state machine of the in-app update cycle (auto-update spec 2026-07-22).
 * The runes store (updates.svelte.ts) owns the timers and the API calls and
 * dispatches events here; keeping the transitions pure makes the silent-vs-
 * manual error routing and the re-check interval testable without Svelte.
 */

export type UpdateState =
  | { kind: 'idle' }
  | { kind: 'checking'; manual: boolean }
  | { kind: 'available'; version: string; notes: string }
  | { kind: 'downloading'; version: string; pct: number }
  | { kind: 'ready'; version: string }
  | { kind: 'upToDate' }
  | { kind: 'error'; message: string }

export type UpdateEvent =
  | { type: 'check'; manual: boolean }
  | { type: 'found'; version: string; notes: string }
  | { type: 'none' }
  | { type: 'install' }
  | { type: 'progress'; pct: number }
  | { type: 'installed' }
  | { type: 'failed'; message: string }

/** One transition. Events that don't apply to the current state are ignored
 *  (the state comes back unchanged) so late/stale events can never corrupt an
 *  in-flight download. */
export function reduce(s: UpdateState, e: UpdateEvent): UpdateState {
  switch (e.type) {
    case 'check':
      // Never interrupt an in-flight check, download, or restart; an AUTO
      // re-check also never yanks an already-visible banner (a manual one
      // from Preferences may re-confirm it).
      if (s.kind === 'checking' || s.kind === 'downloading' || s.kind === 'ready') return s
      if (!e.manual && s.kind === 'available') return s
      return { kind: 'checking', manual: e.manual }
    case 'found':
      return s.kind === 'checking' ? { kind: 'available', version: e.version, notes: e.notes } : s
    case 'none':
      return s.kind === 'checking' ? { kind: 'upToDate' } : s
    case 'install':
      return s.kind === 'available' ? { kind: 'downloading', version: s.version, pct: 0 } : s
    case 'progress':
      return s.kind === 'downloading' ? { ...s, pct: e.pct } : s
    case 'installed':
      return s.kind === 'downloading' ? { kind: 'ready', version: s.version } : s
    case 'failed':
      // A failed AUTO check stays silent (offline is a normal state); a failed
      // MANUAL check or a failed install surfaces the error.
      if (s.kind === 'checking' && !s.manual) return { kind: 'idle' }
      return { kind: 'error', message: e.message }
  }
}

/** Automatic re-check cadence: at boot, then every 4 h (and on window focus
 *  once the interval has elapsed). */
export const CHECK_INTERVAL_MS = 4 * 60 * 60 * 1000

/** Whether enough time has passed for a new automatic check. `lastCheck` is
 *  null before the first check ever. Both are epoch-ms parameters so callers
 *  (and tests) inject the clock — no Date.now() buried here. */
export function shouldCheck(now: number, lastCheck: number | null, intervalMs = CHECK_INTERVAL_MS): boolean {
  return lastCheck === null || now - lastCheck >= intervalMs
}

/** Percentage of a byte download, clamped to 0..100; null while the total is
 *  unknown (the updater's Started event may lack a content length). */
export function progressPct(gotBytes: number, totalBytes: number): number | null {
  if (!Number.isFinite(totalBytes) || totalBytes <= 0) return null
  return Math.max(0, Math.min(100, Math.round((gotBytes / totalBytes) * 100)))
}
