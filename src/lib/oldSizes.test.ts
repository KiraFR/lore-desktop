import { describe, it, expect } from 'vitest'
import { mergeOldSizes, sizeLookupPaths } from './oldSizes'
import type { ChangedFile } from './types'

const f = (path: string, action: ChangedFile['action'], size = 100): ChangedFile =>
  ({ path, action, isBinary: false, size })

describe('sizeLookupPaths', () => {
  it('keeps only modify and delete', () => {
    const files = [f('a.txt', 'add'), f('b.txt', 'modify'), f('c.txt', 'delete'), f('d.txt', 'move')]
    expect(sizeLookupPaths(files)).toEqual(['b.txt', 'c.txt'])
  })
})

describe('mergeOldSizes', () => {
  it('annotates modify and delete rows with oldSize', () => {
    const out = mergeOldSizes([f('a.txt', 'modify', 120), f('b.txt', 'delete', 0)], { 'a.txt': 100, 'b.txt': 3400 })
    expect(out[0].oldSize).toBe(100)
    expect(out[1].oldSize).toBe(3400)
  })
  it('never annotates adds, even if a size is reported', () => {
    const out = mergeOldSizes([f('a.txt', 'add')], { 'a.txt': 50 })
    expect(out[0].oldSize).toBeUndefined()
  })
  it('leaves files without a reported size untouched', () => {
    const out = mergeOldSizes([f('a.txt', 'modify')], {})
    expect(out[0].oldSize).toBeUndefined()
  })
  it('ignores reported paths that vanished from the list (status/file-info race)', () => {
    const out = mergeOldSizes([f('a.txt', 'modify', 120)], { 'a.txt': 100, 'gone.txt': 999 })
    expect(out).toHaveLength(1)
    expect(out[0].oldSize).toBe(100)
  })
})
