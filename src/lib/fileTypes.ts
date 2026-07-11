/** Human-readable asset type names, shared by the Changes and History previews. */
const TYPES: Record<string, string> = {
  uasset: 'Unreal asset', umap: 'Level (map)', pak: 'Unreal package',
  cpp: 'C++ source', h: 'C++ header', cs: 'C# source', ini: 'Config', md: 'Markdown', json: 'JSON',
  png: 'Texture', tga: 'Texture', dds: 'Texture', tif: 'Texture', tiff: 'Texture', jpg: 'Texture', jpeg: 'Texture', webp: 'Texture',
  exr: 'HDR texture', hdr: 'HDR texture', psd: 'Photoshop document',
  fbx: 'Mesh', obj: 'Mesh', abc: 'Alembic cache', gltf: 'Mesh', glb: 'Mesh',
  blend: 'Blender scene', ma: 'Maya scene', mb: 'Maya scene', max: '3ds Max scene', ztl: 'ZBrush tool',
  sbs: 'Substance graph', sbsar: 'Substance archive', spp: 'Substance Painter project',
  wav: 'Audio', ogg: 'Audio', mp3: 'Audio', flac: 'Audio', bank: 'Audio bank',
  anim: 'Animation',
}

export const ext = (p: string): string => {
  const i = p.lastIndexOf('.')
  return i < 0 ? '' : p.slice(i + 1).toLowerCase()
}

export function typeName(p: string): string {
  return TYPES[ext(p)] ?? (ext(p) ? ext(p).toUpperCase() + ' file' : 'File')
}

const TEXT_DIFF_EXT = new Set([
  'txt','text','md','markdown','json','xml','yaml','yml','toml','ini','cfg','conf','csv','tsv','log','sql',
  'c','cc','cxx','cpp','h','hh','hpp','hxx','cs','rs','go','java','kt','rb','php','lua','py','js','jsx','ts','tsx','sh','bat','ps1',
  'glsl','hlsl','usf','ush','shader','html','css','scss','svg','uproject','uplugin','gitignore','gitattributes','editorconfig',
])

/** Files whose historical (revision-range) text diff is worth showing. */
export function isTextDiffable(path: string): boolean {
  return TEXT_DIFF_EXT.has(ext(path))
}
