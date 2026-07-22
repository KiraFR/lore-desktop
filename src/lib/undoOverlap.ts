import type { CommitFile } from './types'

/** Max overlapping paths listed verbatim in the overwrite warning; the rest
 *  collapse into "and N more". */
export const MAX_LISTED_OVERLAPS = 5

/** Paths present BOTH in the commit being undone and in the pending change set
 *  (commit order preserved). These are the files whose pending modification the
 *  undo would overwrite. */
export function overlappingPaths(commitFiles: Pick<CommitFile, 'path'>[], pendingPaths: string[]): string[] {
  const pending = new Set(pendingPaths)
  return commitFiles.map((f) => f.path).filter((p) => pending.has(p))
}

/** The undo confirmation text: the usual undo message, preceded — when pending
 *  changes would be overwritten — by an explicit warning listing up to
 *  MAX_LISTED_OVERLAPS paths ("and N more" beyond that). */
export function undoConfirmMessage(commitMessage: string, overlap: string[]): string {
  const base = `Undo the commit "${commitMessage}"? Its changes go back to Changes (nothing is lost).`
  if (overlap.length === 0) return base
  const listed = overlap.slice(0, MAX_LISTED_OVERLAPS).join(', ')
  const rest = overlap.length - MAX_LISTED_OVERLAPS
  const more = rest > 0 ? ` and ${rest} more` : ''
  const s = overlap.length === 1 ? '' : 's'
  return `Undoing this commit will overwrite your pending changes to ${overlap.length} file${s}: ${listed}${more}.\n\n${base}`
}
