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
  /** The local head's revision number; `revisionNumber < localRevisionNumber` = time-traveled (behind). */
  localRevisionNumber: number
  /** False when the server can't be reached (offline). */
  remoteAvailable: boolean
  /** False when the stored session is no longer accepted. */
  remoteAuthorized: boolean
  /** A merge is waiting for conflict resolution — the Merge view can resume it. */
  mergeInProgress: boolean
  /** An interrupted commit/merge left a staged state; picked up by the next commit/merge. */
  stagedPending: boolean
  /** Compteurs wire repositoryStatusSummary ; absent sur un CLI plus ancien. */
  summary?: { adds: number; mods: number; dels: number }
  /** Chemins exclus par le filtrage natif .loreignore du CLI (dédupliqués ; un dossier exclu compte pour un). */
  ignoredCount: number
  files: ChangedFile[]
}

/** One progress tick of a long operation (clone/sync/push). No `total` = indeterminate. */
export interface OpProgress {
  done: number
  total?: number
  unit?: 'bytes' | 'files'
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

export interface SharedStoreStatus {
  exists: boolean
  path: string | null
  /** Global "use automatically for clones" flag (when the CLI reports it). */
  autoUse?: boolean
}

/** Fields of `lore repository info` — all optional: an absent field hides its row. */
export interface RepositoryInfo {
  id?: string
  name?: string
  remoteUrl?: string
  description?: string
  defaultBranchName?: string
  /** Repo creation time, epoch SECONDS (multiply by 1000 for a JS Date). */
  created?: number
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
  /** 'remote' = existe seulement côté serveur (le switch reste permis — le CLI fait le checkout). Absent → local. */
  location?: 'local' | 'remote'
}

/** Ahead/behind counters fed to `formatAheadBehind`. The CLI's `branch info`
 *  exposes no per-branch counters (verified — fixtures README), so in practice
 *  these are the CURRENT branch's counts sourced from the status. */
export interface BranchInfo {
  ahead?: number
  behind?: number
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
  /** UI theme; unset = dark (the default). */
  theme?: 'light' | 'dark'
}

/** A newer version the update endpoint offers. */
export interface UpdateInfo {
  /** Version of the available update (no leading 'v'). */
  version: string
  /** Release notes (may be empty; the full notes live on the release page). */
  notes: string
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
  /** Open repo's metadata for the About panel (best-effort: may reject offline). */
  getRepositoryInfo(repoPath: string): Promise<RepositoryInfo>
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
  /** Clone <serverUrl>/<repoId> into <destParent>/<repoName>; returns the created path. Progress ticks stream via onProgress. */
  cloneRepo(serverUrl: string, repoId: string, repoName: string, destParent: string, onProgress?: (p: OpProgress) => void): Promise<string>
  /** Whether a shared object store exists on this machine (and the global auto-use flag). */
  sharedStoreStatus(): Promise<SharedStoreStatus>
  /** Create the per-remote store for `serverUrl` if needed, then enable global auto-use. */
  sharedStoreEnable(serverUrl: string): Promise<void>
  /** Turn off global auto-use (the store is kept). */
  sharedStoreDisable(): Promise<void>
  getStatus(repoPath: string): Promise<StatusResult>
  /** Repository-revision sizes of the given files (ONE batch `file info` call) — the "old" side of the size delta. */
  fileSizes(repoPath: string, paths: string[]): Promise<Record<string, number>>
  getDiff(repoPath: string, path: string): Promise<DiffLine[]>
  /** Diff of one file between two revisions (source→target signatures), for the History preview. */
  getFileDiffAt(repoPath: string, path: string, source: string, target: string): Promise<DiffLine[]>
  /** Commit the working changes except `exclude` (unchecked files stay pending). */
  commitAll(repoPath: string, message: string, exclude: string[]): Promise<void>
  push(repoPath: string, onProgress?: (p: OpProgress) => void): Promise<void>
  sync(repoPath: string, onProgress?: (p: OpProgress) => void): Promise<void>
  /** Time-travel the working copy to a revision (hash). Progress ticks stream via onProgress. */
  syncToRevision(repoPath: string, revision: string, onProgress?: (p: OpProgress) => void): Promise<void>
  /** Restore one file to its content at `revision` as a working change (LOCAL — nothing pushed). Progress ticks stream via onProgress. */
  restoreFile(repoPath: string, path: string, revision: string, onProgress?: (p: OpProgress) => void): Promise<void>
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
  /** Archive a branch (it disappears from lists; files untouched). */
  archiveBranch(repoPath: string, name: string): Promise<void>
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
  /** Show the file in the system file manager (parent dir if the file is gone). */
  revealPath(absPath: string): Promise<void>
  /** Open the file with its default application. */
  openPath(absPath: string): Promise<void>
  /** Absolute path of the CLI's log directory (for the Preferences "Open CLI logs" button). */
  logfileLocation(): Promise<string>
  /** Absolute path of the app's own log directory (tauri-plugin-log files), distinct from the CLI's. */
  getAppLogDir(): Promise<string>
  /** Does this directory still exist on disk? Drives the "Missing" repo state. */
  pathExists(path: string): Promise<boolean>
  /** Re-register a repository after its folder moved; resolves when the new path answers a status. */
  updateRepoPath(newPath: string): Promise<void>
  loadConfig(): Promise<AppConfig>
  saveConfig(config: AppConfig): Promise<void>
  /** Ask the update endpoint whether a newer version exists; null = up to date. */
  checkForUpdate(): Promise<UpdateInfo | null>
  /** Download and install the pending update, then RELAUNCH the app — the real
   *  implementation never resolves for the caller (the process restarts).
   *  `onProgress` receives a 0..100 percentage. */
  installUpdate(onProgress: (pct: number) => void): Promise<void>
  /** The running app's version (tauri.conf.json `version`). */
  getAppVersion(): Promise<string>
}
