import type { StatusResult } from './types'

export type StatusChip =
  | { kind: 'merge' }
  | { kind: 'staged' }
  | { kind: 'behind'; revision: number }
  | null

/**
 * Which StatusBar chip to show. Merge takes precedence (a merge implies a
 * staged state); behind comes last — a normal state after a sync-to-revision.
 * "Behind" = the working copy sits on a past revision (revisionNumber below the
 * local head), not the remote-ahead of a teammate push.
 */
export function chipFor(status: StatusResult | null): StatusChip {
  if (!status) return null
  if (status.mergeInProgress) return { kind: 'merge' }
  if (status.stagedPending) return { kind: 'staged' }
  if (status.revisionNumber < status.localRevisionNumber) return { kind: 'behind', revision: status.revisionNumber }
  return null
}
