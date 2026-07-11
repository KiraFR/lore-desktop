import { describe, it, expect } from 'vitest'
import { aboutRows } from './aboutFields'

const ctx = {
  repoPath: 'C:/game/main-repo',
  serverUrl: 'lore://lore.example.com:41337',
  branch: 'main',
  revisionNumber: 42,
}

describe('aboutRows', () => {
  it('renders the full set when everything is known', () => {
    const rows = aboutRows(
      { id: 'abc123', name: 'game-main', remoteUrl: 'lore://srv:41337', description: 'Main game repo', defaultBranchName: 'main', created: 1783270930 },
      ctx,
    )
    expect(rows.map((r) => r.label)).toEqual(
      ['Name', 'Repository id', 'Description', 'Local path', 'Server', 'Default branch', 'Current branch', 'Revision', 'Created'])
    expect(rows.find((r) => r.label === 'Repository id')).toMatchObject({ value: 'abc123', copyable: true })
    expect(rows.find((r) => r.label === 'Local path')).toMatchObject({ value: 'C:/game/main-repo', revealPath: 'C:/game/main-repo' })
    // info.remoteUrl wins over ctx.serverUrl for the Server row.
    expect(rows.find((r) => r.label === 'Server')?.value).toBe('lore://srv:41337')
    expect(rows.find((r) => r.label === 'Revision')?.value).toBe('#42')
    expect(rows.find((r) => r.label === 'Created')?.value).toBe('2026-07-05')
  })

  it('hides absent fields instead of showing blanks (safe defaults)', () => {
    const rows = aboutRows(null, { ...ctx, serverUrl: null, revisionNumber: null })
    expect(rows.map((r) => r.label)).toEqual(['Name', 'Local path', 'Current branch'])
    // Server info entirely missing: the name falls back to the folder name.
    expect(rows[0].value).toBe('main-repo')
  })

  it('renders nothing without a repo', () => {
    expect(aboutRows(null, { repoPath: null, serverUrl: null, branch: null, revisionNumber: null })).toEqual([])
  })
})
