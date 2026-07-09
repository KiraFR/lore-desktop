import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { mock } from './mock'
import type { AppConfig, Branch, CommitFile, DiffLine, FileRevision, HistoryPage, Identity, LockEntry, LoreApi, LoreNotification, MergeConflict, MergePreview, PreviewData, RepoEntry, StatusResult } from './types'

export const tauriApi: LoreApi = {
  ...mock,
  isAuthenticated: () => invoke<boolean>('lore_is_authenticated'),
  signIn: (serverUrl, authUrlOverride) =>
    invoke<void>('lore_sign_in', { serverUrl, authUrl: authUrlOverride ?? null }),
  listRepos: (serverUrl) => invoke<RepoEntry[]>('lore_repositories', { serverUrl }),
  getStatus: (repoPath) => invoke<StatusResult>('lore_status', { repoPath }),
  getDiff: (repoPath, path) => invoke<DiffLine[]>('lore_diff', { repoPath, path }),
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
  cloneRepo: (serverUrl, repoId, repoName, destParent) =>
    invoke<string>('lore_clone', { serverUrl, repoId, repoName, destParent }),
  loadConfig: () => invoke<AppConfig>('config_load'),
  saveConfig: (config) => invoke<void>('config_save', { config }),
  commitAll: (repoPath, message, exclude) => invoke<void>('lore_commit', { repoPath, message, exclude }),
  push: (repoPath) => invoke<void>('lore_push', { repoPath }),
  sync: (repoPath) => invoke<void>('lore_sync', { repoPath }),
  pushedLockFiles: (repoPath) => invoke<string[]>('lore_pushed_lock_files', { repoPath }),
  setLock: (repoPath, path, lock) => invoke<void>('lore_set_lock', { repoPath, path, lock }),
  discardFile: (repoPath, path) => invoke<void>('lore_discard_file', { repoPath, path }),
  undoCommit: (repoPath, parentRevision) => invoke<void>('lore_undo_commit', { repoPath, parentRevision }),
  amendCommit: (repoPath, message) => invoke<void>('lore_amend', { repoPath, message }),
  getLocks: (repoPath) => invoke<LockEntry[]>('lore_locks', { repoPath }),
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
}
