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

/** Case-insensitive substring filter on the path; blank query = everything. */
export function filterByQuery(files: ChangedFile[], query: string): ChangedFile[] {
  const q = query.trim().toLowerCase()
  if (!q) return files
  return files.filter((f) => f.path.toLowerCase().includes(q))
}
