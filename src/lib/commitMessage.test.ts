import { describe, it, expect } from 'vitest'
import { composeCommitMessage } from './commitMessage'

describe('composeCommitMessage', () => {
  it('is the summary alone when the description is empty', () => {
    expect(composeCommitMessage('Fix hero mesh', '')).toBe('Fix hero mesh')
    expect(composeCommitMessage('Fix hero mesh', '   ')).toBe('Fix hero mesh')
  })
  it('joins summary and description with a blank line', () => {
    expect(composeCommitMessage(' Fix hero mesh ', ' Rebaked LODs. ')).toBe('Fix hero mesh\n\nRebaked LODs.')
  })
})
