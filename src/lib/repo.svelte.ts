import { api } from './api'
import { session } from './session.svelte'
import type { LockEntry, StatusResult } from './types'

// The current repository's status + in-flight action, shared by the title bar
// (branch, ahead/behind, sync, push) and the Changes view (files, commit).
export const repo = $state({
  status: null as StatusResult | null,
  busy: '' as '' | 'status' | 'commit' | 'push' | 'sync',
  error: '',
})

export const locks = $state({ list: [] as LockEntry[] })

export async function refreshLocks() {
  const path = session.config.currentRepo
  if (!path) { locks.list = []; return }
  locks.list = await api.getLocks(path)
}

export async function refreshStatus() {
  const path = session.config.currentRepo
  if (!path) { repo.status = null; return }
  repo.error = ''; repo.busy = 'status'
  try { repo.status = await api.getStatus(path) }
  catch (e) { repo.error = String(e) }
  finally { repo.busy = '' }
}

async function act(kind: 'commit' | 'push' | 'sync', run: (path: string) => Promise<void>) {
  const path = session.config.currentRepo
  if (!path) return
  repo.error = ''; repo.busy = kind
  try { await run(path) }
  catch (e) { repo.error = String(e); repo.busy = ''; return }
  await refreshStatus()
}

export const commit = (message: string) => act('commit', (p) => api.commitAll(p, message))
export const push = () => act('push', (p) => api.push(p))
export const sync = () => act('sync', (p) => api.sync(p))

export async function setLock(path: string, lock: boolean) {
  const p = session.config.currentRepo
  if (!p) return
  repo.error = ''
  try { await api.setLock(p, path, lock) }
  catch (e) { repo.error = String(e); return }
  await refreshStatus()
  await refreshLocks()
}
