# Previews Tier 3 Implementation Plan — list thumbnails + 3D viewer

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `docs/superpowers/specs/2026-07-09-lore-desktop-previews-tier3-design.md`

**Goal:** 20 px thumbnails on image rows in Changes / commit detail / Locks, and a three.js turntable viewer for glb/gltf/obj/fbx in FilePreview.

**Architecture:** Rust classifies model extensions as `kind "model"` (asset-protocol scope, no decode). Front gains `getPreview(..., maxPx?)`, a shared `previewKind.ts` classifier, a `thumbs.svelte.ts` request queue (SvelteMap, concurrency 4, cleared on status refresh), and `ModelViewer.svelte` (three/addons loaders, auto-fit, auto-rotate-until-interaction, full disposal).

**Tech Stack:** three ^0.1xx (+ @types/three, installed), Svelte 5, Rust.

---

### Task 1: Rust — `kind: "model"`

**Files:** Modify `src-tauri/src/preview.rs`

- [ ] **Step 1:** Add after `IMAGE_EXTS`:

```rust
const MODEL_EXTS: &[&str] = &["glb", "gltf", "obj", "fbx"];

pub(crate) fn is_model_ext(ext: &str) -> bool {
    MODEL_EXTS.contains(&ext)
}
```

In `lore_preview`, after the audio branch:

```rust
        if is_model_ext(&ext) {
            if !abs.is_file() {
                return Ok(none());
            }
            // The webview streams the model — and its sidecar .bin/textures for
            // glTF/FBX — through the asset protocol.
            let _ = app.asset_protocol_scope().allow_file(&abs);
            if let Some(dir) = abs.parent() {
                let _ = app.asset_protocol_scope().allow_directory(dir, false);
            }
            return Ok(PreviewDto { kind: "model".into(), data_url: None, width: None, height: None });
        }
```

Test in the `tests` module:

```rust
    #[test]
    fn model_extensions_classify() {
        assert!(is_model_ext("fbx"));
        assert!(is_model_ext("glb"));
        assert!(!is_model_ext("wav"));
        assert!(!is_model_ext("png"));
    }
```

- [ ] **Step 2:** `cargo test --manifest-path src-tauri/Cargo.toml` PASS → commit `feat(preview): model kind streams via asset protocol`.

---

### Task 2: TS — API, classifier, thumbs store, mock

**Files:** Create `src/lib/previewKind.ts`, `src/lib/previewKind.test.ts`, `src/lib/thumbs.svelte.ts` ; Modify `src/lib/types.ts`, `src/lib/tauri.ts`, `src/lib/mock.ts`, `src/lib/repo.svelte.ts`

- [ ] **Step 1:** `previewKind.ts` (+ tests: png/dds true, cpp/wav false):

```ts
/** Shared classifier: files whose rows/panels can show an image thumbnail. */
const IMAGE_RE = /\.(png|jpe?g|webp|bmp|gif|tga|tiff?|dds|exr|hdr|psd)$/i
export function isPreviewableImage(path: string): boolean {
  return IMAGE_RE.test(path)
}
```

- [ ] **Step 2:** types.ts — `PreviewData.kind` union gains `'model'`; `getPreview(repoPath: string, path: string, maxPx?: number)`.

- [ ] **Step 3:** tauri.ts —

```ts
  getPreview: async (repoPath, path, maxPx): Promise<PreviewData> => {
    const r = await invoke<{ kind: string; dataUrl: string | null; width?: number | null; height?: number | null }>(
      'lore_preview', { repoPath, path, maxPx: maxPx ?? 512 })
    if (r.kind === 'audio') return { kind: 'audio', url: convertFileSrc(`${repoPath}/${path}`) }
    if (r.kind === 'model') return { kind: 'model', url: convertFileSrc(`${repoPath}/${path}`) }
    if (r.kind === 'image' && r.dataUrl)
      return { kind: 'image', url: r.dataUrl, width: r.width ?? undefined, height: r.height ?? undefined }
    return { kind: 'none', url: null }
  },
```

- [ ] **Step 4:** mock.ts — replace the local image regex by `isPreviewableImage`; add:

