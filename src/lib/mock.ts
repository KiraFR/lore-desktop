import type { AppConfig, Branch, ChangedFile, Commit, CommitFile, DiffLine, LockEntry, LoreApi, MergeConflict, MergePreview, RepoEntry, StatusResult } from './types'

const delay = (ms = 350) => new Promise((r) => setTimeout(r, ms))
const CONFIG_KEY = 'loredesktop.config'
const AUTH_KEY = 'loredesktop.signedin'

const FAKE_REPOS: RepoEntry[] = [
  { id: '019f2e14006f7870a7b27df367c78b72', name: 'game-main' },
  { id: '019f2e1577257382bc89c5a28e3306cb', name: 'game-assets' },
  { id: '019f2e1699887744aa11bb22cc33dd44', name: 'audio' },
]

// A large synthetic history to prove the History graph virtualizes/scrolls smoothly.
// Topology = a continuous `main` lane with short local feature branches that fork and
// merge back within a few rows, so every graph edge stays local (windowing-friendly).
function buildBigHistory(n: number): Commit[] {
  const authors: [string, string][] = [['you', 'JD'], ['Maya R', 'MR'], ['Alex L', 'AL'], ['Sam K', 'SK'], ['Ivy N', 'IN']]
  const verbs = ['Fix', 'Add', 'Refactor', 'Tune', 'Bake', 'Rebalance', 'Update', 'Remove', 'Optimize', 'Wire', 'Polish', 'Cache']
  const nouns = ['player movement', 'loot tables', 'arena lighting', 'hero mesh', 'inventory UI', 'net replication', 'audio mix', 'AI navigation', 'material LODs', 'input mapping', 'save system', 'weapon recoil']
  const dirs = ['Source/Player/', 'Source/Items/', 'Content/Maps/', 'Content/Characters/', 'Content/UI/', 'Config/', 'Content/Environment/']
  const exts = ['.cpp', '.h', '.uasset', '.umap', '.ini']
  const acts: CommitFile['action'][] = ['modify', 'add', 'delete']
  const id = (i: number) => 'c' + (0x100000 + i).toString(16)
  const whenFor = (i: number) =>
    i === 0 ? '2 min ago'
      : i < 5 ? `${i * 12} min ago`
        : i < 30 ? `${Math.ceil(i / 5)} hours ago`
          : i < 120 ? `${Math.ceil(i / 30)} days ago`
            : `${Math.ceil(i / 120)} weeks ago`
  const fileFor = (i: number, k: number): CommitFile => ({
    path: dirs[(i * 3 + k) % dirs.length] + verbs[(i + k) % verbs.length] + '_' + ((i * 7 + k) % 40) + exts[(i * 5 + k) % exts.length],
    action: acts[(i + k) % acts.length],
  })
  const mk = (i: number, lane: number, parents: string[], message: string, head?: string): Commit => {
    const a = authors[(i * 7) % authors.length]
    const nf = 1 + ((i * 3) % 3)
    return {
      id: id(i), rev: n - i, lane, parents, head, message, author: a[0], when: whenFor(i),
      adds: (i * 13) % 7, mods: 1 + (i * 5) % 5, dels: i % 9 === 0 ? 1 : 0,
      files: Array.from({ length: nf }, (_, k) => fileFor(i, k)),
    }
  }
  const out: Commit[] = []
  let labeled = false
  let sinceFeature = 6
  let i = 0
  while (i < n) {
    if (sinceFeature >= 8 && i + 3 < n) {
      out.push(mk(i, 0, [id(i + 3), id(i + 1)], 'Merge feature branch into main', i === 0 ? 'main' : undefined))
      const tip = !labeled ? 'feature/loot' : undefined
      labeled = true
      out.push(mk(i + 1, 1, [id(i + 2)], `${verbs[(i + 1) % verbs.length]} ${nouns[(i + 1) % nouns.length]}`, tip))
      out.push(mk(i + 2, 1, [id(i + 3)], `${verbs[(i + 2) % verbs.length]} ${nouns[(i + 2) % nouns.length]}`))
      i += 3
      sinceFeature = 0
    } else {
      out.push(mk(i, 0, i + 1 < n ? [id(i + 1)] : [], `${verbs[i % verbs.length]} ${nouns[(i * 2) % nouns.length]}`, i === 0 ? 'main' : undefined))
      i += 1
      sinceFeature += 1
    }
  }
  return out
}

