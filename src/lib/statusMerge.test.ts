import { describe, it, expect } from 'vitest'
import { mergeStatus, mergeStatusFiles, annotateLocks } from './statusMerge'
import type { ChangedFile, StatusResult } from './types'

const file = (path: string, over: Partial<ChangedFile> = {}): ChangedFile =>
  ({ path, action: 'modify', isBinary: false, size: 100, ...over })

const status = (files: ChangedFile[], over: Partial<StatusResult> = {}): StatusResult => ({
  branch: 'main', localAhead: 0, remoteAhead: 0, revisionNumber: 5, localRevisionNumber: 5,
  remoteAvailable: true, remoteAuthorized: true, mergeInProgress: false, stagedPending: false,
  summary: { adds: 0, mods: files.length, dels: 0 }, ignoredCount: 0, files, ...over,
})

describe('mergeStatusFiles', () => {
  it('returns the previous ARRAY when every row is unchanged', () => {
    const prev = [file('a.txt'), file('b.txt', { action: 'add', size: 7 })]
    const next = [file('a.txt'), file('b.txt', { action: 'add', size: 7 })]
    expect(mergeStatusFiles(prev, next)).toBe(prev)
  })

  it('keeps the previous object for an unchanged row when another row changed', () => {
    const prev = [file('a.txt'), file('b.txt')]
    const next = [file('a.txt'), file('b.txt', { size: 999 })]
    const out = mergeStatusFiles(prev, next)
    expect(out).not.toBe(prev)
    expect(out[0]).toBe(prev[0])
    expect(out[1]).not.toBe(prev[1])
    expect(out[1].size).toBe(999)
  })

  it('a changed action or isBinary produces a new object', () => {
    const prev = [file('a.txt'), file('b.txt')]
    const next = [file('a.txt', { action: 'delete' }), file('b.txt', { isBinary: true })]
    const out = mergeStatusFiles(prev, next)
    expect(out[0]).not.toBe(prev[0])
    expect(out[1]).not.toBe(prev[1])
  })

  it('an added row is appended without churning the existing ones', () => {
    const prev = [file('a.txt')]
    const next = [file('a.txt'), file('new.txt', { action: 'add' })]
    const out = mergeStatusFiles(prev, next)
    expect(out).not.toBe(prev)
    expect(out[0]).toBe(prev[0])
    expect(out[1]).toBe(next[1])
  })

  it('a removed row yields a new array, survivors keep their identity', () => {
    const prev = [file('a.txt'), file('b.txt')]
    const next = [file('b.txt')]
    const out = mergeStatusFiles(prev, next)
    expect(out).not.toBe(prev)
    expect(out).toHaveLength(1)
    expect(out[0]).toBe(prev[1])
  })

  it('a reorder yields a new array but preserves per-path identity', () => {
    const prev = [file('a.txt'), file('b.txt')]
    const next = [file('b.txt'), file('a.txt')]
    const out = mergeStatusFiles(prev, next)
    expect(out).not.toBe(prev)
    expect(out[0]).toBe(prev[1])
    expect(out[1]).toBe(prev[0])
  })

  it('keeps enrichments (oldSize, lockedBy) the fresh status does not carry', () => {
    const prev = [file('a.txt', { oldSize: 90, lockedBy: 'you' })]
    const next = [file('a.txt')] // fresh from the wire: no enrichments
    expect(mergeStatusFiles(prev, next)).toBe(prev)
  })

  it('keeps enrichments on a row that changed for another reason', () => {
    const prev = [file('a.txt', { oldSize: 90, lockedBy: 'you' })]
    const next = [file('a.txt', { size: 500 })]
    const out = mergeStatusFiles(prev, next)
    expect(out[0]).not.toBe(prev[0])
    expect(out[0]).toMatchObject({ size: 500, oldSize: 90, lockedBy: 'you' })
  })

  it('an EXPLICIT lockedBy on the wire is authoritative (null releases)', () => {
    const prev = [file('a.txt', { lockedBy: 'you' })]
    const next = [file('a.txt', { lockedBy: null })]
    const out = mergeStatusFiles(prev, next)
    expect(out[0]).not.toBe(prev[0])
    expect(out[0].lockedBy).toBeNull()
  })

  it('treats absent and null lockedBy as the same (both mean unlocked)', () => {
    const prev = [file('a.txt', { lockedBy: null })]
    const next = [file('a.txt')]
    expect(mergeStatusFiles(prev, next)).toBe(prev)
  })
})

