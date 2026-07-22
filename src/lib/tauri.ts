import { getVersion } from '@tauri-apps/api/app'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { relaunch } from '@tauri-apps/plugin-process'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { mock } from './mock'
import { progressPct } from './updater'
import type { AppConfig, Branch, CommitFile, DiffLine, FileRevision, HistoryPage, Identity, LockEntry, LoreApi, LoreNotification, MergeConflict, MergePreview, OpProgress, PreviewData, RepoEntry, RepositoryInfo, SharedStoreStatus, StatusResult } from './types'

type WireProgress = { opId: string; kind: string; done: number; total?: number; unit?: 'bytes' | 'files' }

// The Update handle from the last successful check — downloadAndInstall must be
// called on that same object, so it is kept between the two API calls.
let pendingUpdate: Update | null = null

/**
 * Invoke a long command with a frontend-generated opId, listening to
 * `lore://op-progress` filtered on that id for the call's duration. The id is
 * what distinguishes simultaneous operations (e.g. a sync during a clone).
 */
async function invokeWithProgress<T>(
  cmd: string,
  args: Record<string, unknown>,
  onProgress?: (p: OpProgress) => void,
): Promise<T> {
  const opId = crypto.randomUUID()
  let unlisten: (() => void) | null = null
  if (onProgress) {
    try {
      unlisten = await listen<WireProgress>('lore://op-progress', (e) => {
        if (e.payload.opId === opId)
          onProgress({ done: e.payload.done, total: e.payload.total, unit: e.payload.unit })
      })
    } catch { /* progress is best-effort; the op itself must still run */ }
  }
  try {
    return await invoke<T>(cmd, { ...args, opId })
  } finally {
    unlisten?.()
  }
}

