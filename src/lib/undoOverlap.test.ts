import { describe, it, expect } from 'vitest'
import { overlappingPaths, undoConfirmMessage, MAX_LISTED_OVERLAPS } from './undoOverlap'

const cf = (...paths: string[]) => paths.map((path) => ({ path }))

describe('overlappingPaths', () => {
  it('is empty when the commit has no files', () => {
    expect(overlappingPaths([], ['a.cpp'])).toEqual([])
  })

  it('is empty when nothing is pending', () => {
    expect(overlappingPaths(cf('a.cpp', 'b.cpp'), [])).toEqual([])
  })

  it('is empty when the sets are disjoint', () => {
    expect(overlappingPaths(cf('a.cpp', 'b.cpp'), ['c.cpp', 'd.cpp'])).toEqual([])
  })

  it('returns the intersection in commit-file order', () => {
    const commit = cf('Source/a.cpp', 'Config/b.ini', 'Content/c.uasset')
    const pending = ['Content/c.uasset', 'Other/x.png', 'Source/a.cpp']
    expect(overlappingPaths(commit, pending)).toEqual(['Source/a.cpp', 'Content/c.uasset'])
  })
})

describe('undoConfirmMessage', () => {
  const base = 'Undo the commit "fix stuff"? Its changes go back to Changes (nothing is lost).'

  it('without overlap it is the usual undo message, unchanged', () => {
    expect(undoConfirmMessage('fix stuff', [])).toBe(base)
  })

  it('with one overlap it warns in the singular and lists the path', () => {
    expect(undoConfirmMessage('fix stuff', ['Source/a.cpp'])).toBe(
      `Undoing this commit will overwrite your pending changes to 1 file: Source/a.cpp.\n\n${base}`,
    )
  })

  it('lists up to MAX_LISTED_OVERLAPS paths without an "and N more" tail', () => {
    const overlap = ['a', 'b', 'c', 'd', 'e']
    expect(overlap).toHaveLength(MAX_LISTED_OVERLAPS)
    expect(undoConfirmMessage('m', overlap)).toBe(
      `Undoing this commit will overwrite your pending changes to 5 files: a, b, c, d, e.\n\n` +
      'Undo the commit "m"? Its changes go back to Changes (nothing is lost).',
    )
  })

  it('truncates beyond MAX_LISTED_OVERLAPS with "and N more"', () => {
    const overlap = ['a', 'b', 'c', 'd', 'e', 'f', 'g']
    expect(undoConfirmMessage('m', overlap)).toBe(
      `Undoing this commit will overwrite your pending changes to 7 files: a, b, c, d, e and 2 more.\n\n` +
      'Undo the commit "m"? Its changes go back to Changes (nothing is lost).',
    )
  })
})
