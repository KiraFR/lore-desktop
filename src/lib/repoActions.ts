import { api } from './api'
import { session, selectRepo } from './session.svelte'
import { toastError } from './toast'
import { opProgress } from './opProgress.svelte'
import type { RepoEntry } from './types'

/**
 * Pick a local folder, validate it is a Lore working copy (has `.lore/`, which
 * `getStatus` checks), then add it to the known list and switch to it.
 * Returns true when the app switched repos.
 */
export async function addExistingRepo(): Promise<boolean> {
  const path = await api.pickFolder()
  if (!path) return false // cancelled
  try {
    await api.getStatus(path)
    await selectRepo(path)
    return true
  } catch (e) {
    toastError('Not a Lore repository', e)
    return false
  }
}

/**
 * Pick a destination parent folder, clone the server repo into it, then add it
 * to the known list and switch to it. Returns true when the app switched repos.
 */
export async function cloneServerRepo(entry: RepoEntry): Promise<boolean> {
  const parent = await api.pickFolder()
  if (!parent) return false // cancelled
  try {
    const path = await api.cloneRepo(
      session.config.serverUrl!, entry.id, entry.name, parent,
      (p) => { opProgress.clone = p },
    )
    await selectRepo(path)
    return true
  } catch (e) {
    toastError('Clone failed', e)
    return false
  } finally {
    opProgress.clone = null
  }
}
