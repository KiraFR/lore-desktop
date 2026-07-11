import { describe, it, expect } from 'vitest'
import { partitionByLock, filterByQuery, filterByText } from './changesPartition'
import type { ChangedFile, LockEntry } from './types'

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

describe('filterByText', () => {
  const lock = (path: string, holder: string): LockEntry => ({ path, holder, when: '1 h ago' })
  const locks = [
    lock('Content/Maps/Level_01.umap', 'you'),
    lock('Content/Environment/T_Cliff_D.uasset', 'Maya R'),
    lock('Content/Characters/Hero/SK_Hero.uasset', 'Alex L'),
  ]
  const fields = (l: LockEntry) => [l.path, l.holder]

  it('returns the same array for a blank query', () => {
    expect(filterByText(locks, '  ', fields)).toBe(locks)
  })
  it('matches the path case-insensitively', () => {
    expect(filterByText(locks, 'LEVEL', fields).map((l) => l.path)).toEqual(['Content/Maps/Level_01.umap'])
  })
  it('matches the holder too', () => {
    expect(filterByText(locks, 'maya', fields).map((l) => l.holder)).toEqual(['Maya R'])
  })
  it('spans both fields with one query, order preserved', () => {
    expect(filterByText(locks, 'e', fields).map((l) => l.holder)).toEqual(['you', 'Maya R', 'Alex L'])
  })
  it('returns nothing when neither field matches', () => {
    expect(filterByText(locks, 'zzz', fields)).toEqual([])
  })
})
