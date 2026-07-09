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
const IMAGE_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "bmp", "gif", "tga", "tif", "tiff", "dds", "exr", "hdr", "psd", "blend",
    "uasset", "umap",
];

const MODEL_EXTS: &[&str] = &["glb", "gltf", "obj", "fbx"];

pub(crate) fn is_model_ext(ext: &str) -> bool {
    MODEL_EXTS.contains(&ext)
}

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
        (Some(DxgiFormat::BC1_UNorm | DxgiFormat::BC1_UNorm_sRGB), _) | (_, Some(D3DFormat::DXT1)) => {
            Some(texture2ddecoder::decode_bc1)
        }
        (Some(DxgiFormat::BC2_UNorm | DxgiFormat::BC2_UNorm_sRGB), _) | (_, Some(D3DFormat::DXT3)) => {
            Some(texture2ddecoder::decode_bc2)
        }
        (Some(DxgiFormat::BC3_UNorm | DxgiFormat::BC3_UNorm_sRGB), _) | (_, Some(D3DFormat::DXT5)) => {
            Some(texture2ddecoder::decode_bc3)
        }
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
    } else if matches!(d3d, Some(D3DFormat::A8B8G8R8))
        || matches!(dxgi, Some(DxgiFormat::R8G8B8A8_UNorm | DxgiFormat::R8G8B8A8_UNorm_sRGB))
    {
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
        "blend" => decode_blend(path),
        "uasset" | "umap" => decode_uasset(path),
        _ => image::open(path).map(|d| d.to_rgba8()).map_err(|e| e.to_string()),
    }
}

/// The thumbnail Blender embeds at save time. The file may be zstd- (≥3.0) or
/// gzip-compressed; no scene decoding, just the block stream.
fn decode_blend(path: &Path) -> Result<image::RgbaImage, String> {
    let raw = std::fs::read(path).map_err(|e| e.to_string())?;
    let data: Vec<u8> = if raw.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
        zstd::decode_all(&raw[..]).map_err(|e| e.to_string())?
    } else if raw.starts_with(&[0x1F, 0x8B]) {
        use std::io::Read;
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(&raw[..]).read_to_end(&mut out).map_err(|e| e.to_string())?;
        out
    } else {
        raw
    };
    blend_thumbnail(&data).ok_or_else(|| "no embedded thumbnail".into())
}

/// Walk the .blend block stream and pull the `TEST` thumbnail:
/// `i32 width, i32 height`, then w×h×4 RGBA stored bottom-up.
fn blend_thumbnail(data: &[u8]) -> Option<image::RgbaImage> {
    if data.len() < 12 || &data[..7] != b"BLENDER" {
        return None;
    }
    let ptr = match data[7] {
        b'_' => 4usize,
        b'-' => 8,
        _ => return None,
    };
    let little = data[8] == b'v';
    let rd32 = |b: &[u8]| -> u32 {
        let a = [b[0], b[1], b[2], b[3]];
        if little { u32::from_le_bytes(a) } else { u32::from_be_bytes(a) }
    };
    let mut o = 12usize;
    while o + 4 + 4 + ptr + 8 <= data.len() {
        let code = &data[o..o + 4];
        let size = rd32(&data[o + 4..o + 8]) as usize;
        let body = o + 4 + 4 + ptr + 4 + 4;
        if code == b"ENDB" {
            break;
        }
        if code == b"TEST" && size >= 8 && body + size <= data.len() {
            let d = &data[body..body + size];
            let w = rd32(&d[0..4]) as usize;
            let h = rd32(&d[4..8]) as usize;
            if w == 0 || h == 0 || 8 + w * h * 4 > size {
                return None;
            }
            let px = &d[8..8 + w * h * 4];
            // Stored bottom-up (OpenGL convention) — flip to top-down.
            let mut rgba = Vec::with_capacity(px.len());
            for row in (0..h).rev() {
                rgba.extend_from_slice(&px[row * w * 4..(row + 1) * w * 4]);
            }
            return image::RgbaImage::from_raw(w as u32, h as u32, rgba);
        }
        o = body.checked_add(size)?;
    }
    None
}

