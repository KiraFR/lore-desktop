import { describe, it, expect } from 'vitest'
import { reduce, shouldCheck, progressPct, CHECK_INTERVAL_MS, type UpdateState } from './updater'

const idle: UpdateState = { kind: 'idle' }
const checkingAuto: UpdateState = { kind: 'checking', manual: false }
const checkingManual: UpdateState = { kind: 'checking', manual: true }
const available: UpdateState = { kind: 'available', version: '9.9.9', notes: 'notes' }
const downloading: UpdateState = { kind: 'downloading', version: '9.9.9', pct: 40 }

describe('updater reduce — check', () => {
  it('starts a check from idle (auto and manual)', () => {
    expect(reduce(idle, { type: 'check', manual: false })).toEqual(checkingAuto)
    expect(reduce(idle, { type: 'check', manual: true })).toEqual(checkingManual)
  })
  it('re-checks from upToDate and error', () => {
    expect(reduce({ kind: 'upToDate' }, { type: 'check', manual: true })).toEqual(checkingManual)
    expect(reduce({ kind: 'error', message: 'x' }, { type: 'check', manual: false })).toEqual(checkingAuto)
  })
  it('never interrupts an in-flight check, download, or restart', () => {
    expect(reduce(checkingAuto, { type: 'check', manual: true })).toBe(checkingAuto)
    expect(reduce(downloading, { type: 'check', manual: true })).toBe(downloading)
    const ready: UpdateState = { kind: 'ready', version: '9.9.9' }
    expect(reduce(ready, { type: 'check', manual: false })).toBe(ready)
  })
  it('an AUTO check never yanks a visible banner; a MANUAL one may re-confirm it', () => {
    expect(reduce(available, { type: 'check', manual: false })).toBe(available)
    expect(reduce(available, { type: 'check', manual: true })).toEqual(checkingManual)
  })
})

describe('updater reduce — check results', () => {
  it('found → available with version and notes', () => {
    expect(reduce(checkingAuto, { type: 'found', version: '9.9.9', notes: 'notes' })).toEqual(available)
  })
  it('none → upToDate', () => {
    expect(reduce(checkingManual, { type: 'none' })).toEqual({ kind: 'upToDate' })
  })
  it('found/none outside a check are ignored (stale result)', () => {
    expect(reduce(downloading, { type: 'found', version: '1', notes: '' })).toBe(downloading)
    expect(reduce(idle, { type: 'none' })).toBe(idle)
  })
})

describe('updater reduce — silent vs manual errors', () => {
  it('a failed AUTO check goes silently back to idle', () => {
    expect(reduce(checkingAuto, { type: 'failed', message: 'offline' })).toEqual(idle)
  })
  it('a failed MANUAL check surfaces the error', () => {
    expect(reduce(checkingManual, { type: 'failed', message: 'offline' }))
      .toEqual({ kind: 'error', message: 'offline' })
  })
  it('a failed install surfaces the error', () => {
    expect(reduce(downloading, { type: 'failed', message: 'sig mismatch' }))
      .toEqual({ kind: 'error', message: 'sig mismatch' })
  })
})

describe('updater reduce — install cycle', () => {
  it('install from available starts the download at 0%', () => {
    expect(reduce(available, { type: 'install' })).toEqual({ kind: 'downloading', version: '9.9.9', pct: 0 })
  })
  it('install anywhere else is ignored', () => {
    expect(reduce(idle, { type: 'install' })).toBe(idle)
    expect(reduce(downloading, { type: 'install' })).toBe(downloading)
  })
  it('progress updates the pct while downloading, and only then', () => {
    expect(reduce(downloading, { type: 'progress', pct: 80 })).toEqual({ ...downloading, pct: 80 })
    expect(reduce(available, { type: 'progress', pct: 80 })).toBe(available)
  })
  it('installed → ready, keeping the version', () => {
    expect(reduce(downloading, { type: 'installed' })).toEqual({ kind: 'ready', version: '9.9.9' })
    expect(reduce(idle, { type: 'installed' })).toBe(idle)
  })
})

describe('shouldCheck (4 h interval, injected clock)', () => {
  it('always checks when never checked before', () => {
    expect(shouldCheck(123, null)).toBe(true)
  })
  it('does not re-check inside the interval', () => {
    expect(shouldCheck(1000 + CHECK_INTERVAL_MS - 1, 1000)).toBe(false)
  })
  it('re-checks at and past the interval', () => {
    expect(shouldCheck(1000 + CHECK_INTERVAL_MS, 1000)).toBe(true)
    expect(shouldCheck(1000 + CHECK_INTERVAL_MS * 3, 1000)).toBe(true)
  })
})

describe('progressPct', () => {
  it('maps bytes to a rounded percentage', () => {
    expect(progressPct(0, 200)).toBe(0)
    expect(progressPct(50, 200)).toBe(25)
    expect(progressPct(200, 200)).toBe(100)
  })
  it('clamps overshoot (more bytes than announced)', () => {
    expect(progressPct(250, 200)).toBe(100)
  })
  it('returns null when the total is unknown or invalid', () => {
    expect(progressPct(50, 0)).toBeNull()
    expect(progressPct(50, -1)).toBeNull()
    expect(progressPct(50, Number.NaN)).toBeNull()
  })
})
