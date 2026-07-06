import { api } from './api'
import { session } from './session.svelte'
import { toastError, toastAction } from './toast'
import type { Branch, LockEntry, StatusResult } from './types'

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

export async function refreshLocks() {
  const path = session.config.currentRepo
  if (!path) { locks.list = []; return }
  try { locks.list = await api.getLocks(path) }
  catch (e) { toastError("Couldn't load locks", e) }
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
    const status = await api.getStatus(path)
    // Locks are a separate query; annotate each file's holder so the Changes /
    // preview lock toggle reflects reality. Best-effort — a lock-query failure
    // must not hide the status.
    let lockList: LockEntry[] = []
    try { lockList = await api.getLocks(path) } catch { /* ignore */ }
    const holderByPath = new Map(lockList.map((l) => [l.path, l.holder]))
    status.files = status.files.map((f) => ({ ...f, lockedBy: holderByPath.get(f.path) ?? null }))
    repo.status = status
    locks.list = lockList
    // Best-effort, decoupled from the status paint: refresh the branch list at the
    // same trigger points (focus, sync, push, commit, repo change).
    refreshBranches(true)
  } catch (e) { toastError("Couldn't load changes", e) }
  finally { if (!silent) repo.busy = '' }
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
}

export const commit = (message: string) => act('commit', (p) => api.commitAll(p, message))
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
  await refreshStatus()
  await refreshLocks()
}

export async function setLock(path: string, lock: boolean) {
  const p = session.config.currentRepo
  if (!p) return
  try { await api.setLock(p, path, lock) }
  catch (e) { toastError(lock ? 'Lock failed' : 'Unlock failed', e); return }
  await refreshStatus()
  await refreshLocks()
}
