import type { RepositoryInfo } from './types'

export interface AboutRow {
  label: string
  value: string
  /** Copy button (e.g. the repo id). */
  copyable?: boolean
  /** Reveal button opening the file manager at this absolute path. */
  revealPath?: string
}

export interface AboutContext {
  repoPath: string | null
  serverUrl: string | null
  branch: string | null
  revisionNumber: number | null
}

/** Epoch SECONDS → 'YYYY-MM-DD' (UTC, locale-stable). Empty for a bad value. */
function fmtCreated(sec: number): string {
  if (!Number.isFinite(sec) || sec <= 0) return ''
  return new Date(sec * 1000).toISOString().slice(0, 10)
}

/**
 * About-panel rows. Absent data = hidden row, never a blank or a fake zero
 * (safe defaults — `info` may be null when the server is unreachable).
 */
export function aboutRows(info: RepositoryInfo | null, ctx: AboutContext): AboutRow[] {
  const rows: AboutRow[] = []
  const name = info?.name ?? ctx.repoPath?.split(/[\\/]/).pop() ?? null
  if (name) rows.push({ label: 'Name', value: name })
  if (info?.id) rows.push({ label: 'Repository id', value: info.id, copyable: true })
  if (info?.description) rows.push({ label: 'Description', value: info.description })
  if (ctx.repoPath) rows.push({ label: 'Local path', value: ctx.repoPath, revealPath: ctx.repoPath })
  const server = info?.remoteUrl ?? ctx.serverUrl
  if (server) rows.push({ label: 'Server', value: server })
  if (info?.defaultBranchName) rows.push({ label: 'Default branch', value: info.defaultBranchName })
  if (ctx.branch) rows.push({ label: 'Current branch', value: ctx.branch })
  if (ctx.revisionNumber != null && ctx.revisionNumber > 0) rows.push({ label: 'Revision', value: `#${ctx.revisionNumber}` })
  if (info?.created != null) {
    const c = fmtCreated(info.created)
    if (c) rows.push({ label: 'Created', value: c })
  }
  return rows
}
