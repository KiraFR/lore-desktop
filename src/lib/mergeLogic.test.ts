import { describe, it, expect } from 'vitest'
import { mergeWording, externalAbortStep } from './mergeLogic'

describe('mergeWording', () => {
  it('uses the real source when it is known', () => {
    const w = mergeWording('feature/loot', 'main')
    expect(w.banner).toBe('Merging feature/loot into main')
    expect(w.commitMessage).toBe('Merge feature/loot into main')
    expect(w.done).toBe('feature/loot was merged into main.')
    expect(w.theirsCard).toBe('Theirs · feature/loot')
  })
  it('never shows a guessed name when the source is unknown', () => {
    const w = mergeWording(null, 'main')
    expect(w.banner).toBe('Resolving merge into main')
    expect(w.commitMessage).toBe('Merge into main')
    expect(w.done).toBe('A branch was merged into main.')
    expect(w.theirsCard).toBe('Theirs · incoming')
  })
})

describe('externalAbortStep', () => {
  it('latches once the backend confirms the merge, then flags a later false', () => {
    let s = externalAbortStep(true, true, false)
    expect(s).toEqual({ saw: true, aborted: false })
    s = externalAbortStep(true, false, s.saw)
    expect(s).toEqual({ saw: false, aborted: true })
  })
  it('ignores a false before the merge was ever confirmed (status lag)', () => {
    expect(externalAbortStep(true, false, false)).toEqual({ saw: false, aborted: false })
    expect(externalAbortStep(true, undefined, false)).toEqual({ saw: false, aborted: false })
  })
  it('resets outside the resolving phase', () => {
    expect(externalAbortStep(false, true, true)).toEqual({ saw: false, aborted: false })
  })
})
