/** Click/Enter on a commit-file row: the selected path toggles closed, any
 *  other path becomes the selection. Local to the History view — deliberately
 *  NOT a global store (spec: reset on commit change, gone on view leave). */
export function toggleFilePath(current: string | null, clicked: string): string | null {
  return current === clicked ? null : clicked
}

/** Selection surviving a detail refetch: only a same-commit refresh keeps it. */
export function selectionAfterCommitChange(sameCommit: boolean, current: string | null): string | null {
  return sameCommit ? current : null
}

/** True when `commitId` is the local tip (history is newest-first). Drives the
 *  « Preview of the current working copy » caveat: without `file cat <rev>`,
 *  any non-tip commit can only show the disk's current state. */
export function isLocalTip(commitId: string, commits: { id: string }[]): boolean {
  return commits[0]?.id === commitId
}
