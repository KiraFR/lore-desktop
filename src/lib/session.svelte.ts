import { api } from './api'
import { toastError } from './toast'
import type { AppConfig } from './types'

/** The studio's Lore server; used as the default when no server is stored yet. */
export const DEFAULT_SERVER_URL = 'lore://lore.example.com:41337'

// Shared reactive app state. `.svelte.ts` lets us use runes in a module.
export const session = $state({
  ready: false,
  signedIn: false,
  config: { serverUrl: null, currentRepo: null, recentRepos: [] } as AppConfig,
})

export async function bootstrap() {
  try {
    const config = await api.loadConfig()
    // A signed-in user shouldn't have to re-pick a server; default it when the
    // stored config has none so we go straight to the repo picker.
    session.config = config.serverUrl ? config : { ...config, serverUrl: DEFAULT_SERVER_URL }
    session.signedIn = await api.isAuthenticated()
  } catch (e) {
    toastError('Startup failed', e)
  } finally {
    session.ready = true
  }
}

export async function setSignedIn(serverUrl: string) {
  session.config = { ...session.config, serverUrl }
  await api.saveConfig(session.config)
  session.signedIn = true
}

export async function selectRepo(repoPath: string) {
  const recent = [repoPath, ...session.config.recentRepos.filter((r) => r !== repoPath)].slice(0, 10)
  session.config = { ...session.config, currentRepo: repoPath, recentRepos: recent }
  await api.saveConfig(session.config)
}

export async function clearCurrentRepo() {
  session.config = { ...session.config, currentRepo: null }
  await api.saveConfig(session.config)
}

export async function signOut() {
  await api.signOut()
  session.signedIn = false
}
