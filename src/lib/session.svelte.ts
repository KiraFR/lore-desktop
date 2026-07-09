import { api } from './api'
import { toastError } from './toast'
import { promoteRepo, removeRepoPath, nextCurrentRepo } from './repoList'
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
    let config = await api.loadConfig()
    // A signed-in user shouldn't have to re-pick a server; default it when the
    // stored config has none so we go straight to the repo picker.
    if (!config.serverUrl) config = { ...config, serverUrl: DEFAULT_SERVER_URL }
    // Older configs set currentRepo without ever populating recentRepos; make
    // sure the open repo always appears in the known-repos list.
    if (config.currentRepo && !config.recentRepos.includes(config.currentRepo)) {
      config = { ...config, recentRepos: promoteRepo(config.recentRepos, config.currentRepo) }
    }
    session.config = config
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

/** Switch to (or add) a repo: set it current and move it to the front of the known list. */
export async function selectRepo(repoPath: string) {
  session.config = {
    ...session.config,
    currentRepo: repoPath,
    recentRepos: promoteRepo(session.config.recentRepos, repoPath),
  }
  await api.saveConfig(session.config)
}

/** Forget a repo (files stay on disk). If it was current, fall back to the next most recent. */
export async function removeRepo(repoPath: string) {
  const recent = removeRepoPath(session.config.recentRepos, repoPath)
  session.config = {
    ...session.config,
    currentRepo: nextCurrentRepo(session.config.currentRepo, repoPath, recent),
    recentRepos: recent,
  }
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
