export interface ChangedFile {
  path: string
  action: 'add' | 'modify' | 'delete' | 'move' | 'copy'
  isBinary: boolean
  size: number
  /** Previous size in bytes, for modified files (drives "old → new"). */
  oldSize?: number
  /** Lock holder's display name, or 'you' if held by the current user; absent/null when unlocked. */
  lockedBy?: string | null
}

export interface StatusResult {
  branch: string
  localAhead: number
  remoteAhead: number
  files: ChangedFile[]
}

export interface RepoEntry {
  id: string
  name: string
}

export interface CommitFile {
  path: string
  action: 'add' | 'modify' | 'delete' | 'move' | 'copy'
}

export interface Commit {
  id: string        // short hash
  rev: number       // revision number
  message: string
  author: string    // display name; 'you' for the current user
  when: string      // relative time (mock-provided; real backend gives a timestamp)
  adds: number
  mods: number
  dels: number
  lane: number      // graph column (0 = mainline)
  parents: string[] // parent commit ids (2+ ⇒ a merge)
  head?: string     // branch-head label at this commit, e.g. 'main'
  files: CommitFile[]
}

export interface Branch {
  name: string
  current: boolean
  rev: number      // tip revision
}

export interface MergeConflict {
  path: string
  isBinary: boolean
  mine: { author: string; rev: number }
  theirs: { author: string; rev: number }
  resolved?: 'mine' | 'theirs' | null
}

export interface MergePreview {
  commits: number
  files: number
  conflicts: MergeConflict[]
}

export interface LockEntry {
  path: string
  holder: string   // 'you' for the current user, else a display name
  when: string     // relative time (mock-provided)
}

export interface AppConfig {
  serverUrl: string | null
  currentRepo: string | null
  recentRepos: string[]
}

/** The data boundary the whole app uses. Mock now; Tauri-invoke later. */
export interface LoreApi {
  isAuthenticated(): Promise<boolean>
  signIn(serverUrl: string, authUrlOverride?: string): Promise<void>
  signOut(): Promise<void>
  listRepos(serverUrl: string): Promise<RepoEntry[]>
  getStatus(repoPath: string): Promise<StatusResult>
  commitAll(repoPath: string, message: string): Promise<void>
  push(repoPath: string): Promise<void>
  sync(repoPath: string): Promise<void>
  setLock(repoPath: string, path: string, lock: boolean): Promise<void>
  getHistory(repoPath: string): Promise<Commit[]>
  getBranches(repoPath: string): Promise<Branch[]>
  switchBranch(repoPath: string, name: string): Promise<void>
  createBranch(repoPath: string, name: string, basedOn: string): Promise<void>
  previewMerge(repoPath: string, source: string, target: string): Promise<MergePreview>
  getLocks(repoPath: string): Promise<LockEntry[]>
  loadConfig(): Promise<AppConfig>
  saveConfig(config: AppConfig): Promise<void>
}