const BIG_HISTORY: Commit[] = buildBigHistory(5000)

// Per-repo mutable change set, keyed by working-dir path. Defaults for any path.
function seedFiles(): ChangedFile[] {
  return [
    { path: 'Content/Maps/Level_01.umap', action: 'modify', isBinary: true, size: 2359296, oldSize: 2100480, lockedBy: 'you' },
    { path: 'Content/Characters/Hero/SK_Hero.uasset', action: 'add', isBinary: true, size: 4718592 },
    { path: 'Content/Environment/T_Cliff_D.uasset', action: 'modify', isBinary: true, size: 4404019, oldSize: 4093640, lockedBy: 'Maya R' },
    { path: 'Source/Player/PlayerCharacter.cpp', action: 'modify', isBinary: false, size: 8241, oldSize: 7980 },
    { path: 'Source/Player/PlayerCharacter.h', action: 'modify', isBinary: false, size: 1204, oldSize: 1180 },
    { path: 'Config/DefaultInput.ini', action: 'modify', isBinary: false, size: 512, oldSize: 500 },
    { path: 'Docs/old-notes.md', action: 'delete', isBinary: false, size: 0, oldSize: 3400 },
  ]
}

function buildBranches(extra: number): Branch[] {
  const base: Branch[] = [
    { name: 'main', current: true },
    { name: 'feature/loot', current: false },
    { name: 'fix/lighting-bake', current: false },
    { name: 'experimental/ai-nav', current: false },
  ]
  const prefixes = ['feature', 'fix', 'chore', 'release', 'hotfix', 'exp', 'wip', 'user']
  const topics = ['loot', 'lighting', 'nav', 'inventory', 'audio', 'netcode', 'mesh', 'ui', 'save', 'recoil', 'input', 'materials', 'ai', 'physics', 'vfx', 'hud', 'quest', 'crafting']
  const gen: Branch[] = []
  for (let i = 0; i < extra; i++) {
    gen.push({ name: `${prefixes[i % prefixes.length]}/${topics[(i * 5) % topics.length]}-${i + 1}`, current: false })
  }
  return [...base, ...gen]
}

let branchList: Branch[] = buildBranches(2000)

// In-progress merge conflicts (mock): populated by mergeStart, cleared by commit/abort.
let mergeConflictState: MergeConflict[] = []

