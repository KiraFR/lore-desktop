# Asset Previews (tiers 1–2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `docs/superpowers/specs/2026-07-09-lore-desktop-previews-design.md`

**Goal:** Real image thumbnails (incl. tga/dds/exr/hdr/psd) and an audio player in FilePreview, from the working copy, with a disk cache.

**Architecture:** New Rust module `preview.rs` — one `lore_preview` command classifying by extension, decoding via `image`/`ddsfile`+`texture2ddecoder`/`psd`, thumbnailing to ≤512px PNG data-URLs with an `app_cache_dir()/thumbs` cache. Audio streams through Tauri's asset protocol (dynamic per-file scope). Front: `LoreApi.getPreview` (tauri + mock parity), FilePreview renders the After box / audio player / Dimensions row.

**Tech Stack:** Rust (image, ddsfile, texture2ddecoder, psd, base64), Tauri asset protocol, Svelte 5.

**Commands:** `npm run check` · `npm test` · `cargo test --manifest-path src-tauri/Cargo.toml`

---

### Task 1: Rust — preview pipeline

**Files:**
- Create: `src-tauri/src/preview.rs`
- Modify: `src-tauri/Cargo.toml` (deps + `protocol-asset` feature), `src-tauri/tauri.conf.json` (assetProtocol), `src-tauri/src/lib.rs` (mod + register), `src-tauri/src/commands.rs` (make `blocking` and `ext_of` `pub(crate)`; add `"gif"` to `BINARY_EXTS`)

- [ ] **Step 1: Dependencies + config**

Run: `cargo add --manifest-path src-tauri/Cargo.toml image ddsfile texture2ddecoder psd base64`
Then in `src-tauri/Cargo.toml` set `tauri = { version = "2.11.3", features = ["protocol-asset"] }`.

In `tauri.conf.json`, `app.security` becomes:

```json
    "security": {
      "csp": null,
      "assetProtocol": { "enable": true, "scope": [] }
    }
```

- [ ] **Step 2: Small edits in `commands.rs`**

- `async fn blocking` → `pub(crate) async fn blocking`
- `fn ext_of` → `pub(crate) fn ext_of`
- Add `"gif",` to `BINARY_EXTS` (after `"webp"`).

- [ ] **Step 3: Create `src-tauri/src/preview.rs`** (full code, incl. tests)

