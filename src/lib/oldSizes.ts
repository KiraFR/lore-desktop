import type { ChangedFile } from './types'

/** Only modify/delete rows have an "old" (repository-revision) size. */
const hasOldSide = (f: ChangedFile) => f.action === 'modify' || f.action === 'delete'

/** The paths worth a `file info` lookup: only modify/delete have an "old" size. */
export function sizeLookupPaths(files: ChangedFile[]): string[] {
  return files.filter(hasOldSide).map((f) => f.path)
}

/**
 * Merge repository-revision sizes into the change list as `oldSize`.
 * Only modify/delete rows are enriched; paths missing from `sizes` (fetch
 * failed for that file, or the file vanished between status and file info)
 * are left untouched. Pure — used by the fire-and-forget enrichment.
 */
export function mergeOldSizes(files: ChangedFile[], sizes: Record<string, number>): ChangedFile[] {
  return files.map((f) =>
    hasOldSide(f) && Object.hasOwn(sizes, f.path)
      ? { ...f, oldSize: sizes[f.path] }
      : f,
  )
}
