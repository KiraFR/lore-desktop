import { describe, it, expect } from 'vitest'
import { fmtSize, formatDelta } from './sizeFormat'

describe('fmtSize', () => {
  it('formats bytes, KB and MB', () => {
    expect(fmtSize(512)).toBe('512 B')
    expect(fmtSize(2048)).toBe('2.0 KB')
    expect(fmtSize(2359296)).toBe('2.3 MB')
  })
})

describe('formatDelta', () => {
  it('shows a signed compact delta for a grown modified file', () => {
    // delta = 1 MiB exactly → "+1.0 MB"
    expect(formatDelta({ action: 'modify', size: 3 * 1048576, oldSize: 2 * 1048576 })).toBe('+1.0 MB')
    // delta = 258 816 B → formatted in KB, not MB
    expect(formatDelta({ action: 'modify', size: 2359296, oldSize: 2100480 })).toBe('+252.8 KB')
  })
  it('shows a signed compact delta for a shrunk modified file', () => {
    expect(formatDelta({ action: 'modify', size: 2 * 1048576, oldSize: 3 * 1048576 })).toBe('−1.0 MB')
  })
  it('returns null when the delta is zero', () => {
    expect(formatDelta({ action: 'modify', size: 100, oldSize: 100 })).toBeNull()
  })
  it('returns null when the old size is unknown', () => {
    expect(formatDelta({ action: 'modify', size: 100 })).toBeNull()
  })
  it('shows the old size alone for a delete (no sign, no arrow)', () => {
    expect(formatDelta({ action: 'delete', size: 0, oldSize: 2097152 })).toBe('2.0 MB')
  })
  it('returns null for a delete without a known old size', () => {
    expect(formatDelta({ action: 'delete', size: 0 })).toBeNull()
  })
  it('returns null for adds (unchanged from today)', () => {
    expect(formatDelta({ action: 'add', size: 4718592, oldSize: 10 })).toBeNull()
  })
})