```rust
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::Manager;

use crate::commands::{blocking, ext_of};

#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PreviewDto {
    pub kind: String, // "image" | "audio" | "none"
    pub data_url: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

const AUDIO_EXTS: &[&str] = &["wav", "ogg", "mp3", "flac"];
const IMAGE_EXTS: &[&str] =
    &["png", "jpg", "jpeg", "webp", "bmp", "gif", "tga", "tif", "tiff", "dds", "exr", "hdr", "psd"];

fn none() -> PreviewDto {
    PreviewDto { kind: "none".into(), data_url: None, width: None, height: None }
}

/// Decode a DDS (BC1–BC7 or uncompressed RGBA8) to RGBA8.
fn decode_dds(path: &Path) -> Result<image::RgbaImage, String> {
    use ddsfile::{D3DFormat, DxgiFormat};
    let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let dds = ddsfile::Dds::read(&mut f).map_err(|e| e.to_string())?;
    let (w, h) = (dds.get_width() as usize, dds.get_height() as usize);
    let data = dds.get_data(0).map_err(|e| e.to_string())?;

    type Dec = fn(&[u8], usize, usize, &mut [u32]) -> Result<(), &'static str>;
    let dxgi = dds.get_dxgi_format();
    let d3d = dds.get_d3d_format();
    let dec: Option<Dec> = match (dxgi, d3d) {
        (Some(DxgiFormat::BC1_UNorm | DxgiFormat::BC1_UNorm_sRGB), _) | (_, Some(D3DFormat::DXT1)) => Some(texture2ddecoder::decode_bc1),
        (Some(DxgiFormat::BC2_UNorm | DxgiFormat::BC2_UNorm_sRGB), _) | (_, Some(D3DFormat::DXT3)) => Some(texture2ddecoder::decode_bc2),
        (Some(DxgiFormat::BC3_UNorm | DxgiFormat::BC3_UNorm_sRGB), _) | (_, Some(D3DFormat::DXT5)) => Some(texture2ddecoder::decode_bc3),
        (Some(DxgiFormat::BC4_UNorm), _) => Some(texture2ddecoder::decode_bc4),
        (Some(DxgiFormat::BC5_UNorm), _) => Some(texture2ddecoder::decode_bc5),
        (Some(DxgiFormat::BC7_UNorm | DxgiFormat::BC7_UNorm_sRGB), _) => Some(texture2ddecoder::decode_bc7),
        _ => None,
    };
    let rgba: Vec<u8> = if let Some(dec) = dec {
        let mut out = vec![0u32; w * h];
        dec(data, w, h, &mut out).map_err(|e| e.to_string())?;
        // texture2ddecoder emits BGRA-packed u32 pixels.
        let mut v = Vec::with_capacity(w * h * 4);
        for px in out {
            let [b, g, r, a] = px.to_le_bytes();
            v.extend_from_slice(&[r, g, b, a]);
        }
        v
    } else if matches!(d3d, Some(D3DFormat::A8B8G8R8)) || matches!(dxgi, Some(DxgiFormat::R8G8B8A8_UNorm | DxgiFormat::R8G8B8A8_UNorm_sRGB)) {
        data.get(..w * h * 4).ok_or("truncated DDS data")?.to_vec()
    } else {
        return Err("unsupported DDS format".into());
    };
    image::RgbaImage::from_raw(w as u32, h as u32, rgba).ok_or_else(|| "bad DDS buffer".into())
}

/// Flattened PSD preview (simple RGB/RGBA documents; others error → generic icon).
fn decode_psd(path: &Path) -> Result<image::RgbaImage, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let psd = psd::Psd::from_bytes(&bytes).map_err(|e| e.to_string())?;
    image::RgbaImage::from_raw(psd.width(), psd.height(), psd.rgba()).ok_or_else(|| "bad PSD buffer".into())
}

/// EXR/HDR: decode to f32, Reinhard tone-map + gamma so the thumbnail is viewable.
fn decode_hdr_like(path: &Path) -> Result<image::RgbaImage, String> {
    let img = image::open(path).map_err(|e| e.to_string())?;
    let f = img.to_rgba32f();
    let (w, h) = f.dimensions();
    let mut out = image::RgbaImage::new(w, h);
    let map = |x: f32| {
        let x = x.max(0.0);
        ((x / (1.0 + x)).powf(1.0 / 2.2) * 255.0).round().clamp(0.0, 255.0) as u8
    };
    for (p, q) in f.pixels().zip(out.pixels_mut()) {
        *q = image::Rgba([map(p[0]), map(p[1]), map(p[2]), (p[3].clamp(0.0, 1.0) * 255.0) as u8]);
    }
    Ok(out)
}

fn decode(path: &Path, ext: &str) -> Result<image::RgbaImage, String> {
    match ext {
        "dds" => decode_dds(path),
        "psd" => decode_psd(path),
        "exr" | "hdr" => decode_hdr_like(path),
        _ => image::open(path).map(|d| d.to_rgba8()).map_err(|e| e.to_string()),
    }
}

/// Thumbnail cache key: absolute path + mtime + size + max_px, hex-encoded.
fn cache_key(abs: &Path, max_px: u32) -> Option<String> {
    use std::hash::{Hash, Hasher};
    let meta = std::fs::metadata(abs).ok()?;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    abs.hash(&mut h);
    meta.len().hash(&mut h);
    meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos().hash(&mut h);
    max_px.hash(&mut h);
    Some(format!("{:016x}", h.finish()))
}

fn data_url(png: &[u8]) -> String {
    use base64::Engine;
    format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png))
}

/// Full image path → PreviewDto pipeline (classify, cache, decode, thumbnail).
/// Separated from the command so tests can drive it with a temp cache dir.
pub(crate) fn image_preview(abs: &Path, ext: &str, max_px: u32, cache_dir: Option<PathBuf>) -> PreviewDto {
    if !IMAGE_EXTS.contains(&ext) || !abs.is_file() {
        return none();
    }
    let key = cache_key(abs, max_px);
    if let (Some(dir), Some(key)) = (cache_dir.as_ref(), key.as_ref()) {
        let png_p = dir.join(format!("{key}.png"));
        let dims_p = dir.join(format!("{key}.json"));
        if let (Ok(png), Ok(dims)) = (std::fs::read(&png_p), std::fs::read_to_string(&dims_p)) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&dims) {
                return PreviewDto {
                    kind: "image".into(),
                    data_url: Some(data_url(&png)),
                    width: v.get("w").and_then(|x| x.as_u64()).map(|x| x as u32),
                    height: v.get("h").and_then(|x| x.as_u64()).map(|x| x as u32),
                };
            }
        }
    }
    let rgba = match decode(abs, ext) {
        Ok(r) => r,
        Err(_) => return none(), // undecodable → generic icon, never a toast
    };
    let (w, h) = rgba.dimensions();
    let dyn_img = image::DynamicImage::ImageRgba8(rgba);
    let thumb = if w > max_px || h > max_px { dyn_img.thumbnail(max_px, max_px) } else { dyn_img };
    let mut buf = std::io::Cursor::new(Vec::new());
    if thumb.write_to(&mut buf, image::ImageFormat::Png).is_err() {
        return none();
    }
    let png = buf.into_inner();
    if let (Some(dir), Some(key)) = (cache_dir.as_ref(), key.as_ref()) {
        if std::fs::create_dir_all(dir).is_ok() {
            let _ = std::fs::write(dir.join(format!("{key}.png")), &png);
            let _ = std::fs::write(dir.join(format!("{key}.json")), format!("{{\"w\":{w},\"h\":{h}}}"));
        }
    }
    PreviewDto { kind: "image".into(), data_url: Some(data_url(&png)), width: Some(w), height: Some(h) }
}

/// Working-copy preview of `<repo>/<path>`: image thumbnail (cached) or audio
/// (asset-protocol scope opened so the front can stream it) or none.
#[tauri::command]
pub async fn lore_preview(
    app: tauri::AppHandle,
    repo_path: String,
    path: String,
    max_px: u32,
) -> Result<PreviewDto, String> {
    blocking(move || {
        let ext = ext_of(&path);
        let abs = Path::new(&repo_path).join(&path);
        if AUDIO_EXTS.contains(&ext.as_str()) {
            if !abs.is_file() {
                return Ok(none());
            }
            let _ = app.asset_protocol_scope().allow_file(&abs);
            return Ok(PreviewDto { kind: "audio".into(), data_url: None, width: None, height: None });
        }
        let cache = app.path().app_cache_dir().ok().map(|d| d.join("thumbs"));
        Ok(image_preview(&abs, &ext, max_px, cache))
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("lore-desktop-prevtest-{name}"));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn write_png(dir: &Path, name: &str, w: u32, h: u32) -> PathBuf {
        let p = dir.join(name);
        image::RgbaImage::from_pixel(w, h, image::Rgba([200, 60, 60, 255])).save(&p).unwrap();
        p
    }

    #[test]
    fn png_thumbnail_with_source_dims() {
        let d = dir("png");
        let p = write_png(&d, "big.png", 1024, 512);
        let out = image_preview(&p, "png", 256, None);
        assert_eq!(out.kind, "image");
        assert_eq!((out.width, out.height), (Some(1024), Some(512)));
        assert!(out.data_url.unwrap().starts_with("data:image/png;base64,"));
    }

    #[test]
    fn tga_decodes_too() {
        let d = dir("tga");
        let p = d.join("t.tga");
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(32, 32, image::Rgba([1, 2, 3, 255])))
            .save_with_format(&p, image::ImageFormat::Tga)
            .unwrap();
        let out = image_preview(&p, "tga", 256, None);
        assert_eq!(out.kind, "image");
    }

    #[test]
    fn unknown_or_missing_is_none() {
        let d = dir("none");
        assert_eq!(image_preview(&d.join("x.customfmt"), "customfmt", 256, None).kind, "none");
        assert_eq!(image_preview(&d.join("gone.png"), "png", 256, None).kind, "none");
    }

    #[test]
    fn second_call_hits_the_cache() {
        let d = dir("cache");
        let p = write_png(&d, "c.png", 300, 300);
        let cache = d.join("thumbs");
        let a = image_preview(&p, "png", 128, Some(cache.clone()));
        assert_eq!(std::fs::read_dir(&cache).unwrap().count(), 2); // png + json
        let b = image_preview(&p, "png", 128, Some(cache));
        assert_eq!(a, b);
    }
}
```

