import { describe, it, expect } from 'vitest'
import { ext, typeName, isTextDiffable } from './fileTypes'

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

describe('isTextDiffable', () => {
  it('recognizes source and config files as text-diffable', () => {
    expect(isTextDiffable('Source/Player/PlayerCharacter.cpp')).toBe(true)
    expect(isTextDiffable('Config/DefaultInput.ini')).toBe(true)
    expect(isTextDiffable('notes.md')).toBe(true)
  })
  it('rejects binary/media assets and extensionless files', () => {
    expect(isTextDiffable('Content/Maps/Level_01.umap')).toBe(false)
    expect(isTextDiffable('Audio/sfx.wav')).toBe(false)
    expect(isTextDiffable('noext')).toBe(false)
  })
})
