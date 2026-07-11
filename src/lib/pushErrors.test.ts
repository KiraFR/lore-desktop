import { describe, it, expect } from 'vitest'
import { isNonFastForwardPush, errorMessage, NON_FF_PUSH_SAMPLE } from './pushErrors'

describe('isNonFastForwardPush', () => {
  it('recognizes the captured non-fast-forward refusal', () => {
    // NON_FF_PUSH_SAMPLE is a verbatim copy of the message check_ok returns for
    // the fixture push_nonff.ndjson (pinned on the real CLI capture).
    expect(isNonFastForwardPush(NON_FF_PUSH_SAMPLE)).toBe(true)
  })
  it('rejects every other failure shape (no broad match)', () => {
    expect(isNonFastForwardPush('lore exited with status 1')).toBe(false)
    expect(isNonFastForwardPush('lore made no progress for 60 s — operation aborted')).toBe(false)
    expect(isNonFastForwardPush('failed to launch lore: program not found')).toBe(false)
    expect(isNonFastForwardPush('{"errorInner":"not authorized"}')).toBe(false)
    expect(isNonFastForwardPush('Push failed')).toBe(false)
    expect(isNonFastForwardPush('')).toBe(false)
  })
})

describe('errorMessage', () => {
  it('unwraps Error and stringifies the rest (tauri rejects with a plain string)', () => {
    expect(errorMessage(new Error('boom'))).toBe('boom')
    expect(errorMessage('plain string')).toBe('plain string')
    expect(errorMessage(42)).toBe('42')
  })
})