- [ ] **Step 4: Register**

`lib.rs`: add `mod preview;` line? No — modules live in `lib.rs` as `mod commands; mod config; mod lore;` → add `mod preview;` and register `preview::lore_preview,` in `generate_handler!`.

- [ ] **Step 5: Run + commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml` — PASS (fix any dds/psd crate API drift the compiler flags).

```bash
git add src-tauri
git commit -m "feat(preview): thumbnail pipeline (tga/dds/exr/hdr/psd), disk cache, audio via asset protocol"
```

---

### Task 2: TS — API + mock/tauri parity

**Files:**
- Modify: `src/lib/types.ts`, `src/lib/tauri.ts`, `src/lib/mock.ts`, `src/lib/mock.test.ts`

- [ ] **Step 1: Types**

`types.ts`:

```ts
export interface PreviewData {
  kind: 'image' | 'audio' | 'none'
  /** image: PNG data URL of the thumbnail; audio: streamable URL; none: null. */
  url: string | null
  /** Source dimensions (image only). */
  width?: number
  height?: number
}
```

`LoreApi`: add `/** Working-copy visual/audio preview of a repo file. */ getPreview(repoPath: string, path: string): Promise<PreviewData>`.

- [ ] **Step 2: tauri.ts**

Import `convertFileSrc` from `@tauri-apps/api/core` and `PreviewData` type; add:

```ts
  getPreview: async (repoPath, path) => {
    const r = await invoke<{ kind: string; dataUrl: string | null; width?: number | null; height?: number | null }>(
      'lore_preview', { repoPath, path, maxPx: 512 })
    if (r.kind === 'audio') return { kind: 'audio', url: convertFileSrc(`${repoPath}/${path}`) }
    if (r.kind === 'image' && r.dataUrl)
      return { kind: 'image', url: r.dataUrl, width: r.width ?? undefined, height: r.height ?? undefined }
    return { kind: 'none', url: null }
  },
