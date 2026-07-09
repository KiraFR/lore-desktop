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
