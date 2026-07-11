/** Pure helpers for the known-repos list (`config.recentRepos`, MRU-first). */

/** Display name for a repo path: the folder basename. */
export function repoName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path
}

/** Move `path` to the front, prepending it if new — most-recently-used ordering. */
export function promoteRepo(list: string[], path: string): string[] {
  return [path, ...list.filter((r) => r !== path)]
}

/** Drop `path` from the list (does not touch files on disk). */
export function removeRepoPath(list: string[], path: string): string[] {
  return list.filter((r) => r !== path)
}

/** Swap a repo path for its new location (after a move), keeping list order.
 *  Dedups if `newPath` was already present, so the list never gains a twin. */
export function replaceRepoPath(list: string[], oldPath: string, newPath: string): string[] {
  const out = list.map((p) => (p === oldPath ? newPath : p))
  return out.filter((p, i) => out.indexOf(p) === i)
}

/** Current repo after removing `removed`: unchanged unless it *was* current, then the most recent remaining repo (or null). */
export function nextCurrentRepo(current: string | null, removed: string, remaining: string[]): string | null {
  return current === removed ? (remaining[0] ?? null) : current
}

/** Case-insensitive live filter over the full path (the name is part of the path). */
export function filterRepos(list: string[], query: string): string[] {
  const q = query.trim().toLowerCase()
  if (!q) return list
  return list.filter((p) => p.toLowerCase().includes(q))
}
