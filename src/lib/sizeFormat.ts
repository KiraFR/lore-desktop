import type { ChangedFile } from './types'

const KB = 1024
const MB = 1024 * 1024

export function fmtSize(n: number): string {
  if (n >= MB) return (n / MB).toFixed(1) + ' MB'
  if (n >= KB) return (n / KB).toFixed(1) + ' KB'
  return n + ' B'
}

/**
 * Compact end-of-row size annotation for the Changes list.
 * - modify: signed delta ("+0.3 MB" / "−0.1 MB"), null when unknown or zero —
 *   neutral secondary color in the UI (growing is not a fault).
 * - delete: the old size alone ("2.0 MB"), no arrow, no sign.
 * - add/move/copy: null (unchanged from today).
 */
export function formatDelta(f: Pick<ChangedFile, 'action' | 'size' | 'oldSize'>): string | null {
  if (f.action === 'delete') return f.oldSize != null ? fmtSize(f.oldSize) : null
  if (f.action !== 'modify' || f.oldSize == null) return null
  const delta = f.size - f.oldSize
  if (delta === 0) return null
  return (delta > 0 ? '+' : '−') + fmtSize(Math.abs(delta))
}
