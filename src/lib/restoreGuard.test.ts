import { describe, it, expect } from 'vitest'
import { restoreAvailability } from './restoreGuard'

const base = { isCurrent: false, dirtyTree: false, timeTraveled: false, lockHolder: null as string | null }

describe('restoreAvailability', () => {
  it('allows a restore on a clean tree at the tip, file free', () => {
    expect(restoreAvailability(base)).toEqual({ canRestore: true, reason: null, lock: 'free' })
  })
  it('classifies the lock: mine / teammate / free', () => {
    expect(restoreAvailability({ ...base, lockHolder: 'you' }).lock).toBe('mine')
    expect(restoreAvailability({ ...base, lockHolder: 'Maya R' }).lock).toBe('teammate')
    expect(restoreAvailability({ ...base, lockHolder: null }).lock).toBe('free')
  })
  it('a teammate lock does NOT disable the restore (it stays local, just not committable)', () => {
    const r = restoreAvailability({ ...base, lockHolder: 'Maya R' })
    expect(r.canRestore).toBe(true)
    expect(r.lock).toBe('teammate')
  })
  it('disables the current revision (already the working copy)', () => {
    expect(restoreAvailability({ ...base, isCurrent: true })).toMatchObject({ canRestore: false })
    expect(restoreAvailability({ ...base, isCurrent: true }).reason).toMatch(/current/i)
  })
  it('disables on a dirty tree (no stash to park pending work)', () => {
    const r = restoreAvailability({ ...base, dirtyTree: true })
    expect(r.canRestore).toBe(false)
    expect(r.reason).toMatch(/commit or discard/i)
  })
  it('disables while time-traveled (sync back first)', () => {
    const r = restoreAvailability({ ...base, timeTraveled: true })
    expect(r.canRestore).toBe(false)
    expect(r.reason).toMatch(/latest/i)
  })
  it('precedence: current > dirty > time-traveled', () => {
    expect(restoreAvailability({ isCurrent: true, dirtyTree: true, timeTraveled: true, lockHolder: null }).reason).toMatch(/current/i)
    expect(restoreAvailability({ isCurrent: false, dirtyTree: true, timeTraveled: true, lockHolder: null }).reason).toMatch(/commit or discard/i)
  })
})