/// Bounds-checked little-endian cursor over a byte slice (no panics).
struct Cur<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Cur<'a> {
    fn new(b: &'a [u8]) -> Self {
        Cur { b, p: 0 }
    }
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let s = self.b.get(self.p..self.p.checked_add(n)?)?;
        self.p += n;
        Some(s)
    }
    fn i32(&mut self) -> Option<i32> {
        self.take(4).map(|s| i32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn u32(&mut self) -> Option<u32> {
        self.i32().map(|v| v as u32)
    }
    fn skip(&mut self, n: usize) -> Option<()> {
        self.take(n).map(|_| ())
    }
}

/// Unreal FString: `i32 len`; len ≥ 0 ⇒ `len` UTF-8 bytes (trailing NUL
/// included), len < 0 ⇒ `-len` UTF-16LE code units. `max_units` bounds
/// hostile lengths so a corrupt file never triggers a huge allocation.
fn read_fstring(r: &mut Cur, max_units: i32) -> Option<String> {
    let len = r.i32()?;
    if len == 0 {
        return Some(String::new());
    }
    if len > 0 {
        if len > max_units {
            return None;
        }
        let bytes = r.take(len as usize)?;
        let s = bytes.strip_suffix(&[0]).unwrap_or(bytes);
        Some(String::from_utf8_lossy(s).into_owned())
    } else {
        let n = len.checked_neg().filter(|n| *n <= max_units)?;
        let bytes = r.take((n as usize) * 2)?;
        let mut units: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        if units.last() == Some(&0) {
            units.pop();
        }
        Some(String::from_utf16_lossy(&units))
    }
}

/// Seeked read of up to `len` bytes at `off` (short read at EOF is fine).
fn read_at(f: &mut std::fs::File, off: u64, len: usize) -> Option<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    f.seek(SeekFrom::Start(off)).ok()?;
    let mut v = Vec::new();
    f.by_ref().take(len as u64).read_to_end(&mut v).ok()?;
    Some(v)
}

/// Decode bytes only if they start with a PNG or JPEG magic and stay
/// thumbnail-sized (UE thumbnails are ≤ 256 px; the 1024 cap rejects false
/// magic hits inside source data). Trailing garbage after the image is fine.
fn decode_sniffed(data: &[u8]) -> Option<image::RgbaImage> {
    if !(data.starts_with(b"\x89PNG\r\n\x1a\n") || data.starts_with(&[0xFF, 0xD8, 0xFF])) {
        return None;
    }
    let img = image::load_from_memory(data).ok()?.to_rgba8();
    if img.width().max(img.height()) > 1024 {
        return None;
    }
    Some(img)
}

/// The thumbnail the Unreal editor serializes into a package at save time.
/// Seeked reads only — editor textures can weigh hundreds of MB.
fn decode_uasset(path: &Path) -> Result<image::RgbaImage, String> {
    let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let len = f.metadata().map_err(|e| e.to_string())?.len();
    uasset_summary_thumbnail(&mut f, len)
        .or_else(|| uasset_magic_scan(&mut f, len))
        .ok_or_else(|| "no embedded thumbnail".into())
}

/// Parse the FPackageFileSummary far enough to locate ThumbnailTableOffset.
/// The prefix (tag → NameOffset) is stable across UE 4.11+ and read
/// structurally; the tail is version-gated (fields appear/disappear per
/// release, some variable-sized), so instead of hardcoding every gate the
/// offset is found by probing each byte position in a small window and
/// validating the table behind it — only the real one survives validation.
fn uasset_summary_thumbnail(f: &mut std::fs::File, file_len: u64) -> Option<image::RgbaImage> {
    let head = read_at(f, 0, 64 * 1024)?;
    let mut r = Cur::new(&head);
    if r.u32()? != 0x9E2A_83C1 {
        return None; // not a package (or big-endian) — the magic scan may still hit
    }
    let legacy = r.i32()?;
    if !(-12..=-6).contains(&legacy) {
        return None; // pre-4.11 custom-version container or unknown future layout
    }
    r.skip(4)?; // LegacyUE3Version
    r.skip(4)?; // FileVersionUE4 (0 for unversioned ⇒ latest layout, same path)
    if legacy <= -8 {
        r.skip(4)?; // FileVersionUE5
    }
    r.skip(4)?; // FileVersionLicenseeUE4
    if legacy <= -9 {
        r.skip(20)?; // SavedHash (FIoHash) — UE 5.5+
        r.skip(4)?; // TotalHeaderSize (moved before the custom versions here)
    }
    let custom = r.i32()?;
    if !(0..=4096).contains(&custom) {
        return None;
    }
    r.skip(custom as usize * 20)?; // FGuid + i32 per custom version
    if legacy > -9 {
        r.skip(4)?; // TotalHeaderSize (UE ≤ 5.4 position)
    }
    read_fstring(&mut r, 4096)?; // FolderName (≤ 5.4) / PackageName (5.5+)
    r.skip(4)?; // PackageFlags
    r.skip(8)?; // NameCount + NameOffset
    let start = r.p;
    let end = (start + 512).min(head.len().saturating_sub(4));
    for p in start..end {
        let v = i32::from_le_bytes([head[p], head[p + 1], head[p + 2], head[p + 3]]);
        if v > 0 && (v as u64) < file_len {
            if let Some(img) = thumbnail_table(f, v as u64, file_len) {
                return Some(img);
            }
        }
    }
    None
}

