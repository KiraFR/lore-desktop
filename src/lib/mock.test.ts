import { describe, it, expect, beforeEach } from 'vitest'
import { mock } from './mock'
import type { OpProgress } from './types'

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
    await mock.commitAll('C:/repos/game', 'my commit', [])
    const afterCommit = await mock.getStatus('C:/repos/game')
    expect(afterCommit.files.length).toBe(0)
    expect(afterCommit.localAhead).toBe(before.localAhead + 1)
    await mock.push('C:/repos/game')
    const afterPush = await mock.getStatus('C:/repos/game')
    expect(afterPush.localAhead).toBe(0)
  })

  it('discardFile removes the file from changes', async () => {
    const before = await mock.getStatus('C:/repos/d')
    const path = before.files[0].path
    await mock.discardFile('C:/repos/d', path)
    const after = await mock.getStatus('C:/repos/d')
    expect(after.files.some((f) => f.path === path)).toBe(false)
  })

  it('undoCommit lowers ahead and brings changes back to pending', async () => {
    await mock.commitAll('C:/repos/u', 'c', [])
    const committed = await mock.getStatus('C:/repos/u')
    expect(committed.localAhead).toBeGreaterThan(0)
    await mock.undoCommit('C:/repos/u', 'parent-rev')
    const after = await mock.getStatus('C:/repos/u')
    expect(after.localAhead).toBe(committed.localAhead - 1)
    expect(after.files.length).toBeGreaterThan(0)
  })

  it('selective commit keeps excluded files pending', async () => {
    const before = await mock.getStatus('C:/repos/x')
    const keep = before.files[0].path
    await mock.commitAll('C:/repos/x', 'partial', [keep])
    const after = await mock.getStatus('C:/repos/x')
    expect(after.files.map((f) => f.path)).toEqual([keep])
  })

  it('getPreview classifies image, audio, and none', async () => {
    const img = await mock.getPreview('C:/repos/game', 'Content/T_Rock.dds')
    expect(img.kind).toBe('image')
    expect(img.url).toMatch(/^data:image\/svg\+xml,/)
    const au = await mock.getPreview('C:/repos/game', 'Audio/hit.wav')
    expect(au.kind).toBe('audio')
    expect(au.url).toMatch(/^data:audio\/wav;base64,/)
    const no = await mock.getPreview('C:/repos/game', 'Source/main.cpp')
    expect(no.kind).toBe('none')
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

  it('mergeStart raises binary AND text conflicts', async () => {
    await mock.mergeStart('C:/repos/mixed', 'feature/loot')
    const conflicts = await mock.mergeConflicts('C:/repos/mixed')
    expect(conflicts.some((c) => c.isBinary)).toBe(true)
    expect(conflicts.some((c) => !c.isBinary)).toBe(true)
    await mock.mergeAbort('C:/repos/mixed')
  })

  it('getPreview serves a ~theirs sidecar as its base type', async () => {
    const p = await mock.getPreview('C:/repos/game', 'Content/UI/T_Icon_Sword.png~theirs')
    expect(p.kind).toBe('image')
  })
})

describe('mock.fileSizes', () => {
  it('returns old sizes for known modified files only', async () => {
    const sizes = await mock.fileSizes('C:/repos/game', [
      'Content/Maps/Level_01.umap',
      'Content/Characters/Hero/SK_Hero.uasset', // add — no old size seeded
    ])
    expect(sizes['Content/Maps/Level_01.umap']).toBe(2100480)
    expect(sizes['Content/Characters/Hero/SK_Hero.uasset']).toBeUndefined()
  })
})

describe('mock op progress', () => {
  it('clone reports increasing ticks and finishes at total', async () => {
    const ticks: OpProgress[] = []
    await mock.cloneRepo('lore://x', 'id1', 'game', 'C:/repos', (p) => ticks.push(p))
    expect(ticks.length).toBeGreaterThan(3)
    for (let i = 1; i < ticks.length; i++) expect(ticks[i].done).toBeGreaterThanOrEqual(ticks[i - 1].done)
    const last = ticks[ticks.length - 1]
    expect(last.total).toBeGreaterThan(0)
    expect(last.done).toBe(last.total)
  })
  it('sync and push tick too, and still mutate the repo state', async () => {
    const syncTicks: OpProgress[] = []
    await mock.sync('C:/repos/prog', (p) => syncTicks.push(p))
    expect(syncTicks.length).toBeGreaterThan(2)
    expect((await mock.getStatus('C:/repos/prog')).remoteAhead).toBe(0)
    const pushTicks: OpProgress[] = []
    await mock.push('C:/repos/prog', (p) => pushTicks.push(p))
    expect(pushTicks.length).toBeGreaterThan(2)
    expect((await mock.getStatus('C:/repos/prog')).localAhead).toBe(0)
  })
  it('progress callbacks stay optional', async () => {
    await expect(mock.sync('C:/repos/noprog')).resolves.toBeUndefined()
  })
})

describe('mock status flags', () => {
  it('reports no merge and no staged state by default', async () => {
    const s = await mock.getStatus('C:/repos/flags')
    expect(s.mergeInProgress).toBe(false)
    expect(s.stagedPending).toBe(false)
  })
  it('reports mergeInProgress while a conflicting merge is open', async () => {
    await mock.mergeStart('C:/repos/flags', 'feature/loot')
    const duringMerge = await mock.getStatus('C:/repos/flags')
    expect(duringMerge.mergeInProgress).toBe(true)
    // Invariant: a merge implies a staged state.
    expect(duringMerge.stagedPending).toBe(true)
    await mock.mergeAbort('C:/repos/flags')
    expect((await mock.getStatus('C:/repos/flags')).mergeInProgress).toBe(false)
  })
  it('external-abort dev lever clears the merge on the next status', async () => {
    await mock.mergeStart('C:/repos/extabort', 'feature/loot')
    localStorage.setItem('loredesktop.mock.externalAbort', '1')
    const s = await mock.getStatus('C:/repos/extabort')
    expect(s.mergeInProgress).toBe(false)
    expect(localStorage.getItem('loredesktop.mock.externalAbort')).toBeNull()
  })
})
