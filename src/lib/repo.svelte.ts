import { api } from './api'
import { session } from './session.svelte'
import { clearThumbs } from './thumbs.svelte'
import { toastError, toastAction, toastInfo } from './toast'
import { mergeOldSizes, sizeLookupPaths } from './oldSizes'
import { opProgress } from './opProgress.svelte'
import { isNonFastForwardPush, errorMessage } from './pushErrors'
import { setView } from './ui.svelte'
import type { Branch, Commit, LockEntry, StatusResult } from './types'

const HISTORY_PAGE = 200

// The current repository's status + in-flight action, shared by the title bar
// (branch, ahead/behind, sync, push) and the Changes view (files, commit).
export const repo = $state({
  status: null as StatusResult | null,
  busy: '' as '' | 'status' | 'commit' | 'push' | 'sync',
})

export const locks = $state({ list: [] as LockEntry[] })

// Branches are shared so `refreshStatus` can keep them warm — a `sync` may pull new
// remote branches, and a window-focus refresh picks up branches created elsewhere.
export const branches = $state({ list: [] as Branch[] })

// History is shared so switching views doesn't remount + reload it. Re-entering
// History shows the cached commits immediately and refreshes in the background;
// a commit/push/sync refreshes it without blanking the view.
export const history = $state({
  commits: [] as Commit[],
  cursor: undefined as string | null | undefined, // undefined = not loaded, null = end
  selectedId: null as string | null,
  loaded: false,
  repoPath: null as string | null,
})

let historyToken = 0

export async function refreshHistory(silent = false) {
  const path = session.config.currentRepo
  const token = ++historyToken
  if (!path) {
    Object.assign(history, { commits: [], cursor: undefined, selectedId: null, loaded: false, repoPath: null })
    return
  }
  if (history.repoPath !== path) {
    // New repo → drop the stale history so the loading state shows for the first load.
    Object.assign(history, { commits: [], cursor: undefined, selectedId: null, loaded: false, repoPath: path })
  }
  try {
    const page = await api.getHistory(path, HISTORY_PAGE)
    // A newer call (or a repo switch) superseded this one — drop the stale page.
    if (token !== historyToken || session.config.currentRepo !== path) return
    history.commits = page.commits
    history.cursor = page.nextCursor
    if (page.commits.length && (history.selectedId === null || !page.commits.some((c) => c.id === history.selectedId)))
      history.selectedId = page.commits[0].id
    history.loaded = true
  } catch (e) { if (!silent) toastError("Couldn't load history", e) }
}

export async function loadMoreHistory() {
  const path = session.config.currentRepo
  if (!path || !history.cursor) return
  const token = historyToken
  const page = await api.getHistory(path, HISTORY_PAGE, history.cursor)
  // A refresh/switch happened while paging — the appended page would be stale.
  if (token !== historyToken || session.config.currentRepo !== path) return
  history.commits = [...history.commits, ...page.commits]
  history.cursor = page.nextCursor
}

// Locks come from a remote query that can be slow/offline — callers refresh them
// in the background (`silent`) so a hung lock check never blocks the UI or spams
// toasts. Annotates the current status files with their lock holder.
export async function refreshLocks(silent = false) {
  const path = session.config.currentRepo
  if (!path) { locks.list = []; return }
  let lockList: LockEntry[]
  try { lockList = await api.getLocks(path) }
  catch (e) { if (!silent) toastError("Couldn't load locks", e); return }
  locks.list = lockList
  if (repo.status) {
    const holderByPath = new Map(lockList.map((l) => [l.path, l.holder]))
    repo.status.files = repo.status.files.map((f) => ({ ...f, lockedBy: holderByPath.get(f.path) ?? null }))
  }
}

export async function refreshBranches(silent = false) {
  const path = session.config.currentRepo
  if (!path) { branches.list = []; return }
  try { branches.list = await api.getBranches(path) }
  catch (e) { if (!silent) toastError("Couldn't load branches", e) }
}

