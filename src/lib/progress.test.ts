import { describe, it, expect } from 'vitest'
import { pct, cloneLabel, cloneProgressLabel, cloneInFlight } from './progress'

describe('pct', () => {
  it('returns a clamped 0-100 percentage when a total exists', () => {
    expect(pct({ done: 512, total: 2048 })).toBe(25)
    expect(pct({ done: 3000, total: 2048 })).toBe(100)
  })
  it('returns null for indeterminate progress', () => {
    expect(pct(null)).toBeNull()
    expect(pct({ done: 10 })).toBeNull()
    expect(pct({ done: 10, total: 0 })).toBeNull()
  })
})

describe('cloneLabel', () => {
  it('appends the percentage when known', () => {
    expect(cloneLabel(42)).toBe('Cloning… 42%')
  })
  it('stays plain when indeterminate', () => {
    expect(cloneLabel(null)).toBe('Cloning…')
  })
})

describe('cloneProgressLabel', () => {
  it('appends done/total sizes for byte-counted progress', () => {
    expect(cloneProgressLabel({ done: 12 * 1024 * 1024, total: 48 * 1024 * 1024, unit: 'bytes' }))
      .toBe('Cloning… 25% — 12.0 MB / 48.0 MB')
  })
  it('falls back to the plain label for non-byte or indeterminate progress', () => {
    expect(cloneProgressLabel({ done: 3, total: 6, unit: 'files' })).toBe('Cloning… 50%')
    expect(cloneProgressLabel({ done: 0 })).toBe('Cloning…')
    expect(cloneProgressLabel(null)).toBe('Cloning…')
  })
})

describe('cloneInFlight', () => {
  it('is true for any occupied slot — including the pre-first-tick sentinel', () => {
    expect(cloneInFlight(null)).toBe(false)
    expect(cloneInFlight({ done: 0 })).toBe(true)
    expect(cloneInFlight({ done: 1, total: 2, unit: 'bytes' })).toBe(true)
  })
})
