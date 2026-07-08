import { describe, it, expect, beforeEach } from 'vitest'
import { mock } from './mock'

describe('mock api', () => {
  beforeEach(async () => {
    localStorage.clear()
    await mock.signOut()
  })

  it('starts signed out, then signs in', async () => {
    expect(await mock.isAuthenticated()).toBe(false)
    await mock.signIn('lore://demo:41337')
    expect(await mock.isAuthenticated()).toBe(true)
  })

  it('lists fake repos', async () => {
    const repos = await mock.listRepos('lore://demo:41337')
    expect(repos.length).toBeGreaterThan(0)
    expect(repos[0]).toHaveProperty('name')
  })

  it('getStatus returns a branch + files', async () => {
    const s = await mock.getStatus('C:/repos/game')
    expect(s.branch).toBe('main')
    expect(s.files.length).toBeGreaterThan(0)
    expect(s.files.some((f) => f.isBinary)).toBe(true)
  })

  it('commit clears files and bumps ahead; push zeroes ahead', async () => {
    const before = await mock.getStatus('C:/repos/game')
    expect(before.files.length).toBeGreaterThan(0)
    await mock.commitAll('C:/repos/game', 'my commit')
    const afterCommit = await mock.getStatus('C:/repos/game')
    expect(afterCommit.files.length).toBe(0)
    expect(afterCommit.localAhead).toBe(before.localAhead + 1)
    await mock.push('C:/repos/game')
    const afterPush = await mock.getStatus('C:/repos/game')
    expect(afterPush.localAhead).toBe(0)
  })

  it('persists config to localStorage', async () => {
    await mock.saveConfig({ serverUrl: 'lore://x:1', currentRepo: 'C:/r', recentRepos: ['C:/r'] })
    const cfg = await mock.loadConfig()
    expect(cfg.serverUrl).toBe('lore://x:1')
    expect(cfg.currentRepo).toBe('C:/r')
  })

  it('getHistory paginates by length + cursor', async () => {
    const p1 = await mock.getHistory('game-main', 10)
    expect(p1.commits).toHaveLength(10)
    expect(p1.nextCursor).not.toBeNull()
    const p2 = await mock.getHistory('game-main', 10, p1.nextCursor!)
    expect(p2.commits[0].id).not.toBe(p1.commits[0].id)
  })

  it('pickFolder returns a path; cloneRepo returns dest/name', async () => {
    const picked = await mock.pickFolder()
    expect(typeof picked).toBe('string')
    const cloned = await mock.cloneRepo('lore://demo:41337', 'id1', 'game-main', 'C:/repos')
    expect(cloned).toBe('C:/repos/game-main')
  })

  it('getBranches returns name+current rows (one current)', async () => {
    const branches = await mock.getBranches('C:/repos/game')
    expect(branches.length).toBeGreaterThan(0)
    expect(branches[0]).toHaveProperty('name')
    expect(branches[0]).not.toHaveProperty('rev')
    expect(branches.filter((b) => b.current)).toHaveLength(1)
  })

  it('getCommitFiles returns a commit’s files', async () => {
    const page = await mock.getHistory('game-main', 5)
    const files = await mock.getCommitFiles('game-main', page.commits[0].id, page.commits[0].parents[0] ?? '')
    expect(files.length).toBeGreaterThan(0)
    expect(files[0]).toHaveProperty('path')
    expect(files[0]).toHaveProperty('action')
  })

  it('pushedLockFiles returns the paths I hold locked', async () => {
    const files = await mock.pushedLockFiles('C:/repos/game')
    expect(Array.isArray(files)).toBe(true)
    expect(files).toContain('Content/Maps/Level_01.umap')
  })

  it('previewMerge returns file + conflict counts', async () => {
    const clean = await mock.previewMerge('C:/repos/game', 'main', 'feature/loot')
    expect(typeof clean.files).toBe('number')
    expect(typeof clean.conflicts).toBe('number')
    const conflicting = await mock.previewMerge('C:/repos/game', 'feature/loot', 'main')
    expect(conflicting.conflicts).toBeGreaterThan(0)
  })

  it('merge conflict flow: start → resolve → commit clears conflicts', async () => {
    await mock.mergeStart('C:/repos/game', 'feature/loot')
    let conflicts = await mock.mergeConflicts('C:/repos/game')
    expect(conflicts.length).toBeGreaterThan(0)
    expect(conflicts.every((c) => c.unresolved)).toBe(true)
    await mock.mergeResolve('C:/repos/game', conflicts[0].path, 'theirs')
    conflicts = await mock.mergeConflicts('C:/repos/game')
    expect(conflicts.find((c) => !c.unresolved)).toBeTruthy()
    await mock.mergeCommit('C:/repos/game', 'merge')
    expect(await mock.mergeConflicts('C:/repos/game')).toHaveLength(0)
  })

  it('getDiff returns structured diff lines', async () => {
    const d = await mock.getDiff('C:/repos/game', 'src/x.ts')
    expect(d.length).toBeGreaterThan(0)
    expect(d.some((l) => l.kind === 'add')).toBe(true)
    expect(d[0]).toHaveProperty('text')
  })
})