// Fire-and-forget enrichment: fetch the repository-revision sizes of the
// modified/deleted files (ONE batch call) and merge them in as `oldSize`.
// Failure or timeout is TOTAL silence — the deltas simply don't appear.
// Never a toast for enrichment.
let sizesToken = 0
async function refreshFileSizes() {
  const token = ++sizesToken
  const path = session.config.currentRepo
  if (!path || !repo.status) return
  const paths = sizeLookupPaths(repo.status.files)
  if (paths.length === 0) return
  let sizes: Record<string, number>
  try { sizes = await api.fileSizes(path, paths) } catch { return }
  // The status may have been replaced while the sizes were in flight, or a
  // newer refreshFileSizes call may have already landed out of order — only
  // the latest call may annotate the current status (paths that vanished
  // are ignored by the merge).
  if (token === sizesToken && repo.status && session.config.currentRepo === path)
    repo.status.files = mergeOldSizes(repo.status.files, sizes)
}

// `silent` refreshes (e.g. the window-focus refresh) skip the `busy` flag so they
// don't disable the action buttons or flash a loading state.
export async function refreshStatus(silent = false) {
  const path = session.config.currentRepo
  if (!path) { repo.status = null; return }
  if (!silent) repo.busy = 'status'
  try {
    repo.status = await api.getStatus(path)
    // Only wipe row thumbnails on an EXPLICIT refresh (repo switch, post-commit/
    // /sync). The silent window-focus refresh must NOT clear them, or every
    // refocus blanks and re-decodes the whole change list (visible flicker, and
    // a `lore preview` re-shell per file in the real app). New files aren't in
    // the thumb map so they're still fetched; a same-path external content edit
    // keeps its old thumbnail until the next explicit refresh — an acceptable
    // trade for killing the per-focus flicker.
    if (!silent) clearThumbs()
  } catch (e) { toastError("Couldn't load changes", e) }
  finally { if (!silent) repo.busy = '' }
  // Locks + branches hit a remote query that can be slow/offline — fetch them in
  // the background so they NEVER hold the status render or disable the buttons.
  // They annotate / fill in when they arrive (or quietly no-op when offline).
  refreshLocks(true)
  refreshBranches(true)
  refreshFileSizes()
}

// Returns true when the action ran to completion (refresh included) — the
// Sync & push chain relies on it. `onError` may claim an error (resolve true)
// to replace the generic toastError with its own handling; it may be async
// (the non-FF detector may need a fresh status).
async function act(
  kind: 'commit' | 'push' | 'sync',
  run: (path: string) => Promise<void>,
  onError?: (e: unknown) => boolean | Promise<boolean>,
): Promise<boolean> {
  const path = session.config.currentRepo
  if (!path) return false
  repo.busy = kind
  try { await run(path) }
  catch (e) {
    if (!(await onError?.(e))) toastError(`${kind[0].toUpperCase()}${kind.slice(1)} failed`, e)
    repo.busy = ''
    return false
  }
  await refreshStatus()
  // commit/push/sync all change the history — refresh it in the background
  // (cached commits stay visible, no loading screen).
  refreshHistory(true)
  return true
}

export const commit = (message: string, exclude: string[] = []) =>
  act('commit', (p) => api.commitAll(p, message, exclude))
export const sync = () => act('sync', async (p) => {
  try { await api.sync(p, (prog) => { opProgress.sync = prog }) }
  finally { opProgress.sync = null }
})

export const syncToRevision = (revision: string) => act('sync', async (p) => {
  try { await api.syncToRevision(p, revision, (prog) => { opProgress.sync = prog }) }
  finally { opProgress.sync = null }
})

