import type { AppConfig, ChangedFile, LoreApi, RepoEntry, StatusResult } from './types'

const delay = (ms = 350) => new Promise((r) => setTimeout(r, ms))
const CONFIG_KEY = 'loredesktop.config'
const AUTH_KEY = 'loredesktop.signedin'

const FAKE_REPOS: RepoEntry[] = [
  { id: '019f2e14006f7870a7b27df367c78b72', name: 'game-main' },
  { id: '019f2e1577257382bc89c5a28e3306cb', name: 'game-assets' },
  { id: '019f2e1699887744aa11bb22cc33dd44', name: 'audio' },
]

// Per-repo mutable change set, keyed by working-dir path. Defaults for any path.
function seedFiles(): ChangedFile[] {
  return [
    { path: 'Source/Player/PlayerCharacter.cpp', action: 'modify', isBinary: false, size: 8241 },
    { path: 'Source/Player/PlayerCharacter.h', action: 'modify', isBinary: false, size: 1204 },
    { path: 'Content/Characters/Hero/SK_Hero.uasset', action: 'add', isBinary: true, size: 4718592 },
    { path: 'Content/Maps/Level_01.umap', action: 'modify', isBinary: true, size: 2359296 },
    { path: 'Config/DefaultInput.ini', action: 'modify', isBinary: false, size: 512 },
    { path: 'Docs/old-notes.md', action: 'delete', isBinary: false, size: 0 },
  ]
}

interface RepoState { branch: string; localAhead: number; remoteAhead: number; files: ChangedFile[] }
const repoStates = new Map<string, RepoState>()

function stateFor(repoPath: string): RepoState {
  if (!repoStates.has(repoPath)) {
    repoStates.set(repoPath, { branch: 'main', localAhead: 0, remoteAhead: 1, files: seedFiles() })
  }
  return repoStates.get(repoPath)!
}

export const mock: LoreApi = {
  async isAuthenticated() {
    await delay(120)
    return localStorage.getItem(AUTH_KEY) === '1'
  },
  async signIn(_serverUrl: string) {
    await delay(700) // simulate the browser round-trip
    localStorage.setItem(AUTH_KEY, '1')
  },
  async signOut() {
    localStorage.removeItem(AUTH_KEY)
  },
  async listRepos(_serverUrl: string) {
    await delay()
    return FAKE_REPOS
  },
  async getStatus(repoPath: string) {
    await delay(250)
    const s = stateFor(repoPath)
    return { branch: s.branch, localAhead: s.localAhead, remoteAhead: s.remoteAhead, files: [...s.files] } as StatusResult
  },
  async commitAll(repoPath: string, message: string) {
    if (!message.trim()) throw new Error('commit message is required')
    await delay(500)
    const s = stateFor(repoPath)
    s.files = []
    s.localAhead += 1
  },
  async push(repoPath: string) {
    await delay(600)
    stateFor(repoPath).localAhead = 0
  },
  async sync(repoPath: string) {
    await delay(500)
    stateFor(repoPath).remoteAhead = 0
  },
  async loadConfig() {
    await delay(60)
    const raw = localStorage.getItem(CONFIG_KEY)
    if (raw) { try { return JSON.parse(raw) as AppConfig } catch { /* fall through */ } }
    return { serverUrl: null, currentRepo: null, recentRepos: [] }
  },
  async saveConfig(config: AppConfig) {
    localStorage.setItem(CONFIG_KEY, JSON.stringify(config))
  },
}
