import { describe, it, expect } from 'vitest'
import { filterCommits } from './historyFilter'
import type { Commit } from './types'

const c = (over: Pick<Commit, 'id' | 'rev' | 'message' | 'author'>): Commit => ({
  when: '1 min ago', whenMs: 0, lane: 0, parents: [], files: [], ...over,
})

const commits = [
  c({ id: 'cafe12', rev: 421, message: 'Fix player movement', author: 'Maya R' }),
  c({ id: 'cbb001', rev: 42, message: 'Add loot tables', author: 'you' }),
  c({ id: 'cbb002', rev: 7, message: 'Tune audio mix', author: 'Alex L' }),
]

describe('filterCommits', () => {
  it('returns the same array for a blank or whitespace query', () => {
    expect(filterCommits(commits, '')).toBe(commits)
    expect(filterCommits(commits, '   ')).toBe(commits)
  })
  it('matches the message case-insensitively', () => {
    expect(filterCommits(commits, 'PLAYER').map((x) => x.rev)).toEqual([421])
    expect(filterCommits(commits, 'loot').map((x) => x.rev)).toEqual([42])
  })
  it('preserves commit order across multiple matches', () => {
    // 'i' hits "Fix player movement" and "Tune audio mix", not "Add loot tables".
    expect(filterCommits(commits, 'i').map((x) => x.rev)).toEqual([421, 7])
  })
  it('matches the author case-insensitively', () => {
    expect(filterCommits(commits, 'maya').map((x) => x.rev)).toEqual([421])
    expect(filterCommits(commits, 'ALEX').map((x) => x.rev)).toEqual([7])
  })
  it('matches the short hash case-insensitively', () => {
    expect(filterCommits(commits, 'AFE1').map((x) => x.rev)).toEqual([421])
    expect(filterCommits(commits, 'cbb0').map((x) => x.rev)).toEqual([42, 7])
  })
  it('matches revision numbers by prefix — "42" hits #421 and #42', () => {
    expect(filterCommits(commits, '42').map((x) => x.rev)).toEqual([421, 42])
  })
  it('accepts the "#N" form with the same prefix semantics', () => {
    expect(filterCommits(commits, '#42').map((x) => x.rev)).toEqual([421, 42])
    expect(filterCommits(commits, '#421').map((x) => x.rev)).toEqual([421])
    expect(filterCommits(commits, '#7').map((x) => x.rev)).toEqual([7])
  })
  it('a digit query also hits hashes by substring (criteria are OR-ed)', () => {
    expect(filterCommits(commits, '001').map((x) => x.rev)).toEqual([42])
  })
  it('rejects non-matches: unknown text, "#" alone, "#" + non-digits, too-long rev', () => {
    expect(filterCommits(commits, 'zzz')).toEqual([])
    expect(filterCommits(commits, '#')).toEqual([])
    expect(filterCommits(commits, '#abc')).toEqual([])
    expect(filterCommits(commits, '#4211')).toEqual([])
  })
})