// Restore one file to an older revision as a pending change (LOCAL). Callers
// (FileHistorySection) only enable this when the tree is clean and we're at the
// tip — see restoreGuard. Lock handling per the design: free → acquire the lock
// so the change is committable; teammate-locked → restore anyway without locking
// (it lands in the excluded "Locked by teammates" section); mine → just restore.
export async function restoreFile(path: string, revision: string, lockHolder: string | null) {
  const p = session.config.currentRepo
  if (!p || repo.busy) return
  const teammateLocked = lockHolder != null && lockHolder !== 'you'
  const short = path.split(/[\\/]/).pop() ?? path
  repo.busy = 'sync'
  try {
    if (lockHolder == null) {
      // Free file → take the lock first so the restored change is committable.
      try {
        await api.setLock(p, path, true)
      } catch (e) {
        repo.busy = ''
        toastError("Couldn't lock the file — someone may hold it now", e)
        return
      }
    }
    try {
      await api.restoreFile(p, path, revision, (prog) => { opProgress.sync = prog })
    } finally {
      opProgress.sync = null
    }
  } catch (e) {
    repo.busy = ''
    toastError('Restore failed', e)
    return
  }
  await refreshStatus() // resets repo.busy in its finally; surfaces the pending change
  refreshLocks(true)
  setView('changes')
  toastInfo(
    teammateLocked
      ? `Restored ${short} — you can't commit it while someone else holds the lock`
      : `Restored ${short} — review and commit it in Changes`,
  )
}

// Push, then offer to release the locks the user held on files that were part of
// this push (the lock-workflow's "done editing" step). The candidate set must be
// computed BEFORE the push, while the remote and local tips still differ.
export const push = () => act('push', async (p) => {
  let candidates: string[] = []
  try { candidates = await api.pushedLockFiles(p) } catch { /* best-effort; never block the push */ }
  try { await api.push(p, (prog) => { opProgress.push = prog }) }
  finally { opProgress.push = null }
  if (candidates.length) {
    const n = candidates.length
    toastAction(`${n} locked file${n > 1 ? 's' : ''} pushed`, {
      label: 'Release locks',
      run: () => releaseLocks(candidates),
    })
  }
}, (e) => {
  // Non-fast-forward refusal (the remote advanced under us): offer the
  // sync-then-push chain instead of a dead-end "Push failed".
  if (!isNonFastForwardPush(errorMessage(e))) return false
  toastAction('Remote has new changes', { label: 'Sync & push', run: () => { void syncAndPush() } })
  refreshStatus(true) // silent: surface remoteAhead in the title bar
  return true
})

// The "Sync & push" toast action: sync, then push — UNLESS the sync failed
// (its own toast already showed) or left an UNRESOLVED merge. A clean catch-up
// sync auto-commits the merge and leaves us ahead, ready to push. We check
// stagedPending (revisionStaged non-zero) and NOT mergeInProgress: a committed
// merge keeps revisionMerged non-zero permanently (it's the merge's 2nd parent),
// so mergeInProgress stays true even with nothing to resolve — only a genuinely
// unresolved merge sets revisionStaged (verified against status_merge.ndjson).
export async function syncAndPush() {
  if (!(await sync())) return
  if (repo.status?.stagedPending) return
  await push()
}

export async function releaseLocks(paths: string[]) {
  const p = session.config.currentRepo
  if (!p) return
  for (const path of paths) {
    try { await api.setLock(p, path, false) }
    catch (e) { toastError('Unlock failed', e) }
  }
  await refreshStatus() // also refreshes locks in the background
}

export async function setLock(path: string, lock: boolean) {
  const p = session.config.currentRepo
  if (!p) return
  try { await api.setLock(p, path, lock) }
  catch (e) { toastError(lock ? 'Lock failed' : 'Unlock failed', e); return }
  await refreshStatus() // also refreshes locks in the background
}

export async function discardFile(path: string) {
  const p = session.config.currentRepo
  if (!p) return
  try { await api.discardFile(p, path) }
  catch (e) { toastError('Discard failed', e); return }
  await refreshStatus()
}

// Undo the last local commit: the tip moves back to `parentRevision` and the
// commit's changes return to the pending set (Changes).
export async function undoCommit(parentRevision: string) {
  const p = session.config.currentRepo
  if (!p) return
  try { await api.undoCommit(p, parentRevision) }
  catch (e) { toastError('Undo failed', e); return }
  await refreshStatus()
  await refreshHistory(true)
}
