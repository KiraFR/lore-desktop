import { describe, it, expect } from 'vitest'
import { ext, typeName } from './fileTypes'

describe('ext', () => {
  it('lowercases the extension and handles missing ones', () => {
    expect(ext('Content/T_Rock.PNG')).toBe('png')
    expect(ext('Makefile')).toBe('')
  })
})

describe('typeName', () => {
  it('maps known asset extensions to human names', () => {
    expect(typeName('Content/Maps/Level_01.umap')).toBe('Level (map)')
    expect(typeName('Content/Hero/SK_Hero.uasset')).toBe('Unreal asset')
    expect(typeName('Audio/sfx_hit.wav')).toBe('Audio')
  })
  it('falls back to "<EXT> file" then "File"', () => {
    expect(typeName('data.xyz')).toBe('XYZ file')
    expect(typeName('LICENSE')).toBe('File')
  })
})
