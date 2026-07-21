import { describe, it, expect } from 'vitest'
import { isPreviewableImage, theirsSidecar, stripTheirsSuffix } from './previewKind'

describe('isPreviewableImage', () => {
  it('accepts common game image formats, case-insensitively', () => {
    expect(isPreviewableImage('Content/T_Rock.dds')).toBe(true)
    expect(isPreviewableImage('Content/UI/icon.PNG')).toBe(true)
    expect(isPreviewableImage('renders/beauty.exr')).toBe(true)
    expect(isPreviewableImage('Content/Chars/hero.blend')).toBe(true)
    expect(isPreviewableImage('Content/Meshes/SM_Crate.uasset')).toBe(true)
    expect(isPreviewableImage('Content/Maps/Arena.UMAP')).toBe(true)
  })
  it('rejects everything else', () => {
    expect(isPreviewableImage('Source/main.cpp')).toBe(false)
    expect(isPreviewableImage('Audio/hit.wav')).toBe(false)
    expect(isPreviewableImage('Content/SM_Crate.obj')).toBe(false)
    // .spp preview was dropped — its thumbnail lives in compressed HDF5 datasets,
    // not raw image bytes (verified on a real file). Shows a generic icon now.
    expect(isPreviewableImage('Painter/HeroSkin.spp')).toBe(false)
    // .sbsar preview was dropped — the 7z icon extraction was never validated
    // on a real file. Shows a generic icon now.
    expect(isPreviewableImage('Materials/Rock_Wall.sbsar')).toBe(false)
  })
})

describe('theirs sidecar helpers', () => {
  it('builds and strips the sidecar path', () => {
    expect(theirsSidecar('Content/T_Rock.png')).toBe('Content/T_Rock.png~theirs')
    expect(stripTheirsSuffix('Content/T_Rock.png~theirs')).toBe('Content/T_Rock.png')
    expect(stripTheirsSuffix('Content/T_Rock.png')).toBe('Content/T_Rock.png')
  })
  it('classifies a sidecar like its base file', () => {
    expect(isPreviewableImage('Content/T_Rock.png~theirs')).toBe(true)
    expect(isPreviewableImage('Source/main.cpp~theirs')).toBe(false)
  })
})
