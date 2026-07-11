import type { ChangedFile } from './types'

export interface ChangesPartition {
  /** Files that can be committed: unlocked, or locked by the current user. */
  committable: ChangedFile[]
  /** Files locked by a teammate — excluded from commit by construction. */
  lockedByOthers: ChangedFile[]
}

export function partitionByLock(files: ChangedFile[]): ChangesPartition {
  const committable: ChangedFile[] = []
  const lockedByOthers: ChangedFile[] = []
  for (const f of files) {
    if (f.lockedBy && f.lockedBy !== 'you') lockedByOthers.push(f)
    else committable.push(f)
  }
  return { committable, lockedByOthers }
}

/** Case-insensitive substring filter over the text fields `haystack` extracts
 *  from each item (e.g. path + lock holder); blank query = everything. */
export function filterByText<T>(items: T[], query: string, haystack: (item: T) => string[]): T[] {
  const q = query.trim().toLowerCase()
  if (!q) return items
  return items.filter((it) => haystack(it).some((s) => s.toLowerCase().includes(q)))
}

/** Case-insensitive substring filter on the path; blank query = everything. */
export function filterByQuery(files: ChangedFile[], query: string): ChangedFile[] {
  return filterByText(files, query, (f) => [f.path])
}