let lockList: LockEntry[] = [
  { path: 'Content/Maps/Level_01.umap', holder: 'you', when: '12 min ago' },
  { path: 'Content/Environment/T_Cliff_D.uasset', holder: 'Maya R', when: '2 hours ago' },
  { path: 'Content/Characters/Hero/SK_Hero.uasset', holder: 'Alex L', when: 'yesterday' },
]

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
  async pickFolder() {
    await delay(120)
    return 'C:/SoonerOrLater/picked-repo'
  },
  async cloneRepo(_serverUrl: string, _repoId: string, repoName: string, destParent: string) {
    await delay(600) // simulate the network + disk work
    return `${destParent}/${repoName}`
  },
  async getStatus(repoPath: string) {
    await delay(250)
    const s = stateFor(repoPath)
    return { branch: s.branch, localAhead: s.localAhead, remoteAhead: s.remoteAhead, files: [...s.files] } as StatusResult
  },
  async getDiff(_repoPath: string, _path: string) {
    await delay(120)
    return [
      { kind: 'hunk', text: '@@ -1,3 +1,4 @@', oldLine: null, newLine: null },
      { kind: 'context', text: 'export const x = 1', oldLine: 1, newLine: 1 },
      { kind: 'del', text: 'const y = 2', oldLine: 2, newLine: null },
      { kind: 'add', text: 'const y = 3', oldLine: null, newLine: 2 },
      { kind: 'add', text: 'const z = 4', oldLine: null, newLine: 3 },
    ] as DiffLine[]
  },
  async commitAll(repoPath: string, message: string, exclude: string[] = []) {
    if (!message.trim()) throw new Error('commit message is required')
    await delay(500)
    const s = stateFor(repoPath)
    // Unchecked (excluded) files stay pending; the rest are committed away.
    s.files = s.files.filter((f) => exclude.includes(f.path))
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
  async pushedLockFiles(_repoPath: string) {
    await delay(150)
    // Stand-in for the real diff∩locks: every lock I currently hold.
    return lockList.filter((l) => l.holder === 'you').map((l) => l.path)
  },
  async setLock(repoPath: string, path: string, lock: boolean) {
    await delay(200)
    const f = stateFor(repoPath).files.find((x) => x.path === path)
    if (f) f.lockedBy = lock ? 'you' : null
    lockList = lockList.filter((l) => l.path !== path)
    if (lock) lockList = [{ path, holder: 'you', when: 'just now' }, ...lockList]
  },
  async getLocks(_repoPath: string) {
    await delay(150)
    return lockList.map((l) => ({ ...l }))
  },
  async discardFile(repoPath: string, path: string) {
    await delay(200)
    const s = stateFor(repoPath)
    s.files = s.files.filter((f) => f.path !== path)
  },
  async getHistory(_repoPath: string, length: number, cursor?: string) {
    await delay(280)
    const start = cursor ? BIG_HISTORY.findIndex((c) => c.id === cursor) + 1 : 0
    const commits = BIG_HISTORY.slice(start, start + length)
    const nextIndex = start + length
    return { commits, nextCursor: nextIndex < BIG_HISTORY.length ? commits[commits.length - 1].id : null }
  },
  async getCommitFiles(_repoPath: string, revision: string, _parent: string) {
    await delay(160)
    return (BIG_HISTORY.find((c) => c.id === revision)?.files ?? []).map((f) => ({ ...f }))
  },
  async getBranches(_repoPath: string) {
    await delay(200)
    return branchList.map((b) => ({ ...b }))
  },
  async switchBranch(repoPath: string, name: string) {
    await delay(300)
    branchList = branchList.map((b) => ({ ...b, current: b.name === name }))
    stateFor(repoPath).branch = name
  },
  async createBranch(repoPath: string, name: string, _basedOn: string) {
    await delay(400)
    branchList = branchList.map((b) => ({ ...b, current: false }))
    branchList = [...branchList, { name, current: true }]
    stateFor(repoPath).branch = name
  },
  async previewMerge(_repoPath: string, source: string, target: string): Promise<MergePreview> {
    await delay(300)
    if (source === target) return { files: 0, conflicts: 0 }
    if (source === 'feature/loot') return { files: 23, conflicts: 2 }
    return { files: 5, conflicts: 0 }
  },
  async mergeBranch(_repoPath: string, _source: string, _message: string) {
    await delay(500)
  },
  async mergeStart(_repoPath: string, _source: string) {
    await delay(400)
    mergeConflictState = [
      { path: 'Content/Environment/T_Cliff_D.uasset', isBinary: true, unresolved: true },
      { path: 'Content/Maps/Arena.umap', isBinary: true, unresolved: true },
    ]
  },
  async mergeConflicts(_repoPath: string) {
    await delay(150)
    return mergeConflictState.map((c) => ({ ...c }))
  },
  async mergeResolve(_repoPath: string, path: string, _side: 'mine' | 'theirs') {
    await delay(200)
    mergeConflictState = mergeConflictState.map((c) => (c.path === path ? { ...c, unresolved: false } : c))
  },
  async mergeCommit(_repoPath: string, _message: string) {
    await delay(400)
    mergeConflictState = []
  },
  async mergeAbort(_repoPath: string) {
    await delay(300)
    mergeConflictState = []
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
