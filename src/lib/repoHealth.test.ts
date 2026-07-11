import { describe, it, expect } from 'vitest'
import { missingRepos } from './repoHealth'

describe('missingRepos', () => {
  it('flags exactly the paths whose directory is gone', () => {
    const exists = new Map([['C:/a', true], ['C:/b', false], ['C:/c', true]])
    expect(missingRepos(['C:/a', 'C:/b', 'C:/c'], exists)).toEqual(new Set(['C:/b']))
  })
  it('treats unknown paths as present (no false alarm before the check ran)', () => {
    expect(missingRepos(['C:/a'], new Map())).toEqual(new Set())
  })
})
