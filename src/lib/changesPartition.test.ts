import { describe, it, expect } from 'vitest'
import { partitionByLock, filterByQuery } from './changesPartition'
import type { ChangedFile } from './types'

const f = (path: string, lockedBy?: string | null): ChangedFile =>
  ({ path, action: 'modify', isBinary: false, size: 10, lockedBy })

describe('partitionByLock', () => {
  it('keeps unlocked and self-locked files committable', () => {
    const { committable, lockedByOthers } = partitionByLock([f('a'), f('b', null), f('c', 'you')])
    expect(committable.map((x) => x.path)).toEqual(['a', 'b', 'c'])
    expect(lockedByOthers).toEqual([])
  })
  it('moves teammate-locked files to the locked group, order preserved', () => {
    const { committable, lockedByOthers } = partitionByLock([f('a', 'Maya R'), f('b'), f('c', 'Alex L')])
    expect(committable.map((x) => x.path)).toEqual(['b'])
    expect(lockedByOthers.map((x) => x.path)).toEqual(['a', 'c'])
  })
  it('the commit counter counts committables only', () => {
    const { committable } = partitionByLock([f('a', 'Maya R'), f('b'), f('c', 'you')])
    expect(committable.length).toBe(2)
  })
})

describe('filterByQuery', () => {
  const files = [f('Content/Maps/Level_01.umap'), f('Source/Player.cpp', 'Maya R')]
  it('returns everything for a blank query', () => {
    expect(filterByQuery(files, '  ')).toEqual(files)
  })
  it('matches case-insensitively anywhere in the path', () => {
    expect(filterByQuery(files, 'PLAYER').map((x) => x.path)).toEqual(['Source/Player.cpp'])
  })
  it('applies to locked files too (the filter spans both groups)', () => {
    const { lockedByOthers } = partitionByLock(filterByQuery(files, 'player'))
    expect(lockedByOthers.map((x) => x.path)).toEqual(['Source/Player.cpp'])
  })
})