```

- [ ] **Step 3: mock.ts** — runtime-generated fakes

```ts
/** Minimal valid WAV (0.2 s of silence) so the mock audio player renders and plays. */
export function silentWavDataUrl(): string {
  const sampleRate = 8000, samples = 1600
  const buf = new ArrayBuffer(44 + samples * 2)
  const v = new DataView(buf)
  const w4 = (o: number, s: string) => { for (let i = 0; i < 4; i++) v.setUint8(o + i, s.charCodeAt(i)) }
  w4(0, 'RIFF'); v.setUint32(4, 36 + samples * 2, true); w4(8, 'WAVE')
  w4(12, 'fmt '); v.setUint32(16, 16, true); v.setUint16(20, 1, true); v.setUint16(22, 1, true)
  v.setUint32(24, sampleRate, true); v.setUint32(28, sampleRate * 2, true); v.setUint16(32, 2, true); v.setUint16(34, 16, true)
  w4(36, 'data'); v.setUint32(40, samples * 2, true)
  let bin = ''
  new Uint8Array(buf).forEach((b) => (bin += String.fromCharCode(b)))
  return `data:audio/wav;base64,${btoa(bin)}`
}

const PREVIEW_IMAGE_RE = /\.(png|jpe?g|webp|bmp|gif|tga|tiff?|dds|exr|hdr|psd)$/i
const PREVIEW_AUDIO_RE = /\.(wav|ogg|mp3|flac)$/i
```

and the method:

```ts
  async getPreview(_repoPath: string, path: string) {
    await delay(200)
    if (PREVIEW_AUDIO_RE.test(path)) return { kind: 'audio', url: silentWavDataUrl() } as PreviewData
    if (PREVIEW_IMAGE_RE.test(path)) {
      const name = path.split('/').pop() ?? path
      const svg =
        `<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512">` +
        `<defs><pattern id="c" width="32" height="32" patternUnits="userSpaceOnUse">` +
        `<rect width="32" height="32" fill="#2b2f35"/><rect width="16" height="16" fill="#3a4048"/>` +
        `<rect x="16" y="16" width="16" height="16" fill="#3a4048"/></pattern></defs>` +
        `<rect width="512" height="512" fill="url(#c)"/>` +
        `<text x="256" y="264" font-family="sans-serif" font-size="26" fill="#9fb0c0" text-anchor="middle">${name}</text></svg>`
      return { kind: 'image', url: `data:image/svg+xml,${encodeURIComponent(svg)}`, width: 2048, height: 2048 } as PreviewData
    }
    return { kind: 'none', url: null } as PreviewData
  },
```

- [ ] **Step 4: mock tests**

Append to `mock.test.ts`:

```ts
  it('getPreview classifies image, audio, and none', async () => {
    const img = await mock.getPreview('C:/repos/game', 'Content/T_Rock.dds')
    expect(img.kind).toBe('image')
    expect(img.url).toMatch(/^data:image\/svg\+xml,/)
    const au = await mock.getPreview('C:/repos/game', 'Audio/hit.wav')
    expect(au.kind).toBe('audio')
    expect(au.url).toMatch(/^data:audio\/wav;base64,/)
    const no = await mock.getPreview('C:/repos/game', 'Source/main.cpp')
    expect(no.kind).toBe('none')
  })
