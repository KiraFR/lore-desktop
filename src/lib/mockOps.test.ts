import { describe, it, expect } from 'vitest'
import { mock } from './mock'

describe('mock.fileSizes', () => {
  it('returns old sizes for known modified files only', async () => {
    const sizes = await mock.fileSizes('C:/repos/game', [
      'Content/Maps/Level_01.umap',
      'Content/Characters/Hero/SK_Hero.uasset', // add — no old size seeded
    ])
    expect(sizes['Content/Maps/Level_01.umap']).toBe(2100480)
    expect(sizes['Content/Characters/Hero/SK_Hero.uasset']).toBeUndefined()
  })
})
