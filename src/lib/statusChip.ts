import type { StatusResult } from './types'

export type StatusChip = { kind: 'merge' } | { kind: 'staged' } | null

/**
 * Which StatusBar chip to show. Merge takes precedence: a merge implies a
 * staged state, so the staged chip is hidden while a merge is in progress.
 */
export function chipFor(status: StatusResult | null): StatusChip {
  if (!status) return null
  if (status.mergeInProgress) return { kind: 'merge' }
  if (status.stagedPending) return { kind: 'staged' }
  return null
}
