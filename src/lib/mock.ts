import { isPreviewableImage, stripTheirsSuffix } from './previewKind'
import type { AppConfig, Branch, ChangedFile, Commit, CommitFile, DiffLine, FileRevision, LockEntry, LoreApi, MergeConflict, MergePreview, OpProgress, PreviewData, RepoEntry, RepositoryInfo, StatusResult } from './types'

/** Small 440 Hz sine burst with decay (~0.5 s) so the mock waveform has a visible shape. */
export function mockWavDataUrl(): string {
  const sampleRate = 8000, samples = 4000
  const buf = new ArrayBuffer(44 + samples * 2)
  const v = new DataView(buf)
  const w4 = (o: number, s: string) => { for (let i = 0; i < 4; i++) v.setUint8(o + i, s.charCodeAt(i)) }
  w4(0, 'RIFF'); v.setUint32(4, 36 + samples * 2, true); w4(8, 'WAVE')
  w4(12, 'fmt '); v.setUint32(16, 16, true); v.setUint16(20, 1, true); v.setUint16(22, 1, true)
  v.setUint32(24, sampleRate, true); v.setUint32(28, sampleRate * 2, true); v.setUint16(32, 2, true); v.setUint16(34, 16, true)
  w4(36, 'data'); v.setUint32(40, samples * 2, true)
  for (let i = 0; i < samples; i++) {
    const env = Math.exp(-i / 1200)
    v.setInt16(44 + i * 2, Math.round(Math.sin((2 * Math.PI * 440 * i) / sampleRate) * env * 20000), true)
  }
  let bin = ''
  new Uint8Array(buf).forEach((b) => (bin += String.fromCharCode(b)))
  return `data:audio/wav;base64,${btoa(bin)}`
}

const PREVIEW_AUDIO_RE = /\.(wav|ogg|mp3|flac)$/i
const PREVIEW_MODEL_RE = /\.(glb|gltf|obj|fbx)$/i
const CUBE_OBJ = 'v -1 -1 -1\nv 1 -1 -1\nv 1 1 -1\nv -1 1 -1\nv -1 -1 1\nv 1 -1 1\nv 1 1 1\nv -1 1 1\nf 1 2 3 4\nf 5 8 7 6\nf 1 5 6 2\nf 2 6 7 3\nf 3 7 8 4\nf 5 1 4 8\n'

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
      whenMs: Date.now() - i * 12 * 60_000, // plausible spacing; the labels above stay approximate
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
    { path: 'Content/Maps/Level_01.umap', action: 'modify', isBinary: true, size: 2359296, lockedBy: 'you' },
    { path: 'Content/Characters/Hero/SK_Hero.uasset', action: 'add', isBinary: true, size: 4718592 },
    { path: 'Content/Environment/T_Cliff_D.uasset', action: 'modify', isBinary: true, size: 4404019, lockedBy: 'Maya R' },
    { path: 'Content/UI/T_Icon_Sword.png', action: 'modify', isBinary: true, size: 182044 },
    { path: 'Audio/sfx_hit.wav', action: 'add', isBinary: true, size: 912384 },
    { path: 'Content/Props/SM_Crate.obj', action: 'add', isBinary: true, size: 20480 },
    { path: 'Source/Player/PlayerCharacter.cpp', action: 'modify', isBinary: false, size: 8241 },
    { path: 'Source/Player/PlayerCharacter.h', action: 'modify', isBinary: false, size: 1204 },
    { path: 'Config/DefaultInput.ini', action: 'modify', isBinary: false, size: 512 },
    { path: 'Docs/old-notes.md', action: 'delete', isBinary: false, size: 0 },
  ]
}

// "Old" (repository-revision) sizes served by fileSizes, so the browser dev
// exercises the same fire-and-forget enrichment as the real app (deltas pop
// in ~400 ms after the status). T_Icon_Sword old == new → no delta shown.
const MOCK_OLD_SIZES: Record<string, number> = {
  'Content/Maps/Level_01.umap': 2100480,
  'Content/Environment/T_Cliff_D.uasset': 4093640,
  'Content/UI/T_Icon_Sword.png': 182044,
  'Source/Player/PlayerCharacter.cpp': 7980,
  'Source/Player/PlayerCharacter.h': 1180,
  'Config/DefaultInput.ini': 500,
  'Docs/old-notes.md': 3400,
}

function buildBranches(extra: number): Branch[] {
  const base: Branch[] = [
    { name: 'main', current: true, location: 'local' },
    { name: 'feature/loot', current: false, location: 'local' },
    { name: 'fix/lighting-bake', current: false, location: 'local' },
    { name: 'experimental/ai-nav', current: false, location: 'local' },
    // Remote-only: visible in the new Remote section; switching checks them out.
    { name: 'release/1.0-cut', current: false, location: 'remote' },
    { name: 'user/maya/lighting-wip', current: false, location: 'remote' },
    { name: 'hotfix/crash-on-load', current: false, location: 'remote' },
  ]
  const prefixes = ['feature', 'fix', 'chore', 'release', 'hotfix', 'exp', 'wip', 'user']
  const topics = ['loot', 'lighting', 'nav', 'inventory', 'audio', 'netcode', 'mesh', 'ui', 'save', 'recoil', 'input', 'materials', 'ai', 'physics', 'vfx', 'hud', 'quest', 'crafting']
  const gen: Branch[] = []
  for (let i = 0; i < extra; i++) {
    gen.push({ name: `${prefixes[i % prefixes.length]}/${topics[(i * 5) % topics.length]}-${i + 1}`, current: false, location: 'local' })
  }
  return [...base, ...gen]
}

