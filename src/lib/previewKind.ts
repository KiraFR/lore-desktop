/** Shared classifier: files whose rows/panels can show an image thumbnail. */
const IMAGE_RE = /\.(png|jpe?g|webp|bmp|gif|tga|tiff?|dds|exr|hdr|psd|blend|uasset|umap)$/i

export function isPreviewableImage(path: string): boolean {
  return IMAGE_RE.test(path)
}