/// Validate and read a thumbnail table at `off`: `i32 count`, then per entry
/// ClassName (FString), ObjectPathWithoutPackageName (FString), `i32
/// FileOffset`. Returns the first entry whose FObjectThumbnail decodes.
fn thumbnail_table(f: &mut std::fs::File, off: u64, file_len: u64) -> Option<image::RgbaImage> {
    let probe = read_at(f, off, 4)?;
    let count = Cur::new(&probe).i32()?;
    if !(1..=64).contains(&count) {
        return None;
    }
    let buf = read_at(f, off + 4, 64 * 1024)?;
    let mut r = Cur::new(&buf);
    let mut offsets = Vec::new();
    for _ in 0..count {
        let class = read_fstring(&mut r, 1024)?;
        if !class.chars().all(|c| c.is_ascii_graphic()) {
            return None; // class names are plain ASCII — reject noise early
        }
        read_fstring(&mut r, 4096)?; // object path
        let data_off = r.i32()?;
        if data_off <= 0 || (data_off as u64) >= file_len {
            return None;
        }
        offsets.push(data_off as u64);
    }
    offsets.into_iter().find_map(|o| object_thumbnail(f, o, file_len))
}

/// FObjectThumbnail: `i32 width`, `i32 height` (a negative value encodes a
/// compression variant — ignore the sign), `i32 size`, then PNG/JPEG bytes.
fn object_thumbnail(f: &mut std::fs::File, off: u64, file_len: u64) -> Option<image::RgbaImage> {
    let head = read_at(f, off, 12)?;
    let mut r = Cur::new(&head);
    let w = r.i32()?.unsigned_abs();
    let h = r.i32()?.unsigned_abs();
    let size = r.i32()?;
    if w == 0 || h == 0 || w > 4096 || h > 4096 {
        return None;
    }
    if size <= 0 || size as u64 > 16 << 20 || off + 12 + size as u64 > file_len {
        return None;
    }
    decode_sniffed(&read_at(f, off + 12, size as usize)?)
}

