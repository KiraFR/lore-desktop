import { describe, it, expect, beforeEach } from 'vitest'
import { resolveTheme, applyTheme, cachedTheme, otherTheme } from './theme'

describe('resolveTheme', () => {
  it('defaults undefined/null/dark to dark', () => {
    expect(resolveTheme(undefined)).toBe('dark')
    expect(resolveTheme(null)).toBe('dark')
    expect(resolveTheme('dark')).toBe('dark')
  })

  it('resolves light', () => {
    expect(resolveTheme('light')).toBe('light')
  })
})

describe('applyTheme', () => {
  beforeEach(() => {
    delete document.documentElement.dataset.theme
    localStorage.clear()
  })

  it('sets the data-theme attribute for light', () => {
    applyTheme('light')
    expect(document.documentElement.dataset.theme).toBe('light')
  })

  it('removes the data-theme attribute for dark', () => {
    document.documentElement.dataset.theme = 'light'
    applyTheme('dark')
    expect(document.documentElement.dataset.theme).toBeUndefined()
  })

  it('caches the applied theme', () => {
    applyTheme('light')
    expect(localStorage.getItem('loredesktop.theme')).toBe('light')
    applyTheme('dark')
    expect(localStorage.getItem('loredesktop.theme')).toBe('dark')
  })
})

describe('cachedTheme', () => {
  beforeEach(() => { localStorage.clear() })

  it('reads back the applied theme', () => {
    applyTheme('light')
    expect(cachedTheme()).toBe('light')
  })

  it('defaults to dark when nothing is cached', () => {
    expect(cachedTheme()).toBe('dark')
  })
})

describe('otherTheme', () => {
  it('flips light and dark', () => {
    expect(otherTheme('light')).toBe('dark')
    expect(otherTheme('dark')).toBe('light')
  })
})