export const tauriApi: LoreApi = {
  ...mock,
  isAuthenticated: () => invoke<boolean>('lore_is_authenticated'),
  signIn: (serverUrl, authUrlOverride) =>
    invoke<void>('lore_sign_in', { serverUrl, authUrl: authUrlOverride ?? null }),
  listRepos: (serverUrl) => invoke<RepoEntry[]>('lore_repositories', { serverUrl }),
  getRepositoryInfo: (repoPath) => invoke<RepositoryInfo>('lore_repository_info', { repoPath }),
  getStatus: (repoPath) => invoke<StatusResult>('lore_status', { repoPath }),
  fileSizes: (repoPath, paths) => invoke<Record<string, number>>('lore_file_sizes', { repoPath, paths }),
  getDiff: (repoPath, path) => invoke<DiffLine[]>('lore_diff', { repoPath, path }),
  getFileDiffAt: (repoPath, path, source, target) => invoke<DiffLine[]>('lore_diff_revs', { repoPath, path, source, target }),
  getHistory: (repoPath, length, cursor) =>
    invoke<HistoryPage>('lore_history', { repoPath, length, cursor: cursor ?? null }),
  getCommitFiles: (repoPath, revision, parent) =>
    invoke<CommitFile[]>('lore_commit_files', { repoPath, revision, parent }),
  pickFolder: async () => {
    const picked = await open({ directory: true, multiple: false })
    return typeof picked === 'string' ? picked : null
  },
  pickRepoFile: async (repoPath) => {
    const picked = await open({ directory: false, multiple: false, defaultPath: repoPath })
    return typeof picked === 'string' ? picked : null
  },
  getIdentity: (repoPath) => invoke<Identity>('lore_identity', { repoPath }),
  getFileHistory: (repoPath, path) => invoke<FileRevision[]>('lore_file_history', { repoPath, path }),
  signOut: () => invoke<void>('lore_sign_out'),
  startNotifications: async (repoPath, onEvent) => {
    const unlisten = await listen<LoreNotification>('lore://notification', (e) => onEvent(e.payload))
    await invoke('lore_notifications_start', { repoPath })
    return () => {
      unlisten()
      invoke('lore_notifications_stop').catch(() => { /* app closing */ })
    }
  },
  getPreview: async (repoPath, path, maxPx): Promise<PreviewData> => {
    const r = await invoke<{ kind: string; dataUrl: string | null; width?: number | null; height?: number | null }>(
      'lore_preview', { repoPath, path, maxPx: maxPx ?? 512 })
    if (r.kind === 'audio') return { kind: 'audio', url: convertFileSrc(`${repoPath}/${path}`) }
    if (r.kind === 'model') return { kind: 'model', url: convertFileSrc(`${repoPath}/${path}`) }
    if (r.kind === 'image' && r.dataUrl)
      return { kind: 'image', url: r.dataUrl, width: r.width ?? undefined, height: r.height ?? undefined }
    return { kind: 'none', url: null }
  },
  cloneRepo: (serverUrl, repoId, repoName, destParent, onProgress) =>
    invokeWithProgress<string>('lore_clone', { serverUrl, repoId, repoName, destParent }, onProgress),
  sharedStoreStatus: () => invoke<SharedStoreStatus>('lore_shared_store_status'),
  sharedStoreEnable: (serverUrl) => invoke<void>('lore_shared_store_enable', { serverUrl }),
  sharedStoreDisable: () => invoke<void>('lore_shared_store_disable'),
  loadConfig: () => invoke<AppConfig>('config_load'),
  saveConfig: (config) => invoke<void>('config_save', { config }),
  commitAll: (repoPath, message, exclude) => invoke<void>('lore_commit', { repoPath, message, exclude }),
  push: (repoPath, onProgress) => invokeWithProgress<void>('lore_push', { repoPath }, onProgress),
  sync: (repoPath, onProgress) => invokeWithProgress<void>('lore_sync', { repoPath }, onProgress),
  syncToRevision: (repoPath, revision, onProgress) => invokeWithProgress<void>('lore_sync_to', { repoPath, revision }, onProgress),
  restoreFile: (repoPath, path, revision, onProgress) =>
    invokeWithProgress<void>('lore_restore_file', { repoPath, path, revision }, onProgress),
  pushedLockFiles: (repoPath) => invoke<string[]>('lore_pushed_lock_files', { repoPath }),
  setLock: (repoPath, path, lock) => invoke<void>('lore_set_lock', { repoPath, path, lock }),
  discardFile: (repoPath, path) => invoke<void>('lore_discard_file', { repoPath, path }),
  undoCommit: (repoPath, parentRevision) => invoke<void>('lore_undo_commit', { repoPath, parentRevision }),
  amendCommit: (repoPath, message) => invoke<void>('lore_amend', { repoPath, message }),
  getLocks: (repoPath) => invoke<LockEntry[]>('lore_locks', { repoPath }),
  revealPath: (absPath) => invoke<void>('os_reveal_path', { path: absPath }),
  openPath: (absPath) => invoke<void>('os_open_path', { path: absPath }),
  logfileLocation: () => invoke<string>('lore_logfile_location'),
  getAppLogDir: () => invoke<string>('app_log_dir'),
  pathExists: (path) => invoke<boolean>('os_path_exists', { path }),
  updateRepoPath: (newPath) => invoke<void>('lore_update_path', { newPath }),
  getBranches: (repoPath) => invoke<Branch[]>('lore_branches', { repoPath }),
  previewMerge: (repoPath, source) => invoke<MergePreview>('lore_merge_preview', { repoPath, source }),
  mergeBranch: (repoPath, source, message) => invoke<void>('lore_merge', { repoPath, source, message }),
  mergeStart: (repoPath, source) => invoke<void>('lore_merge_start', { repoPath, source }),
  mergeConflicts: (repoPath) => invoke<MergeConflict[]>('lore_merge_conflicts', { repoPath }),
  mergeResolve: (repoPath, path, side) => invoke<void>('lore_merge_resolve', { repoPath, path, side }),
  mergeCommit: (repoPath, message) => invoke<void>('lore_merge_commit', { repoPath, message }),
  mergeAbort: (repoPath) => invoke<void>('lore_merge_abort', { repoPath }),
  switchBranch: (repoPath, name) => invoke<void>('lore_switch_branch', { repoPath, name }),
  // The base is always the current HEAD in Lore, so `basedOn` is not forwarded.
  createBranch: (repoPath, name) => invoke<void>('lore_create_branch', { repoPath, name }),
  archiveBranch: (repoPath, name) => invoke<void>('lore_archive_branch', { repoPath, name }),
  checkForUpdate: async () => {
    const update = await check()
    pendingUpdate = update
    return update ? { version: update.version, notes: update.body ?? '' } : null
  },
  installUpdate: async (onProgress) => {
    const update = pendingUpdate ?? (await check())
    if (!update) throw new Error('No update is available')
    // Windows: the app puts itself in a kill-on-close Job object at startup
    // (src-tauri/src/job.rs) so the lore sidecars die with it. The NSIS
    // installer spawned by downloadAndInstall would be a member of that job
    // too, and the exit(0) inside install() closes the job's last handle —
    // TERMINATING the installer before it does anything. That is why every
    // in-app update failed silently while the same installer worked launched
    // by hand. prepare_update_breakaway flips
    // JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK on the job so processes spawned
    // from this point on are born outside it. Accepted side effect: lore
    // commands spawned during the short download+install window also escape
    // the job (they'd only be orphaned by a hard kill inside that window).
    // No-op on macOS/Linux. The command logs to the app file logs; the
    // console lines below cover the webview side.
    console.info('[update] preparing job breakaway before download+install')
    await invoke('prepare_update_breakaway')
    console.info('[update] job breakaway enabled, starting download+install')
    let total = 0
    let got = 0
    await update.downloadAndInstall((e) => {
      if (e.event === 'Started') {
        total = e.data.contentLength ?? 0
      } else if (e.event === 'Progress') {
        got += e.data.chunkLength
        const pct = progressPct(got, total)
        if (pct !== null) onProgress(pct)
      } else if (e.event === 'Finished') {
        onProgress(100)
      }
    })
    // What happens next is platform-specific (verified against the sources of
    // tauri-plugin-updater 2.10.1 and the NSIS template of @tauri-apps/cli
    // 2.11.4, nsis_tauri_utils 0.5.3):
    // - Windows (NSIS): install() spawns the new setup.exe with
    //   `/P /R /UPDATE /ARGS <argv>` (installMode "passive" implies /R) and
    //   immediately kills this process via std::process::exit(0) — the line
    //   below is never reached. The INSTALLER relaunches the app: its
    //   .onInstSuccess handler sees /R and starts the installed exe through
    //   nsis_tauri_utils::RunAsUser. A relaunch failure there is silent (the
    //   template ignores RunAsUser's return code) — check the app file logs
    //   (Preferences > Support > Open app logs) when chasing one.
    // - macOS/Linux: install() returns after swapping the bundle/AppImage and
    //   relaunch() below is the documented follow-up.
    await relaunch()
  },
  getAppVersion: () => getVersion(),
}