```ts
const PREVIEW_MODEL_RE = /\.(glb|gltf|obj|fbx)$/i
const CUBE_OBJ = 'v -1 -1 -1\nv 1 -1 -1\nv 1 1 -1\nv -1 1 -1\nv -1 -1 1\nv 1 -1 1\nv 1 1 1\nv -1 1 1\nf 1 2 3 4\nf 5 8 7 6\nf 1 5 6 2\nf 2 6 7 3\nf 3 7 8 4\nf 5 1 4 8\n'
```

model branch in `getPreview` (before the image branch):

```ts
    if (PREVIEW_MODEL_RE.test(path)) {
      const url = /\.obj$/i.test(path) ? `data:text/plain,${encodeURIComponent(CUBE_OBJ)}` : null
      return { kind: 'model', url } as PreviewData
    }
```

Seed: add `{ path: 'Content/Props/SM_Crate.obj', action: 'add', isBinary: true, size: 20480 },` after the wav seed.

- [ ] **Step 5:** `thumbs.svelte.ts`:

```ts
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
```

- [ ] **Step 6:** repo.svelte.ts — `refreshStatus` calls `clearThumbs()` right after `repo.status = await api.getStatus(path)` (import from `./thumbs.svelte`).

- [ ] **Step 7:** `npm run check && npm test` PASS → commit `feat(previews): model kind, shared classifier, row-thumbnail store`.

---

### Task 3: UI — thumbnails in the three lists

**Files:** Modify `src/lib/Changes.svelte`, `src/lib/History.svelte`, `src/lib/Locks.svelte`

- [ ] **Step 1:** Each view imports `{ listThumbs, requestThumb }`, requests in an effect (`action !== 'delete'` guarded where applicable), and renders before the path:

```svelte
{#if listThumbs.get(f.path)}<img class="rowthumb" src={listThumbs.get(f.path)} alt="" />{/if}
```

CSS (each view): `.rowthumb { width: 20px; height: 20px; border-radius: 4px; object-fit: cover; flex: none; }`

- Changes: effect over `files`; thumb between the action tag and the path.
- History detail: effect over `detailFiles`; thumb in the `.fl li` rows.
- Locks: effect over `locks.list`; thumb replaces the `iconFor` icon when present.

- [ ] **Step 2:** `npm run check` PASS → commit `feat(previews): row thumbnails in Changes, commit detail, and Locks`.

---

### Task 4: `ModelViewer.svelte` + FilePreview

**Files:** Create `src/lib/ModelViewer.svelte`; Modify `src/lib/FilePreview.svelte`, `package.json` (three — already installed)

- [ ] **Step 1:** ModelViewer per the spec: three/addons `OrbitControls` + `GLTFLoader`/`OBJLoader`/`FBXLoader` picked by `name` extension, hemisphere+key light, subtle grid, auto-fit (center + camera at 1.8 × maxDim, near/far adapted, grid scaled/floored), `autoRotate` until the first `start` event, `ResizeObserver`, `setAnimationLoop`, full disposal (geometries, materials, renderer, DOM). `failed` state → dashed fallback box. Missing normals → `computeVertexNormals()`.

- [ ] **Step 2:** FilePreview — insert between the audio and image branches:

```svelte
        {:else if preview?.kind === 'model' && preview.url}
          <ModelViewer url={preview.url} name={baseName(file.path)} />
          <p class="note muted"><Icon name="info" size={14} /> 3D preview of the working copy — drag to orbit, scroll to zoom.</p>
```

- [ ] **Step 3:** `npm run check && npm test` PASS → commit `feat(previews): three.js turntable viewer for glb/gltf/obj/fbx`.

---

### Task 5: Verification

- [ ] Suites TS + Rust PASS.
- [ ] Mock : vignettes 20 px sur les lignes image de Changes (png), détail de commit et Locks ; `SM_Crate.obj` → cube en rotation, orbit au drag.
- [ ] App réelle : vignette de ligne sur `T_Gradient.png` ; cube `.obj` généré dans `lore-test-repo` → turntable.
- [ ] Commit fixes.

## Self-review

Spec A couvert (Tasks 2-3 : maxPx, store concurrence 4, clearThumbs sur refreshStatus, 3 listes, guard delete) ; spec B couvert (Tasks 1, 2, 4 : kind model + scope dir, convertFileSrc, viewer complet, mock cube OBJ + seed, fallback) ; hors périmètre respecté (pas de spine/abc). Signatures cohérentes : `getPreview(repoPath, path, maxPx?)` partout ; `isPreviewableImage` partagé store/mock ; `ModelViewer {url, name}` = appel FilePreview.
