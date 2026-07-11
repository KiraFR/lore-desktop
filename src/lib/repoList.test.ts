import { describe, it, expect } from 'vitest'
import { repoName, promoteRepo, removeRepoPath, replaceRepoPath, nextCurrentRepo, filterRepos } from './repoList'

describe('repoName', () => {
  it('returns the folder basename for Windows paths', () => {
    expect(repoName('C:\\SoonerOrLater\\game-main')).toBe('game-main')
  })
  it('returns the folder basename for POSIX paths', () => {
    expect(repoName('/home/jd/repos/audio')).toBe('audio')
  })
  it('ignores a trailing separator', () => {
    expect(repoName('C:/SoonerOrLater/game-main/')).toBe('game-main')
  })
})

describe('promoteRepo', () => {
  it('prepends a new path', () => {
    expect(promoteRepo(['a', 'b'], 'c')).toEqual(['c', 'a', 'b'])
  })
  it('moves an existing path to the front without duplicating it', () => {
    expect(promoteRepo(['a', 'b', 'c'], 'b')).toEqual(['b', 'a', 'c'])
  })
  it('keeps a front-most path in place', () => {
    expect(promoteRepo(['a', 'b'], 'a')).toEqual(['a', 'b'])
  })
})

describe('removeRepoPath', () => {
  it('drops the path', () => {
    expect(removeRepoPath(['a', 'b', 'c'], 'b')).toEqual(['a', 'c'])
  })
  it('is a no-op for an unknown path', () => {
    expect(removeRepoPath(['a'], 'zzz')).toEqual(['a'])
  })
})

describe('replaceRepoPath', () => {
  it('swaps the old path for the new one in place, preserving order', () => {
    expect(replaceRepoPath(['a', 'b', 'c'], 'b', 'B')).toEqual(['a', 'B', 'c'])
  })
  it('dedups when the new path already exists in the list', () => {
    expect(replaceRepoPath(['a', 'b', 'c'], 'c', 'a')).toEqual(['a', 'b'])
  })
  it('is a no-op when the old path is absent', () => {
    expect(replaceRepoPath(['a', 'b'], 'zzz', 'c')).toEqual(['a', 'b'])
  })
})

describe('nextCurrentRepo', () => {
  it('keeps the current repo when another one was removed', () => {
    expect(nextCurrentRepo('a', 'b', ['a', 'c'])).toBe('a')
  })
  it('falls back to the most recent remaining repo when the current one was removed', () => {
    expect(nextCurrentRepo('a', 'a', ['b', 'c'])).toBe('b')
  })
  it('returns null when the last repo was removed', () => {
    expect(nextCurrentRepo('a', 'a', [])).toBeNull()
  })
})

describe('filterRepos', () => {
  const list = ['C:/SoonerOrLater/game-main', 'C:/SoonerOrLater/game-assets', 'D:/other/audio']
  it('returns everything for a blank query', () => {
    expect(filterRepos(list, '  ')).toEqual(list)
  })
  it('matches the repo name case-insensitively', () => {
    expect(filterRepos(list, 'AUDIO')).toEqual(['D:/other/audio'])
  })
  it('matches anywhere in the path', () => {
    expect(filterRepos(list, 'soonerorlater')).toEqual(['C:/SoonerOrLater/game-main', 'C:/SoonerOrLater/game-assets'])
  })
})
