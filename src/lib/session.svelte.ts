import { api } from './api'
import { toastError } from './toast'
import type { AppConfig } from './types'

// Shared reactive app state. `.svelte.ts` lets us use runes in a module.
export const session = $state({
  ready: false,
  signedIn: false,
  config: { serverUrl: null, currentRepo: null, recentRepos: [] } as AppConfig,
})

export async function bootstrap() {
  try {
    session.config = await api.loadConfig()
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
