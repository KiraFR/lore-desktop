import type { OpProgress } from './types'
import { fmtSize } from './sizeFormat'

/** 0-100 (clamped) when a total is known, null for indeterminate progress. */
export function pct(p: OpProgress | null): number | null {
  if (!p || !p.total || p.total <= 0) return null
  return Math.min(100, Math.round((p.done / p.total) * 100))
}

/** Button label for an in-flight clone. */
export function cloneLabel(percent: number | null): string {
  return percent === null ? 'Cloning…' : `Cloning… ${percent}%`
}

/** Full clone label: « Cloning… 42% — 12.0 MB / 48.0 MB » when the progress is
 *  byte-counted with a known total; plain percentage/indeterminate otherwise.
 *  Sync/push deliberately keep a bar without text (P1 decision, see TitleBar). */
export function cloneProgressLabel(p: OpProgress | null): string {
  const percent = pct(p)
  if (percent === null || !p || p.unit !== 'bytes' || !p.total) return cloneLabel(percent)
  return `${cloneLabel(percent)} — ${fmtSize(p.done)} / ${fmtSize(p.total)}`
}

/** Global anti-double-clone guard: any occupied slot means a clone is in
 *  flight somewhere (RepoPicker or RepoSwitcher). repoActions parks a
 *  `{ done: 0 }` sentinel in the slot before the folder pick, so the guard
 *  holds even before the first real tick. */
export function cloneInFlight(p: OpProgress | null): boolean {
  return p !== null
}
