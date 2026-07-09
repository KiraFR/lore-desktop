import { describe, it, expect } from 'vitest'
import { initialsFor, displayNameFor } from './identity'

describe('initialsFor', () => {
  it('uses the display name words', () => {
    expect(initialsFor('Jimmy D.', 'x@y.z')).toBe('JD')
  })
  it('takes two letters from a single-word display name', () => {
    expect(initialsFor('Zelda', null)).toBe('ZE')
  })
  it('falls back to the email local part, split on separators', () => {
    expect(initialsFor(null, 'jane.doe@studio.dev')).toBe('JD')
  })
  it('takes two letters from an unseparated email local part', () => {
    expect(initialsFor(null, 'jimmy@example.com')).toBe('JI')
  })
  it('is ? when nothing is known', () => {
    expect(initialsFor(null, null)).toBe('?')
    expect(initialsFor('  ', '')).toBe('?')
  })
})

describe('displayNameFor', () => {
  it('prefers the display name', () => {
    expect(displayNameFor('Jimmy D.', 'x@y.z')).toBe('Jimmy D.')
  })
  it('falls back to the email local part', () => {
    expect(displayNameFor(null, 'jane.doe@studio.dev')).toBe('jane.doe')
  })
  it('reports not signed in otherwise', () => {
    expect(displayNameFor(null, null)).toBe('Not signed in')
  })
})
