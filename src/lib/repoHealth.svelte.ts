import { SvelteSet } from 'svelte/reactivity'
import { api } from './api'
import { session } from './session.svelte'
import { missingRepos } from './repoHealth'

/** Repo paths whose folder has vanished from disk — reactive, shared by App
 *  (startup fallback to the picker) and RepoSwitcher (dimmed row + Locate).
 *  Refreshed on every repo change. */
export const missingRepoPaths = new SvelteSet<string>()

/** Re-probe every known repo's folder and refresh {@link missingRepoPaths}.
 *  Best-effort: a probe that never answers just leaves the path present. */
export async function checkRepoHealth(): Promise<void> {
  const paths = session.config.recentRepos ?? []
  const entries = await Promise.all(paths.map(async (p) => [p, await api.pathExists(p)] as const))
  const gone = missingRepos(paths, new Map(entries))
  missingRepoPaths.clear()
  for (const p of gone) missingRepoPaths.add(p)
}
