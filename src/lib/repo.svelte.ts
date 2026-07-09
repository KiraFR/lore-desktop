import { api } from './api'
import { session } from './session.svelte'
import { clearThumbs } from './thumbs.svelte'
import { toastError, toastAction } from './toast'
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

export async function refreshHistory(silent = false) {
  const path = session.config.currentRepo
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
  const page = await api.getHistory(path, HISTORY_PAGE, history.cursor)
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

// `silent` refreshes (e.g. the window-focus refresh) skip the `busy` flag so they
// don't disable the action buttons or flash a loading state.
export async function refreshStatus(silent = false) {
  const path = session.config.currentRepo
  if (!path) { repo.status = null; return }
  if (!silent) repo.busy = 'status'
  try {
    repo.status = await api.getStatus(path)
    clearThumbs() // files may have changed on disk — row thumbnails re-resolve via the mtime-keyed cache
  } catch (e) { toastError("Couldn't load changes", e) }
  finally { if (!silent) repo.busy = '' }
  // Locks + branches hit a remote query that can be slow/offline — fetch them in
  // the background so they NEVER hold the status render or disable the buttons.
  // They annotate / fill in when they arrive (or quietly no-op when offline).
  refreshLocks(true)
  refreshBranches(true)
}

async function act(kind: 'commit' | 'push' | 'sync', run: (path: string) => Promise<void>) {
  const path = session.config.currentRepo
  if (!path) return
  repo.busy = kind
  try { await run(path) }
  catch (e) {
    toastError(`${kind[0].toUpperCase()}${kind.slice(1)} failed`, e)
    repo.busy = ''
    return
  }
  await refreshStatus()
  // commit/push/sync all change the history — refresh it in the background
  // (cached commits stay visible, no loading screen).
  refreshHistory(true)
}

export const commit = (message: string, exclude: string[] = []) =>
  act('commit', (p) => api.commitAll(p, message, exclude))
export const sync = () => act('sync', (p) => api.sync(p))

// Push, then offer to release the locks the user held on files that were part of
// this push (the lock-workflow's "done editing" step). The candidate set must be
// computed BEFORE the push, while the remote and local tips still differ.
export const push = () => act('push', async (p) => {
  let candidates: string[] = []
  try { candidates = await api.pushedLockFiles(p) } catch { /* best-effort; never block the push */ }
  await api.push(p)
  if (candidates.length) {
    const n = candidates.length
    toastAction(`${n} locked file${n > 1 ? 's' : ''} pushed`, {
      label: 'Release locks',
      run: () => releaseLocks(candidates),
    })
  }
})

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
