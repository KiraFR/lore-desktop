import type { OpProgress } from './types'

/** 0-100 (clamped) when a total is known, null for indeterminate progress. */
export function pct(p: OpProgress | null): number | null {
  if (!p || !p.total || p.total <= 0) return null
  return Math.min(100, Math.round((p.done / p.total) * 100))
}

/** Button label for an in-flight clone. */
export function cloneLabel(percent: number | null): string {
  return percent === null ? 'Cloning…' : `Cloning… ${percent}%`
}
