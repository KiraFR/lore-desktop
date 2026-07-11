import { api } from './api'
import { session, selectRepo } from './session.svelte'
import { toastError } from './toast'
import { opProgress } from './opProgress.svelte'
import { cloneInFlight } from './progress'
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
 *
 * Global anti-double-clone guard: one clone at a time across ALL surfaces
 * (RepoPicker and RepoSwitcher can coexist and used to race each other into
 * the single `opProgress.clone` slot). The slot doubles as the flag — a
 * `{ done: 0 }` sentinel occupies it from before the folder pick, so a second
 * surface can't slip in while the dialog is open.
 */
export async function cloneServerRepo(entry: RepoEntry): Promise<boolean> {
  if (cloneInFlight(opProgress.clone)) return false
  opProgress.clone = { done: 0 } // indeterminate until the first real tick
  try {
    const parent = await api.pickFolder()
    if (!parent) return false // cancelled
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
