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

export interface HistoryPage {
  commits: Commit[]
  nextCursor: string | null
}

export interface DiffLine {
  kind: 'add' | 'del' | 'context' | 'hunk'
  text: string
  oldLine: number | null
  newLine: number | null
}

/** The data boundary the whole app uses. Mock now; Tauri-invoke later. */
export interface LoreApi {
  isAuthenticated(): Promise<boolean>
  signIn(serverUrl: string, authUrlOverride?: string): Promise<void>
  signOut(): Promise<void>
  listRepos(serverUrl: string): Promise<RepoEntry[]>
  /** Native OS directory chooser; returns the absolute path or null if cancelled. */
  pickFolder(): Promise<string | null>
  /** Clone <serverUrl>/<repoId> into <destParent>/<repoName>; returns the created path. */
  cloneRepo(serverUrl: string, repoId: string, repoName: string, destParent: string): Promise<string>
  getStatus(repoPath: string): Promise<StatusResult>
  getDiff(repoPath: string, path: string): Promise<DiffLine[]>
  commitAll(repoPath: string, message: string): Promise<void>
  push(repoPath: string): Promise<void>
  sync(repoPath: string): Promise<void>
  /** Files the current user holds locked that are part of the pending push. */
  pushedLockFiles(repoPath: string): Promise<string[]>
  setLock(repoPath: string, path: string, lock: boolean): Promise<void>
  getHistory(repoPath: string, length: number, cursor?: string): Promise<HistoryPage>
  /** Files changed by a single commit (diff vs its first parent); fetched lazily on select. */
  getCommitFiles(repoPath: string, revision: string, parent: string): Promise<CommitFile[]>
  getBranches(repoPath: string): Promise<Branch[]>
  switchBranch(repoPath: string, name: string): Promise<void>
  createBranch(repoPath: string, name: string, basedOn: string): Promise<void>
  previewMerge(repoPath: string, source: string, target: string): Promise<MergePreview>
  getLocks(repoPath: string): Promise<LockEntry[]>
  loadConfig(): Promise<AppConfig>
  saveConfig(config: AppConfig): Promise<void>
}