describe('mergeStatus', () => {
  it('returns next as-is when there is no previous status', () => {
    const next = status([file('a.txt')])
    expect(mergeStatus(null, next)).toBe(next)
  })

  it('returns the PREVIOUS status object for a fully no-op refresh', () => {
    const prev = status([file('a.txt'), file('b.txt', { action: 'add' })])
    const next = status([file('a.txt'), file('b.txt', { action: 'add' })])
    expect(mergeStatus(prev, next)).toBe(prev)
  })

  it('a top-level change keeps the UNCHANGED files array by reference', () => {
    const prev = status([file('a.txt')])
    const next = status([file('a.txt')], { remoteAhead: 2 })
    const out = mergeStatus(prev, next)
    expect(out).not.toBe(prev)
    expect(out.remoteAhead).toBe(2)
    expect(out.files).toBe(prev.files)
  })

  it('detects a summary change by value', () => {
    const prev = status([file('a.txt')])
    const next = status([file('a.txt')], { summary: { adds: 1, mods: 0, dels: 0 } })
    expect(mergeStatus(prev, next)).not.toBe(prev)
  })

  it('summary equal by value (fresh object) is still a no-op', () => {
    const prev = status([file('a.txt')], { summary: { adds: 1, mods: 2, dels: 3 } })
    const next = status([file('a.txt')], { summary: { adds: 1, mods: 2, dels: 3 } })
    expect(mergeStatus(prev, next)).toBe(prev)
  })

  it('summary appearing or vanishing (older CLI) is a change', () => {
    const withSum = status([file('a.txt')])
    const without = status([file('a.txt')], { summary: undefined })
    expect(mergeStatus(withSum, without)).not.toBe(withSum)
    expect(mergeStatus(without, status([file('a.txt')]))).not.toBe(without)
  })

  it('both summaries absent is a no-op', () => {
    const prev = status([file('a.txt')], { summary: undefined })
    const next = status([file('a.txt')], { summary: undefined })
    expect(mergeStatus(prev, next)).toBe(prev)
  })

  it('every scalar field participates in the comparison', () => {
    const base = status([file('a.txt')])
    const variants: Partial<StatusResult>[] = [
      { branch: 'dev' }, { localAhead: 1 }, { remoteAhead: 1 }, { revisionNumber: 4 },
      { localRevisionNumber: 6 }, { remoteAvailable: false }, { remoteAuthorized: false },
      { mergeInProgress: true }, { stagedPending: true }, { ignoredCount: 3 },
    ]
    for (const v of variants) {
      const out = mergeStatus(base, status([file('a.txt')], v))
      expect(out, JSON.stringify(v)).not.toBe(base)
      expect(out.files).toBe(base.files) // the files still merge by reference
    }
  })

  it('a file change flows through into a new status with merged files', () => {
    const prev = status([file('a.txt'), file('b.txt')])
    const next = status([file('a.txt'), file('b.txt', { size: 1 })])
    const out = mergeStatus(prev, next)
    expect(out).not.toBe(prev)
    expect(out.files[0]).toBe(prev.files[0])
    expect(out.files[1].size).toBe(1)
  })
})

describe('annotateLocks', () => {
  it('returns the same array when no holder changed', () => {
    const files = [file('a.txt', { lockedBy: 'you' }), file('b.txt', { lockedBy: null })]
    const out = annotateLocks(files, new Map([['a.txt', 'you']]))
    expect(out).toBe(files)
  })

  it('treats absent lockedBy as null (no churn on unlocked rows)', () => {
    const files = [file('a.txt')]
    expect(annotateLocks(files, new Map())).toBe(files)
  })

  it('only the row whose holder changed gets a new object', () => {
    const files = [file('a.txt', { lockedBy: 'you' }), file('b.txt')]
    const out = annotateLocks(files, new Map([['a.txt', 'you'], ['b.txt', 'Maya R']]))
    expect(out).not.toBe(files)
    expect(out[0]).toBe(files[0])
    expect(out[1]).not.toBe(files[1])
    expect(out[1].lockedBy).toBe('Maya R')
  })

  it('a released lock annotates back to null', () => {
    const files = [file('a.txt', { lockedBy: 'Maya R' })]
    const out = annotateLocks(files, new Map())
    expect(out[0].lockedBy).toBeNull()
  })
})
