import { describe, it, expect } from 'vitest'
import { isPreviewableImage } from './previewKind'

describe('isPreviewableImage', () => {
  it('accepts common game image formats, case-insensitively', () => {
    expect(isPreviewableImage('Content/T_Rock.dds')).toBe(true)
    expect(isPreviewableImage('Content/UI/icon.PNG')).toBe(true)
    expect(isPreviewableImage('renders/beauty.exr')).toBe(true)
    expect(isPreviewableImage('Content/Chars/hero.blend')).toBe(true)
    expect(isPreviewableImage('Content/Meshes/SM_Crate.uasset')).toBe(true)
    expect(isPreviewableImage('Content/Maps/Arena.UMAP')).toBe(true)
    expect(isPreviewableImage('Materials/Rock_Wall.sbsar')).toBe(true)
    expect(isPreviewableImage('Painter/HeroSkin.spp')).toBe(true)
  })
  it('rejects everything else', () => {
    expect(isPreviewableImage('Source/main.cpp')).toBe(false)
    expect(isPreviewableImage('Audio/hit.wav')).toBe(false)
    expect(isPreviewableImage('Content/SM_Crate.obj')).toBe(false)
  })
})
