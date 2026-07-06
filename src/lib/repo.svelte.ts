import { api } from './api'
import { session } from './session.svelte'
import { toastError } from './toast'
import type { LockEntry, StatusResult } from './types'

// The current repository's status + in-flight action, shared by the title bar
// (branch, ahead/behind, sync, push) and the Changes view (files, commit).
export const repo = $state({
  status: null as StatusResult | null,
  busy: '' as '' | 'status' | 'commit' | 'push' | 'sync',
})

export const locks = $state({ list: [] as LockEntry[] })

export async function refreshLocks() {
  const path = session.config.currentRepo
  if (!path) { locks.list = []; return }
  try { locks.list = await api.getLocks(path) }
  catch (e) { toastError("Couldn't load locks", e) }
}

export async function refreshStatus() {
  const path = session.config.currentRepo
  if (!path) { repo.status = null; return }
  repo.busy = 'status'
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
  } catch (e) { toastError("Couldn't load changes", e) }
  finally { repo.busy = '' }
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
export const push = () => act('push', (p) => api.push(p))
export const sync = () => act('sync', (p) => api.sync(p))

export async function setLock(path: string, lock: boolean) {
  const p = session.config.currentRepo
  if (!p) return
  try { await api.setLock(p, path, lock) }
  catch (e) { toastError(lock ? 'Lock failed' : 'Unlock failed', e); return }
  await refreshStatus()
  await refreshLocks()
}
