export interface ChangedFile {
  path: string
  action: 'add' | 'modify' | 'delete' | 'move' | 'copy'
  isBinary: boolean
  size: number
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
  loadConfig(): Promise<AppConfig>
  saveConfig(config: AppConfig): Promise<void>
}