let branchList: Branch[] = buildBranches(2000)

// In-progress merge conflicts (mock): populated by mergeStart, cleared by commit/abort.
// Deliberately global (like branchList/lockList below), not per-repo.
let mergeConflictState: MergeConflict[] = []

let lockList: LockEntry[] = [
  { path: 'Content/Maps/Level_01.umap', holder: 'you', when: '12 min ago' },
  { path: 'Content/Environment/T_Cliff_D.uasset', holder: 'Maya R', when: '2 hours ago' },
  { path: 'Content/Characters/Hero/SK_Hero.uasset', holder: 'Alex L', when: 'yesterday' },
]

interface RepoState { branch: string; localAhead: number; remoteAhead: number; revisionNumber: number; localRevisionNumber: number; files: ChangedFile[] }
const repoStates = new Map<string, RepoState>()

function stateFor(repoPath: string): RepoState {
  if (!repoStates.has(repoPath)) {
    repoStates.set(repoPath, { branch: 'main', localAhead: 0, remoteAhead: 1, revisionNumber: 5, localRevisionNumber: 5, files: seedFiles() })
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
  async getRepositoryInfo(_repoPath: string) {
    await delay(180)
    return {
      id: '019f333af5e073d28bb117ad1596784a',
      name: 'game-main',
      remoteUrl: 'lore://lore.example.com:41337',
      description: 'Main game repository',
      defaultBranchName: 'main',
      created: 1783270930,
    } as RepositoryInfo
  },
  async pickFolder() {
    await delay(120)
    return 'C:/SoonerOrLater/picked-repo'
  },
  async pickRepoFile(repoPath: string) {
    await delay(120)
    return `${repoPath}/Content/Environment/SM_Rock_02.uasset`
  },
  async getIdentity(_repoPath: string) {
    await delay(100)
    return { id: 'mock-user', email: 'jane.doe@studio.dev' }
  },
  async startNotifications() {
    // No simulated team noise in dev — the real flow is exercised in Tauri.
    return () => {}
  },
  async getFileHistory(_repoPath: string, path: string) {
    await delay(220)
    const name = path.split('/').pop() ?? path
    return [
      { revision: 'fh3', revisionNumber: 5, action: 'modify', size: 2359296, message: 'Rebalance lighting pass', author: 'jane.doe@studio.dev', when: '2 hours ago', whenMs: Date.now() - 7_200_000 },
      { revision: 'fh2', revisionNumber: 3, action: 'modify', size: 2100480, message: 'First blockout', author: 'maya.r@studio.dev', when: '3 days ago', whenMs: Date.now() - 259_200_000 },
      { revision: 'fh1', revisionNumber: 1, action: 'add', size: 1848000, message: `Import ${name}`, author: 'maya.r@studio.dev', when: 'last week', whenMs: Date.now() - 604_800_000 },
    ] as FileRevision[]
  },
  async getPreview(_repoPath: string, path: string) {
    await delay(200)
    // A merge's ~theirs sidecar previews like its base file (dev parity with
    // preview_ext in preview.rs).
    path = stripTheirsSuffix(path)
    if (PREVIEW_AUDIO_RE.test(path)) return { kind: 'audio', url: mockWavDataUrl() } as PreviewData
    if (PREVIEW_MODEL_RE.test(path)) {
      // Only .obj carries a payload (a tiny cube — OBJ is plain text); other
      // model formats fall back to the placeholder in dev.
      const url = /\.obj$/i.test(path) ? `data:text/plain,${encodeURIComponent(CUBE_OBJ)}` : null
      return { kind: 'model', url } as PreviewData
    }
    if (isPreviewableImage(path)) {
      const name = path.split('/').pop() ?? path
      const svg =
        `<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512">` +
        `<defs><pattern id="c" width="32" height="32" patternUnits="userSpaceOnUse">` +
        `<rect width="32" height="32" fill="#2b2f35"/><rect width="16" height="16" fill="#3a4048"/>` +
        `<rect x="16" y="16" width="16" height="16" fill="#3a4048"/></pattern></defs>` +
        `<rect width="512" height="512" fill="url(#c)"/>` +
        `<text x="256" y="264" font-family="sans-serif" font-size="26" fill="#9fb0c0" text-anchor="middle">${name}</text></svg>`
      return { kind: 'image', url: `data:image/svg+xml,${encodeURIComponent(svg)}`, width: 2048, height: 2048 } as PreviewData
    }
    return { kind: 'none', url: null } as PreviewData
  },
  async cloneRepo(_serverUrl: string, _repoId: string, repoName: string, destParent: string, onProgress?: (p: OpProgress) => void) {
    // Simulated determinate transfer so the clone progress bar lives in dev.
    const total = 48 * 1024 * 1024
    for (let i = 1; i <= 12; i++) {
      await delay(90)
      onProgress?.({ done: Math.round((total * i) / 12), total, unit: 'bytes' })
    }
    return `${destParent}/${repoName}`
  },
  async getStatus(repoPath: string) {
    await delay(250)
    // Dev lever: simulate an out-of-app `branch merge abort` — run
    // `localStorage.setItem('loredesktop.mock.externalAbort', '1')` in the
    // devtools, then refocus the window (the focus refresh picks it up).
    if (localStorage.getItem('loredesktop.mock.externalAbort') === '1') {
      localStorage.removeItem('loredesktop.mock.externalAbort')
      mergeConflictState = []
    }
    const s = stateFor(repoPath)
    return {
      branch: s.branch, localAhead: s.localAhead, remoteAhead: s.remoteAhead,
      revisionNumber: s.revisionNumber, localRevisionNumber: s.localRevisionNumber,
      remoteAvailable: true, remoteAuthorized: true,
      mergeInProgress: mergeConflictState.length > 0,
      // A merge implies a staged state; otherwise, dev lever:
      // `localStorage.setItem('loredesktop.mock.staged', '1')` in the browser
      // devtools to preview the staged chip; removeItem to clear it.
      stagedPending: mergeConflictState.length > 0 || localStorage.getItem('loredesktop.mock.staged') === '1',
      // Seedé depuis les fichiers courants pour rester cohérent (spec P4 item 1).
      summary: {
        adds: s.files.filter((f) => f.action === 'add').length,
        mods: s.files.filter((f) => f.action === 'modify' || f.action === 'move' || f.action === 'copy').length,
        dels: s.files.filter((f) => f.action === 'delete').length,
      },
      files: [...s.files],
    } as StatusResult
  },
  async fileSizes(_repoPath: string, paths: string[]) {
    await delay(400)
    const out: Record<string, number> = {}
    for (const p of paths) if (Object.hasOwn(MOCK_OLD_SIZES, p)) out[p] = MOCK_OLD_SIZES[p]
    return out
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
  async push(repoPath: string, onProgress?: (p: OpProgress) => void) {
    for (let i = 1; i <= 6; i++) {
      await delay(100)
      onProgress?.({ done: i, total: 6, unit: 'files' })
    }
    stateFor(repoPath).localAhead = 0
  },
  async sync(repoPath: string, onProgress?: (p: OpProgress) => void) {
    for (let i = 1; i <= 6; i++) {
      await delay(80)
      onProgress?.({ done: i, total: 6, unit: 'files' })
    }
    const s = stateFor(repoPath)
    s.remoteAhead = 0
    s.revisionNumber = s.localRevisionNumber
  },
  async syncToRevision(repoPath: string, _revision: string, onProgress?: (p: OpProgress) => void) {
    for (let i = 1; i <= 6; i++) {
      await delay(80)
      onProgress?.({ done: i, total: 6, unit: 'files' })
    }
    stateFor(repoPath).revisionNumber = 3 // time-traveled → below the head (5)
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
  async undoCommit(repoPath: string, _parentRevision: string) {
    await delay(400)
    const s = stateFor(repoPath)
    if (s.localAhead > 0) s.localAhead -= 1
    // The undone commit's change reappears as pending.
    if (!s.files.some((f) => f.path === 'Source/Player/Undone.cpp'))
      s.files = [{ path: 'Source/Player/Undone.cpp', action: 'modify', isBinary: false, size: 1024 }, ...s.files]
  },
  async amendCommit(_repoPath: string, message: string) {
    await delay(300)
    // BIG_HISTORY[0] is the newest commit — the only one amend may rewrite.
    BIG_HISTORY[0].message = message
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
    branchList = branchList.map((b) => ({
      ...b,
      current: b.name === name,
      location: b.name === name ? 'local' : b.location,
    }))
    stateFor(repoPath).branch = name
  },
  async createBranch(repoPath: string, name: string, _basedOn: string) {
    await delay(400)
    branchList = branchList.map((b) => ({ ...b, current: false }))
    branchList = [...branchList, { name, current: true }]
    stateFor(repoPath).branch = name
  },
  async archiveBranch(_repoPath: string, name: string) {
    await delay(250)
    branchList = branchList.filter((b) => b.name !== name)
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
      { path: 'Source/Player/PlayerCharacter.cpp', isBinary: false, unresolved: true },
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
  async revealPath(_absPath: string) {
    // No OS shell in the browser — parity no-op.
  },
  async openPath(_absPath: string) {
    // No OS shell in the browser — parity no-op.
  },
  async pathExists(path: string) {
    // Dev lever: localStorage.setItem('loredesktop.mock.missing', JSON.stringify(['C:/repos/x']))
    let missing: string[] = []
    try { missing = JSON.parse(localStorage.getItem('loredesktop.mock.missing') ?? '[]') } catch { /* corrupt lever → nothing missing */ }
    return !missing.includes(path)
  },
  async updateRepoPath(_newPath: string) {
    await delay(300)
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
