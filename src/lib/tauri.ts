import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { mock } from './mock'
import type { AppConfig, Branch, CommitFile, DiffLine, HistoryPage, LockEntry, LoreApi, MergeConflict, MergePreview, RepoEntry, StatusResult } from './types'

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
  cloneRepo: (serverUrl, repoId, repoName, destParent) =>
    invoke<string>('lore_clone', { serverUrl, repoId, repoName, destParent }),
  loadConfig: () => invoke<AppConfig>('config_load'),
  saveConfig: (config) => invoke<void>('config_save', { config }),
  commitAll: (repoPath, message) => invoke<void>('lore_commit', { repoPath, message }),
  push: (repoPath) => invoke<void>('lore_push', { repoPath }),
  sync: (repoPath) => invoke<void>('lore_sync', { repoPath }),
  pushedLockFiles: (repoPath) => invoke<string[]>('lore_pushed_lock_files', { repoPath }),
  setLock: (repoPath, path, lock) => invoke<void>('lore_set_lock', { repoPath, path, lock }),
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
