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
  revisionNumber: number
  /** False when the server can't be reached (offline). */
  remoteAvailable: boolean
  /** False when the stored session is no longer accepted. */
  remoteAuthorized: boolean
  files: ChangedFile[]
}

export type { LoreNotification } from './notifyRouting'
import type { LoreNotification } from './notifyRouting'

export interface PreviewData {
  kind: 'image' | 'audio' | 'model' | 'none'
  /** image: PNG data URL of the thumbnail; audio/model: streamable URL; none: null. */
  url: string | null
  /** Source dimensions (image only). */
  width?: number
  height?: number
}

export interface FileRevision {
  revision: string
  revisionNumber: number
  action: 'add' | 'modify' | 'delete' | 'move' | 'copy'
  size: number
  message: string
  /** Email (resolved) or raw user id; the 'you' mapping is UI-side. */
  author: string
  /** Relative time. */
  when: string
  /** Absolute epoch-ms, for the tooltip. */
  whenMs: number
}

export interface Identity {
  id: string
  /** The account email as the server knows it (authUserInfo.name). */
  email: string
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
  whenMs: number    // absolute epoch-ms, for the exact-date tooltip
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
  /** Optional user-chosen name shown in the avatar/menu; falls back to the email. */
  displayName?: string | null
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
  /** Native file chooser starting inside the repo; absolute path or null if cancelled. */
  pickRepoFile(repoPath: string): Promise<string | null>
  /** Identity per the current repo's server; rejects when no repo/no session. */
  getIdentity(repoPath: string): Promise<Identity>
  /** Working-copy visual/audio preview of a repo file. `maxPx` bounds image thumbnails (default 512). */
  getPreview(repoPath: string, path: string, maxPx?: number): Promise<PreviewData>
  /** Live server events for the repo; resolves to a stop function. */
  startNotifications(repoPath: string, onEvent: (e: LoreNotification) => void): Promise<() => void>
  /** Revision timeline of one file (newest first). */
  getFileHistory(repoPath: string, path: string): Promise<FileRevision[]>
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
  /** Undo the last local commit — its changes return to the pending set. */
  undoCommit(repoPath: string, parentRevision: string): Promise<void>
  /** Rewrite the last local commit's message. */
  amendCommit(repoPath: string, message: string): Promise<void>
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
