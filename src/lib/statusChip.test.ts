import { describe, it, expect } from 'vitest'
import { chipFor } from './statusChip'
import type { StatusResult } from './types'

const base: StatusResult = {
  branch: 'main', localAhead: 0, remoteAhead: 0, revisionNumber: 1, localRevisionNumber: 1,
  remoteAvailable: true, remoteAuthorized: true,
  mergeInProgress: false, stagedPending: false, files: [],
}

describe('chipFor', () => {
  it('returns null with no status', () => {
    expect(chipFor(null)).toBeNull()
  })
  it('returns null when neither flag is set', () => {
    expect(chipFor(base)).toBeNull()
  })
  it('shows the staged chip when only stagedPending is set', () => {
    expect(chipFor({ ...base, stagedPending: true })).toEqual({ kind: 'staged' })
  })
  it('merge takes precedence over staged (a merge implies a staged state)', () => {
    expect(chipFor({ ...base, mergeInProgress: true, stagedPending: true })).toEqual({ kind: 'merge' })
  })
  it('shows the merge chip when only mergeInProgress is set', () => {
    expect(chipFor({ ...base, mergeInProgress: true })).toEqual({ kind: 'merge' })
  })
  it('shows the behind chip when the current revision trails the local head', () => {
    expect(chipFor({ ...base, revisionNumber: 3, localRevisionNumber: 5 })).toEqual({ kind: 'behind', revision: 3 })
  })
  it('no behind chip at the local head', () => {
    expect(chipFor({ ...base, revisionNumber: 5, localRevisionNumber: 5 })).toBeNull()
  })
  it('merge and staged take precedence over behind', () => {
    expect(chipFor({ ...base, revisionNumber: 3, localRevisionNumber: 5, mergeInProgress: true })).toEqual({ kind: 'merge' })
    expect(chipFor({ ...base, revisionNumber: 3, localRevisionNumber: 5, stagedPending: true })).toEqual({ kind: 'staged' })
  })
})
