import { describe, it, expect } from 'vitest'
import { summaryParts } from './statusSummary'

describe('summaryParts', () => {
  it('is empty when the summary is absent (older CLI) or all-zero', () => {
    expect(summaryParts(undefined)).toEqual([])
    expect(summaryParts(null)).toEqual([])
    expect(summaryParts({ adds: 0, mods: 0, dels: 0 })).toEqual([])
  })
  it('renders only the non-zero counters, in +/~/− order', () => {
    expect(summaryParts({ adds: 3, mods: 2, dels: 1 })).toEqual([
      { text: '+3', cls: 'added' },
      { text: '~2', cls: 'modified' },
      { text: '−1', cls: 'deleted' },
    ])
    expect(summaryParts({ adds: 0, mods: 4, dels: 0 })).toEqual([{ text: '~4', cls: 'modified' }])
  })
  it('appends a muted "N ignored" segment when files were filtered', () => {
    expect(summaryParts({ adds: 1, mods: 0, dels: 0 }, 2)).toEqual([
      { text: '+1', cls: 'added' },
      { text: '2 ignored', cls: 'ignored' },
    ])
  })
  it('shows the ignored segment even without a wire summary, never at zero', () => {
    expect(summaryParts(undefined, 3)).toEqual([{ text: '3 ignored', cls: 'ignored' }])
    expect(summaryParts({ adds: 1, mods: 0, dels: 0 }, 0)).toEqual([{ text: '+1', cls: 'added' }])
  })
})
