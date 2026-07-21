import { describe, it, expect } from 'vitest'
import { parseIgnore, isIgnored, filterIgnored, adjustSummary } from './loreIgnore'

const ignored = (path: string, text: string) => isIgnored(path, parseIgnore(text))

describe('parseIgnore', () => {
  it('skips blank lines, comments, and trims whitespace', () => {
    const rules = parseIgnore('\n# junk dirs\n  Saved/  \n\n   # more\n*.tmp\n')
    expect(rules.map((r) => r.source)).toEqual(['Saved/', '*.tmp'])
  })
  it('handles CRLF input', () => {
    expect(parseIgnore('Saved/\r\n*.tmp\r\n').map((r) => r.source)).toEqual(['Saved/', '*.tmp'])
  })
  it('drops degenerate patterns that compile to nothing', () => {
    expect(parseIgnore('/\n//\n')).toEqual([])
  })
  it('returns no rules for empty text', () => {
    expect(parseIgnore('')).toEqual([])
  })
})

// The normative examples from the design spec (2026-07-21).
describe('isIgnored — spec examples', () => {
  it('`Saved/` ignores Saved/x.txt and Sub/Saved/y.txt', () => {
    expect(ignored('Saved/x.txt', 'Saved/')).toBe(true)
    expect(ignored('Sub/Saved/y.txt', 'Saved/')).toBe(true)
    expect(ignored('SavedGames/x.txt', 'Saved/')).toBe(false)
  })
  it('`*.blend1` ignores at any depth', () => {
    expect(ignored('scene.blend1', '*.blend1')).toBe(true)
    expect(ignored('Art/props/crate.blend1', '*.blend1')).toBe(true)
    expect(ignored('scene.blend', '*.blend1')).toBe(false)
  })
  it('`/Config/*.ini` is root-anchored and single-level', () => {
    expect(ignored('Config/a.ini', '/Config/*.ini')).toBe(true)
    expect(ignored('Sub/Config/a.ini', '/Config/*.ini')).toBe(false)
    expect(ignored('Config/deep/a.ini', '/Config/*.ini')).toBe(false)
  })
  it('`Content/**/Temp` traverses directories', () => {
    expect(ignored('Content/A/B/Temp/f.txt', 'Content/**/Temp')).toBe(true)
    expect(ignored('Content/A/Temp/f.txt', 'Content/**/Temp')).toBe(true)
    expect(ignored('Other/A/Temp/f.txt', 'Content/**/Temp')).toBe(false)
  })
  it('a `.loreignore` rule never ignores .loreignore itself', () => {
    expect(ignored('.loreignore', '.loreignore')).toBe(false)
    expect(ignored('.loreignore', '*')).toBe(false)
    // Non-root .loreignore files are ordinary files and CAN be ignored.
    expect(ignored('Sub/.loreignore', '.loreignore')).toBe(true)
  })
})

describe('isIgnored — glob semantics', () => {
  it('is case-sensitive (v1 choice: rules apply verbatim to on-disk casing)', () => {
    expect(ignored('Saved/x.txt', 'saved/')).toBe(false)
    expect(ignored('a.TMP', '*.tmp')).toBe(false)
  })
  it('`*` and `?` do not cross `/`', () => {
    expect(ignored('Dir/a.tmp', 'Dir*.tmp')).toBe(false)
    expect(ignored('ab/c', 'a?c')).toBe(false)
    expect(ignored('abc.txt', 'a?c.txt')).toBe(true)
    expect(ignored('abbc.txt', 'a?c.txt')).toBe(false)
  })
  it('an interior `/` anchors to the root', () => {
    expect(ignored('Foo/Bar/x.txt', 'Foo/Bar/')).toBe(true)
    expect(ignored('Sub/Foo/Bar/x.txt', 'Foo/Bar/')).toBe(false)
  })
  it('a leading `/` anchors a slashless name', () => {
    expect(ignored('build.log', '/build.log')).toBe(true)
    expect(ignored('Sub/build.log', '/build.log')).toBe(false)
    expect(ignored('Sub/build.log', 'build.log')).toBe(true)
  })
  it('`**` variations', () => {
    expect(ignored('Content/a/b/c.uasset', 'Content/**')).toBe(true)
    expect(ignored('Sub/Temp/f.txt', '**/Temp')).toBe(true)
    expect(ignored('DerivedDataCache/x/y.ddc', 'DerivedDataCache/**/*.ddc')).toBe(true)
  })
  it('regex specials in patterns are escaped, not interpreted', () => {
    expect(ignored('notes(1).txt', 'notes(1).txt')).toBe(true)
    expect(ignored('notesX1Y.txt', 'notes(1).txt')).toBe(false)
    expect(ignored('axtxt', 'a.txt')).toBe(false)
  })
  it('a matched directory name ignores everything below it', () => {
    expect(ignored('Intermediate/Build/Win64/x.obj', 'Intermediate')).toBe(true)
  })
  it('no rules → nothing ignored', () => {
    expect(isIgnored('anything.txt', [])).toBe(false)
  })
})

describe('filterIgnored', () => {
  const files = [
    { path: 'Content/Hero.uasset' },
    { path: 'Saved/autosave.tmp' },
    { path: 'Saved/Logs/game.log' },
    { path: '.loreignore' },
  ]
  it('splits kept vs ignored and never drops .loreignore', () => {
    const { kept, ignored: out } = filterIgnored(files, parseIgnore('Saved/\n*.tmp\n'))
    expect(kept.map((f) => f.path)).toEqual(['Content/Hero.uasset', '.loreignore'])
    expect(out.map((f) => f.path)).toEqual(['Saved/autosave.tmp', 'Saved/Logs/game.log'])
  })
  it('is a pass-through with zero rules', () => {
    const { kept, ignored: out } = filterIgnored(files, [])
    expect(kept).toBe(files)
    expect(out).toEqual([])
  })
})

describe('adjustSummary', () => {
  it('subtracts the ignored files per action (modify/move/copy → mods)', () => {
    const out = adjustSummary({ adds: 2, mods: 3, dels: 1 }, [
      { action: 'add' }, { action: 'modify' }, { action: 'move' }, { action: 'delete' },
    ])
    expect(out).toEqual({ adds: 1, mods: 1, dels: 0 })
  })
  it('keeps an absent summary absent and clamps at zero', () => {
    expect(adjustSummary(undefined, [{ action: 'add' }])).toBeUndefined()
    expect(adjustSummary({ adds: 0, mods: 0, dels: 0 }, [{ action: 'add' }, { action: 'delete' }]))
      .toEqual({ adds: 0, mods: 0, dels: 0 })
  })
  it('returns the summary untouched when nothing was ignored', () => {
    const s = { adds: 1, mods: 2, dels: 3 }
    expect(adjustSummary(s, [])).toBe(s)
  })
})