```

- [ ] **Step 5: Verify + commit**

Run: `npm run check && npm test` — PASS.

```bash
git add src/lib/types.ts src/lib/tauri.ts src/lib/mock.ts src/lib/mock.test.ts
git commit -m "feat(api): getPreview with mock SVG/WAV parity"
```

---

### Task 3: FilePreview — render the previews

**Files:**
- Modify: `src/lib/FilePreview.svelte`

- [ ] **Step 1: Fetch state** — after the diff effect, same anti-race pattern:

```ts
  let preview = $state<PreviewData | null>(null)
  let lastPreviewPath = ''

  $effect(() => {
    const f = file
    const repoPath = session.config.currentRepo
    if (!f || !f.isBinary || f.action === 'delete' || !repoPath) { preview = null; lastPreviewPath = ''; return }
    const same = f.path === lastPreviewPath
    lastPreviewPath = f.path
    if (!same) preview = null
    api.getPreview(repoPath, f.path)
      .then((p) => { if (file?.path === f.path) preview = p })
      .catch(() => { if (file?.path === f.path) preview = null })
  })
```

(import `PreviewData` type.)

- [ ] **Step 2: Markup** — replace the binary branch content:

```svelte
      {#if file.isBinary}
        {#if preview?.kind === 'audio' && preview.url}
          <div class="audio"><audio controls src={preview.url}></audio></div>
          <p class="note muted"><Icon name="info" size={14} /> Audio asset — plays the working copy.</p>
        {:else}
          <div class="cmp">
            {#if file.action !== 'add'}
              <figure class="cbox">
                <div class="thumb before"><Icon name="image" size={26} /></div>
                <figcaption>Before · previous revision</figcaption>
              </figure>
            {/if}
            {#if file.action !== 'delete'}
              <figure class="cbox">
                {#if preview?.kind === 'image' && preview.url}
                  <div class="thumb after img"><img src={preview.url} alt={baseName(file.path)} /></div>
                {:else}
                  <div class="thumb after"><Icon name="image" size={26} /></div>
                {/if}
                <figcaption class="aft">{file.action === 'add' ? 'New file' : 'After · working copy'}</figcaption>
              </figure>
            {/if}
          </div>
          {#if preview?.kind === 'image'}
            <p class="note muted"><Icon name="info" size={14} /> Previous-revision preview needs server support — working copy only.</p>
          {:else}
            <p class="note muted"><Icon name="info" size={14} /> Binary asset — visual compare, no text diff.</p>
          {/if}
        {/if}
```

- [ ] **Step 3: Meta row** — in the `<dl class="meta">`, after Size:

```svelte
        {#if preview?.width && preview?.height}
          <div><dt>Dimensions</dt><dd>{preview.width} × {preview.height}</dd></div>
        {/if}
```

- [ ] **Step 4: CSS additions**

```css
  .thumb.img { padding: 0; overflow: hidden; background: repeating-conic-gradient(#2b2f35 0% 25%, #333a44 0% 50%) 50% / 24px 24px; }
  .thumb.img img { width: 100%; height: 100%; object-fit: contain; display: block; }
  .audio { padding: 10px 0 2px; }
  .audio audio { width: 100%; }
```

- [ ] **Step 5: Verify + commit**

Run: `npm run check && npm test` — PASS.

```bash
git add src/lib/FilePreview.svelte
git commit -m "feat(preview): working-copy image thumbnails, audio player, dimensions row"
```

---

### Task 4: End-to-end verification

- [ ] **Step 1: Suites** — `npm run check && npm test && cargo test --manifest-path src-tauri/Cargo.toml` all PASS.
- [ ] **Step 2: Mock (browser)** — un fichier image des changes mock montre la vignette SVG damier + « Dimensions 2048 × 2048 » + la note server-support ; un `.wav` ajouté au mock seed montre le lecteur audio.
- [ ] **Step 3: Real app** — générer un PNG + un WAV dans `lore-test-repo`, sélectionner dans Changes : vraie vignette (After) + dimensions ; lecteur audio fonctionnel ; re-sélection instantanée (cache) ; fichier `.cpp` → comportement inchangé.
- [ ] **Step 4: Commit fixes** if any.

---

## Self-review

- **Spec coverage :** pipeline+cache+formats (Task 1), asset protocol + scope dynamique (Task 1), API front + parité mock (Task 2), FilePreview image/audio/dimensions/notes (Task 3), gif → BINARY_EXTS (Task 1), delete sans preview (Task 3 guard). ✔
- **Placeholders :** code complet partout. ✔
- **Type consistency :** `PreviewDto { kind, dataUrl, width, height }` (camelCase serde) ↔ invoke shape in tauri.ts ; `PreviewData` identique types/mock/tauri/FilePreview ; `image_preview(abs, ext, max_px, cache_dir)` signature partagée code/tests. ✔
