import { describe, it, expect } from 'vitest'
import { toggleFilePath, selectionAfterCommitChange, isLocalTip } from './historySelection'

describe('toggleFilePath', () => {
  it('opens a file, switches to another, closes on re-click', () => {
    expect(toggleFilePath(null, 'a.png')).toBe('a.png')
    expect(toggleFilePath('a.png', 'b.png')).toBe('b.png')
    expect(toggleFilePath('a.png', 'a.png')).toBeNull()
  })
})

describe('selectionAfterCommitChange', () => {
  it('keeps the selection on a same-commit refresh, resets on a commit change', () => {
    expect(selectionAfterCommitChange(true, 'a.png')).toBe('a.png')
    expect(selectionAfterCommitChange(false, 'a.png')).toBeNull()
    expect(selectionAfterCommitChange(false, null)).toBeNull()
  })
})

describe('isLocalTip', () => {
  it('is true only for the newest loaded commit', () => {
    const commits = [{ id: 'c2' }, { id: 'c1' }]
    expect(isLocalTip('c2', commits)).toBe(true)
    expect(isLocalTip('c1', commits)).toBe(false)
    expect(isLocalTip('c0', [])).toBe(false)
  })
})
