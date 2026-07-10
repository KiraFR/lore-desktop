import { describe, it, expect } from 'vitest'
import { pct, cloneLabel } from './progress'

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
