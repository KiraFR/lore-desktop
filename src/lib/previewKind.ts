/** Shared classifier: files whose rows/panels can show an image thumbnail. */
const IMAGE_RE = /\.(png|jpe?g|webp|bmp|gif|tga|tiff?|dds|exr|hdr|psd|blend|uasset|umap)$/i

/** Exact sidecar naming the CLI materializes for the incoming version of a
 *  conflicted file during a merge — pinned by the P2 item-2 real-merge
 *  verification (plan Task 12). Mirrored by THEIRS_SUFFIX in preview.rs. */
const THEIRS_SUFFIX = '~theirs'

export function theirsSidecar(path: string): string {
  return path + THEIRS_SUFFIX
}

export function stripTheirsSuffix(path: string): string {
  return path.endsWith(THEIRS_SUFFIX) ? path.slice(0, -THEIRS_SUFFIX.length) : path
}

export function isPreviewableImage(path: string): boolean {
  return IMAGE_RE.test(stripTheirsSuffix(path))
}
