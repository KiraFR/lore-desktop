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
  /** Still needs a mine/theirs choice; false once resolved (until the merge commits). */
  unresolved: boolean
}

export interface MergePreview {
  /** Incoming file changes the merge would bring in. */
  files: number
  /** Number of conflicting files (0 = a clean, executable merge). */
  conflicts: number
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
  /** Commit the working changes except `exclude` (unchecked files stay pending). */
  commitAll(repoPath: string, message: string, exclude: string[]): Promise<void>
  push(repoPath: string): Promise<void>
  sync(repoPath: string): Promise<void>
  /** Files the current user holds locked that are part of the pending push. */
  pushedLockFiles(repoPath: string): Promise<string[]>
  setLock(repoPath: string, path: string, lock: boolean): Promise<void>
  /** Discard a file's working changes, restoring the committed version. */
  discardFile(repoPath: string, path: string): Promise<void>
  getHistory(repoPath: string, length: number, cursor?: string): Promise<HistoryPage>
  /** Files changed by a single commit (diff vs its first parent); fetched lazily on select. */
  getCommitFiles(repoPath: string, revision: string, parent: string): Promise<CommitFile[]>
  getBranches(repoPath: string): Promise<Branch[]>
  switchBranch(repoPath: string, name: string): Promise<void>
  createBranch(repoPath: string, name: string, basedOn: string): Promise<void>
  previewMerge(repoPath: string, source: string, target: string): Promise<MergePreview>
  /** Merge `source` into the current branch (clean/no-conflict path — auto-commits). */
  mergeBranch(repoPath: string, source: string, message: string): Promise<void>
  /** Start a conflicting merge of `source` into the current branch (enters resolution). */
  mergeStart(repoPath: string, source: string): Promise<void>
  /** Conflicted files of the in-progress merge. */
  mergeConflicts(repoPath: string): Promise<MergeConflict[]>
  /** Resolve one file with 'mine' (current) or 'theirs' (source). */
  mergeResolve(repoPath: string, path: string, side: 'mine' | 'theirs'): Promise<void>
  /** Finalize the merge once all conflicts are resolved. */
  mergeCommit(repoPath: string, message: string): Promise<void>
  /** Abort the in-progress merge. */
  mergeAbort(repoPath: string): Promise<void>
  getLocks(repoPath: string): Promise<LockEntry[]>
  loadConfig(): Promise<AppConfig>
  saveConfig(config: AppConfig): Promise<void>
}
