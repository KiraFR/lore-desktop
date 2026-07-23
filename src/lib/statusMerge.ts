import type { ChangedFile, StatusResult } from './types'

// Structural stability for status refreshes: every refresh (window focus,
// coalesced notifications, post-action) replaces `repo.status` with a freshly
// parsed object — brand-new ChangedFile identities even when NOTHING changed.
// Every $effect keyed on the selected file's identity then re-runs: diff
// refetch, media preview re-fetch (visible flash), file-history refetch.
// `mergeStatus` reuses the PREVIOUS objects wherever the values are unchanged,
// so a no-op refresh invalidates nothing downstream.

/** Reuse `prev` when the row is unchanged. `oldSize`/`lockedBy` are
 *  enrichments merged in AFTER the status lands (fileSizes, locks) — a fresh
 *  status that omits them (undefined) must not wipe the previous values;
 *  an explicit value (null included for lockedBy) is authoritative. */
function mergeFile(prev: ChangedFile, next: ChangedFile): ChangedFile {
  const oldSize = next.oldSize !== undefined ? next.oldSize : prev.oldSize
  const lockedBy = next.lockedBy !== undefined ? next.lockedBy : prev.lockedBy
  const same =
    prev.action === next.action &&
    prev.isBinary === next.isBinary &&
    prev.size === next.size &&
    prev.oldSize === oldSize &&
    (prev.lockedBy ?? null) === (lockedBy ?? null)
  if (same) return prev
  const out: ChangedFile = { ...next }
  if (oldSize !== undefined) out.oldSize = oldSize
  if (lockedBy !== undefined) out.lockedBy = lockedBy
  return out
}

/** Merge `next`'s file list over `prev`'s, keyed by path: unchanged rows keep
 *  their previous object identity, and when every row (and the order) is
 *  unchanged the previous ARRAY itself is returned. */
export function mergeStatusFiles(prev: ChangedFile[], next: ChangedFile[]): ChangedFile[] {
  const byPath = new Map(prev.map((f) => [f.path, f]))
  let allSame = prev.length === next.length
  const out = next.map((nf, i) => {
    const pf = byPath.get(nf.path)
    const merged = pf ? mergeFile(pf, nf) : nf
    if (allSame && merged !== prev[i]) allSame = false
    return merged
  })
  return allSame ? prev : out
}

const sameSummary = (a: StatusResult['summary'], b: StatusResult['summary']) =>
  a === b || (a != null && b != null && a.adds === b.adds && a.mods === b.mods && a.dels === b.dels)

/** Fold a fresh status over the previous one, preserving unchanged references
 *  at every level — up to returning `prev` itself for a fully no-op refresh. */
export function mergeStatus(prev: StatusResult | null, next: StatusResult): StatusResult {
  if (!prev) return next
  const files = mergeStatusFiles(prev.files, next.files)
  const same =
    files === prev.files &&
    sameSummary(prev.summary, next.summary) &&
    prev.branch === next.branch &&
    prev.localAhead === next.localAhead &&
    prev.remoteAhead === next.remoteAhead &&
    prev.revisionNumber === next.revisionNumber &&
    prev.localRevisionNumber === next.localRevisionNumber &&
    prev.remoteAvailable === next.remoteAvailable &&
    prev.remoteAuthorized === next.remoteAuthorized &&
    prev.mergeInProgress === next.mergeInProgress &&
    prev.stagedPending === next.stagedPending &&
    prev.ignoredCount === next.ignoredCount
  return same ? prev : { ...next, files }
}

/** Annotate each row's `lockedBy` from the lock list, preserving the object
 *  identity of rows whose holder is unchanged (and the array itself when no
 *  row changed). Absent map entry = not locked (null), like before. */
export function annotateLocks(files: ChangedFile[], holderByPath: Map<string, string>): ChangedFile[] {
  let changed = false
  const out = files.map((f) => {
    const holder = holderByPath.get(f.path) ?? null
    if ((f.lockedBy ?? null) === holder) return f
    changed = true
    return { ...f, lockedBy: holder }
  })
  return changed ? out : files
}
