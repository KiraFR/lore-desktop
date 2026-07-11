import { describe, it, expect } from 'vitest'
import { formatAheadBehind } from './branchInfoCache'

describe('formatAheadBehind', () => {
  it('formats present, non-zero parts only', () => {
    expect(formatAheadBehind({ ahead: 2, behind: 5 })).toBe('↑2 ↓5')
    expect(formatAheadBehind({ ahead: 2, behind: 0 })).toBe('↑2')
    expect(formatAheadBehind({ behind: 3 })).toBe('↓3')
    expect(formatAheadBehind({ ahead: 0, behind: 0 })).toBeNull()
    expect(formatAheadBehind(undefined)).toBeNull()
  })
})
