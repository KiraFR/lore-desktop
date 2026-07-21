import { describe, it, expect } from 'vitest'
import { stepPath, rangePaths, stagePartition } from './changesKeyboard'

const order = ['a', 'b', 'c', 'd']

describe('stepPath', () => {
  it('returns null on an empty list', () => {
    expect(stepPath([], null, 1)).toBeNull()
    expect(stepPath([], 'a', -1)).toBeNull()
  })
  it('from no selection, ↓ lands on the first row and ↑ on the last', () => {
    expect(stepPath(order, null, 1)).toBe('a')
    expect(stepPath(order, null, -1)).toBe('d')
  })
  it('a current path filtered out of view behaves like no selection', () => {
    expect(stepPath(order, 'zzz', 1)).toBe('a')
    expect(stepPath(order, 'zzz', -1)).toBe('d')
  })
  it('moves one row in displayed order', () => {
    expect(stepPath(order, 'b', 1)).toBe('c')
    expect(stepPath(order, 'b', -1)).toBe('a')
  })
  it('clamps at the edges instead of wrapping', () => {
    expect(stepPath(order, 'a', -1)).toBe('a')
    expect(stepPath(order, 'd', 1)).toBe('d')
  })
})

describe('rangePaths', () => {
  it('spans anchor to focus inclusive, in displayed order', () => {
    expect(rangePaths(order, 'b', 'd')).toEqual(['b', 'c', 'd'])
  })
  it('works backwards (focus above the anchor)', () => {
    expect(rangePaths(order, 'c', 'a')).toEqual(['a', 'b', 'c'])
  })
  it('collapses to a single row when both ends match', () => {
    expect(rangePaths(order, 'b', 'b')).toEqual(['b'])
  })
  it('returns null when either end is not displayed', () => {
    expect(rangePaths(order, 'zzz', 'b')).toBeNull()
    expect(rangePaths(order, 'b', 'zzz')).toBeNull()
  })
})

describe('stagePartition', () => {
  const committable = ['a', 'b', 'c']
  it('splits the selection into unstaged (to stage) and staged (to unstage)', () => {
    const { toStage, toUnstage } = stagePartition(new Set(['a', 'b', 'c']), committable, new Set(['b']))
    expect(toStage).toEqual(['a', 'c'])
    expect(toUnstage).toEqual(['b'])
  })
  it('ignores rows outside the selection', () => {
    const { toStage, toUnstage } = stagePartition(new Set(['b']), committable, new Set(['b']))
    expect(toStage).toEqual([])
    expect(toUnstage).toEqual(['b'])
  })
  it('never stages teammate-locked files (absent from committable)', () => {
    // 'locked' is selected but not committable — it must not show up anywhere.
    const { toStage, toUnstage } = stagePartition(new Set(['a', 'locked']), committable, new Set())
    expect(toStage).toEqual(['a'])
    expect(toUnstage).toEqual([])
  })
  it('keeps displayed (committable) order regardless of selection order', () => {
    const { toStage } = stagePartition(new Set(['c', 'a']), committable, new Set())
    expect(toStage).toEqual(['a', 'c'])
  })
  it('handles an empty selection', () => {
    expect(stagePartition(new Set(), committable, new Set(['a']))).toEqual({ toStage: [], toUnstage: [] })
  })
})