/// Last-resort, version-agnostic fallback: bounded chunked scan (64 MiB read
/// budget, chunks overlapping enough that a whole thumbnail payload is always
/// seen in one piece) for a PNG/JPEG magic that decodes thumbnail-sized.
fn uasset_magic_scan(f: &mut std::fs::File, file_len: u64) -> Option<image::RgbaImage> {
    const CHUNK: usize = 16 << 20;
    const OVERLAP: usize = 4 << 20;
    let mut pos = 0u64;
    let mut budget: u64 = 64 << 20;
    while pos < file_len && budget > 0 {
        let want = CHUNK.min((file_len - pos) as usize).min(budget as usize);
        let buf = read_at(f, pos, want)?;
        if buf.is_empty() {
            return None;
        }
        budget -= buf.len() as u64;
        let last = pos + buf.len() as u64 >= file_len;
        // The tail overlap of a non-final chunk is re-scanned (with full
        // context) at the start of the next one — skip it here.
        let scan_end = if last { buf.len() } else { buf.len().saturating_sub(OVERLAP) };
        for i in 0..scan_end {
            let b = buf[i];
            if b != 0x89 && b != 0xFF {
                continue;
            }
            if let Some(img) = decode_sniffed(&buf[i..]) {
                return Some(img);
            }
        }
        if last || buf.len() <= OVERLAP {
            break;
        }
        pos += (buf.len() - OVERLAP) as u64;
    }
    None
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

    /// Minimal synthetic .blend: header + one TEST block (2×2 RGBA) + ENDB.
    fn synthetic_blend() -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(b"BLENDER-v300"); // 8-byte pointers, little-endian
        let w: u32 = 2;
        let h: u32 = 2;
        // Rows bottom-up: bottom row RED, top row BLUE.
        let bottom = [255u8, 0, 0, 255, 255, 0, 0, 255];
        let top = [0u8, 0, 255, 255, 0, 0, 255, 255];
        let mut body = Vec::new();
        body.extend_from_slice(&w.to_le_bytes());
        body.extend_from_slice(&h.to_le_bytes());
        body.extend_from_slice(&bottom);
        body.extend_from_slice(&top);
        b.extend_from_slice(b"TEST");
        b.extend_from_slice(&(body.len() as u32).to_le_bytes());
        b.extend_from_slice(&[0u8; 8]); // old pointer
        b.extend_from_slice(&0u32.to_le_bytes()); // SDNA index
        b.extend_from_slice(&1u32.to_le_bytes()); // count
        b.extend_from_slice(&body);
        b.extend_from_slice(b"ENDB");
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&[0u8; 8]);
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b
    }

    #[test]
    fn blend_thumbnail_parses_and_flips() {
        let img = blend_thumbnail(&synthetic_blend()).unwrap();
        assert_eq!(img.dimensions(), (2, 2));
        // Top-left pixel must be the (stored-last) BLUE top row after the flip.
        assert_eq!(img.get_pixel(0, 0).0, [0, 0, 255, 255]);
        assert_eq!(img.get_pixel(0, 1).0, [255, 0, 0, 255]);
    }

    #[test]
    fn blend_zstd_roundtrip_decodes() {
        let d = dir("blendz");
        let compressed = zstd::encode_all(&synthetic_blend()[..], 1).unwrap();
        let p = d.join("scene.blend");
        std::fs::write(&p, compressed).unwrap();
        let out = image_preview(&p, "blend", 128, None);
        assert_eq!(out.kind, "image");
        assert_eq!((out.width, out.height), (Some(2), Some(2)));
    }

    #[test]
    fn blend_without_thumbnail_is_none() {
        let d = dir("blendn");
        let p = d.join("empty.blend");
        std::fs::write(&p, b"BLENDER-v300ENDB").unwrap();
        assert_eq!(image_preview(&p, "blend", 128, None).kind, "none");
        let bad = d.join("bad.blend");
        std::fs::write(&bad, b"NOTABLEND").unwrap();
        assert_eq!(image_preview(&bad, "blend", 128, None).kind, "none");
    }

    fn mini_png(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(w, h, image::Rgba([10, 200, 30, 255]));
        let mut buf = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img).write_to(&mut buf, image::ImageFormat::Png).unwrap();
        buf.into_inner()
    }

    /// UTF-8 FString bytes: i32 len (NUL included) + bytes + NUL.
    fn fstr(s: &str) -> Vec<u8> {
        let mut v = ((s.len() + 1) as i32).to_le_bytes().to_vec();
        v.extend_from_slice(s.as_bytes());
        v.push(0);
        v
    }

    /// Deterministic garbage that can never alias a PNG/JPEG magic
    /// (consecutive bytes always differ by 7).
    fn noise(n: usize) -> Vec<u8> {
        (0..n).map(|i| (i * 7 + 13) as u8).collect()
    }

    /// Minimal synthetic UE-style package: summary prefix, one
    /// FObjectThumbnail (real PNG payload), then the thumbnail table.
    /// `legacy` -8 = UE 5.0–5.4 layout, -9 = UE 5.5+ (SavedHash, moved
    /// TotalHeaderSize).
    fn synthetic_uasset(legacy: i32) -> Vec<u8> {
        let png = mini_png(4, 4);
        let thumb_off = 256u64;
        let table_off = thumb_off + 12 + png.len() as u64;
        let mut b = Vec::new();
        b.extend_from_slice(&0x9E2A_83C1u32.to_le_bytes()); // package tag
        b.extend_from_slice(&legacy.to_le_bytes()); // LegacyFileVersion
        b.extend_from_slice(&864i32.to_le_bytes()); // LegacyUE3Version
        b.extend_from_slice(&522i32.to_le_bytes()); // FileVersionUE4
        b.extend_from_slice(&1009i32.to_le_bytes()); // FileVersionUE5
        b.extend_from_slice(&0i32.to_le_bytes()); // FileVersionLicenseeUE4
        if legacy <= -9 {
            b.extend_from_slice(&[0xABu8; 20]); // SavedHash
            b.extend_from_slice(&(thumb_off as i32).to_le_bytes()); // TotalHeaderSize
        }
        b.extend_from_slice(&1i32.to_le_bytes()); // custom version count
        b.extend_from_slice(&[0u8; 20]); // FGuid + i32 entry
        if legacy > -9 {
            b.extend_from_slice(&(thumb_off as i32).to_le_bytes()); // TotalHeaderSize
        }
        b.extend_from_slice(&fstr("None")); // FolderName / PackageName
        b.extend_from_slice(&0u32.to_le_bytes()); // PackageFlags
        b.extend_from_slice(&8i32.to_le_bytes()); // NameCount
        b.extend_from_slice(&64i32.to_le_bytes()); // NameOffset
        b.extend_from_slice(&[0u8; 24]); // assorted version-gated counts/offsets
        b.extend_from_slice(&(table_off as i32).to_le_bytes()); // ThumbnailTableOffset
        b.resize(thumb_off as usize, 0);
        // FObjectThumbnail: 4×4, negative height = compression-variant marker.
        b.extend_from_slice(&4i32.to_le_bytes());
        b.extend_from_slice(&(-4i32).to_le_bytes());
        b.extend_from_slice(&(png.len() as i32).to_le_bytes());
        b.extend_from_slice(&png);
        b.extend_from_slice(&1i32.to_le_bytes()); // table: 1 entry
        b.extend_from_slice(&fstr("Texture2D"));
        b.extend_from_slice(&fstr("T_Test"));
        b.extend_from_slice(&(thumb_off as i32).to_le_bytes());
        b
    }

    #[test]
    fn uasset_thumbnail_via_summary_table() {
        for legacy in [-8i32, -9] {
            let d = dir(&format!("uasset{}", -legacy));
            let p = d.join("T_Test.uasset");
            std::fs::write(&p, synthetic_uasset(legacy)).unwrap();
            let out = image_preview(&p, "uasset", 128, None);
            assert_eq!(out.kind, "image", "legacy {legacy}");
            assert_eq!((out.width, out.height), (Some(4), Some(4)));
            // Specifically through the structured path, not the scan fallback.
            let mut f = std::fs::File::open(&p).unwrap();
            let len = f.metadata().unwrap().len();
            assert!(uasset_summary_thumbnail(&mut f, len).is_some(), "legacy {legacy}");
        }
    }

    #[test]
    fn uasset_fallback_scan_finds_embedded_png() {
        let d = dir("uassetscan");
        let png = mini_png(8, 2);
        let mut b = noise(3000);
        b.extend_from_slice(&png);
        b.extend_from_slice(&noise(3000));
        let p = d.join("Level.umap");
        std::fs::write(&p, b).unwrap();
        let out = image_preview(&p, "umap", 128, None);
        assert_eq!(out.kind, "image");
        assert_eq!((out.width, out.height), (Some(8), Some(2)));
    }

    #[test]
    fn uasset_without_thumbnail_is_none() {
        let d = dir("uassetnone");
        let p = d.join("Cooked.uasset");
        std::fs::write(&p, noise(4096)).unwrap();
        assert_eq!(image_preview(&p, "uasset", 128, None).kind, "none");
    }

    #[test]
    fn read_fstring_handles_both_encodings() {
        // UTF-8: positive len, trailing NUL included.
        let mut b = 3i32.to_le_bytes().to_vec();
        b.extend_from_slice(b"Hi\0");
        let mut r = Cur::new(&b);
        assert_eq!(read_fstring(&mut r, 64).as_deref(), Some("Hi"));
        assert_eq!(r.p, b.len()); // fully consumed, next field aligned
        // UTF-16LE: negative len counts code units, NUL included.
        let mut b = (-3i32).to_le_bytes().to_vec();
        for u in [0x48u16, 0xE9, 0] {
            b.extend_from_slice(&u.to_le_bytes());
        }
        let mut r = Cur::new(&b);
        assert_eq!(read_fstring(&mut r, 64).as_deref(), Some("Hé"));
        assert_eq!(r.p, b.len());
        // Hostile length is rejected, never allocated.
        let big = i32::MAX.to_le_bytes();
        assert!(read_fstring(&mut Cur::new(&big), 64).is_none());
    }

    #[test]
    fn model_extensions_classify() {
        assert!(is_model_ext("fbx"));
        assert!(is_model_ext("glb"));
        assert!(!is_model_ext("wav"));
        assert!(!is_model_ext("png"));
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
