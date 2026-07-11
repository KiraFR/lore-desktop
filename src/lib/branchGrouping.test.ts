import { describe, it, expect } from 'vitest'
import { groupBranches } from './branchGrouping'
import type { Branch } from './types'

const b = (name: string, location?: 'local' | 'remote', current = false): Branch => ({ name, current, location })

describe('groupBranches', () => {
  const list = [b('main', 'local', true), b('feature/loot', 'local'), b('release/srv', 'remote'), b('hotfix/srv', 'remote')]

  it('locals first, then a Remote header, then remote-only branches', () => {
    const rows = groupBranches(list, '')
    expect(rows.map((r) => (r.kind === 'header' ? '§' + r.label : r.branch.name)))
      .toEqual(['main', 'feature/loot', '§Remote', 'release/srv', 'hotfix/srv'])
  })

  it('emits no header when there is no remote-only branch', () => {
    const rows = groupBranches([b('main', 'local', true), b('feature/loot', 'local')], '')
    expect(rows.every((r) => r.kind === 'branch')).toBe(true)
  })

  it('the filter applies to both groups and drops an emptied Remote section', () => {
    const onlyLocal = groupBranches(list, 'loot')
    expect(onlyLocal).toEqual([{ kind: 'branch', branch: b('feature/loot', 'local') }])
    const onlyRemote = groupBranches(list, 'srv')
    expect(onlyRemote[0]).toEqual({ kind: 'header', label: 'Remote' })
    expect(onlyRemote).toHaveLength(3)
  })

  it('a missing location counts as local (safe default: no phantom Remote section)', () => {
    const rows = groupBranches([b('legacy', undefined, true)], '')
    expect(rows).toEqual([{ kind: 'branch', branch: b('legacy', undefined, true) }])
  })
})
