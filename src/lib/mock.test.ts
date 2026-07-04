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
})
