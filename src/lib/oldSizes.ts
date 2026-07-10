import type { ChangedFile } from './types'

/** The paths worth a `file info` lookup: only modify/delete have an "old" size. */
export function sizeLookupPaths(files: ChangedFile[]): string[] {
  return files.filter((f) => f.action === 'modify' || f.action === 'delete').map((f) => f.path)
}

/**
 * Merge repository-revision sizes into the change list as `oldSize`.
 * Only modify/delete rows are enriched; paths missing from `sizes` (fetch
 * failed for that file, or the file vanished between status and file info)
 * are left untouched. Pure — used by the fire-and-forget enrichment.
 */
export function mergeOldSizes(files: ChangedFile[], sizes: Record<string, number>): ChangedFile[] {
  return files.map((f) =>
    (f.action === 'modify' || f.action === 'delete') && sizes[f.path] != null
      ? { ...f, oldSize: sizes[f.path] }
      : f,
  )
}
