import { SvelteMap } from 'svelte/reactivity'
import { api } from './api'
import { session } from './session.svelte'
import { isPreviewableImage } from './previewKind'

/** Row thumbnails: path → data URL (null = no thumbnail). Requests are queued
 *  (concurrency 4) so a 300-file changelist never floods the CLI or the disk. */
export const listThumbs = new SvelteMap<string, string | null>()

const pending: string[] = []
let running = 0

export function requestThumb(path: string) {
  if (listThumbs.has(path) || !isPreviewableImage(path)) return
  listThumbs.set(path, null)
  pending.push(path)
  pump()
}

function pump() {
  const repo = session.config.currentRepo
  if (!repo) return
  while (running < 4 && pending.length > 0) {
    const p = pending.shift()!
    running++
    api.getPreview(repo, p, 64)
      .then((r) => listThumbs.set(p, r.kind === 'image' ? r.url : null))
      .catch(() => listThumbs.set(p, null))
      .finally(() => { running--; pump() })
  }
}

/** Drop the in-memory map (disk cache stays; mtime keys handle staleness). */
export function clearThumbs() {
  listThumbs.clear()
  pending.length = 0
}
