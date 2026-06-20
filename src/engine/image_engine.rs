use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use image::RgbImage;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::Frame;

// ─────────────────────────────────────────────────────────────────────────────
// TrueColor half-block pixel-art engine.
//
// Terminal cells are ~1 wide × 2 tall. By printing the lower-half-block glyph `▄`
// and colouring its foreground (lower half) and background (upper half) from two
// stacked image pixels, one cell renders two square pixels — so a portrait keeps a
// 1:1 aspect ratio on any shell, Linux or Windows Terminal, with no sixel/kitty
// protocol and no disturbance to surrounding text cells.
//
// Decoding happens once per asset and is cached in memory (decode failures are
// cached too, so a missing file is never re-probed every frame). Sampling is
// nearest-neighbour straight from the cached RGB buffer, so per-frame cost is just
// W×H colour writes — no allocation, no frame-rate cliff.
// ─────────────────────────────────────────────────────────────────────────────

/// A cached decode result. `Failed` is stored deliberately so a broken/absent asset
/// is probed exactly once, then falls back to text forever without thrashing the disk.
enum CacheEntry {
    Ok(Arc<RgbImage>),
    Failed,
}

fn cache() -> &'static Mutex<HashMap<PathBuf, CacheEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, CacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Decode `path` once and hand back a shared handle, or `None` if it could not be
/// read/decoded. Both outcomes are memoised. A poisoned lock degrades to "no image"
/// rather than propagating a panic into the render loop.
fn load_cached(path: &Path) -> Option<Arc<RgbImage>> {
    let mut map = match cache().lock() {
        Ok(m) => m,
        Err(_) => return None,
    };
    if let Some(entry) = map.get(path) {
        return match entry {
            CacheEntry::Ok(img) => Some(img.clone()),
            CacheEntry::Failed => None,
        };
    }
    // First touch — decode, high-quality downsample, lift contrast, then memoise.
    let decoded = decode_and_process(path).map(Arc::new);
    match decoded {
        Some(img) => {
            map.insert(path.to_path_buf(), CacheEntry::Ok(img.clone()));
            Some(img)
        }
        None => {
            map.insert(path.to_path_buf(), CacheEntry::Failed);
            None
        }
    }
}

/// The cache-warming pass downsamples every asset to fit this box (aspect preserved)
/// with a high-quality Lanczos3 filter — far crisper on fine facial edges than the old
/// nearest-neighbour shrink — then runtime cover-sampling reads from this clean buffer.
const CACHE_MAX_DIM: u32 = 512;

/// The single, expensive decode pass run once per asset and memoised:
///   1. decode PNG/JPEG,
///   2. Lanczos3 downsample to ≤512² to preserve crisp edge structure,
///   3. lift contrast so dark leather/wood tones don't swallow portrait detail.
/// Any failure short-circuits to `None`, which the cache stores as `Failed`.
fn decode_and_process(path: &Path) -> Option<RgbImage> {
    let dynamic = image::open(path).ok()?;
    // High-fidelity downsample (aspect preserved, fits within CACHE_MAX_DIM²).
    let scaled = dynamic.resize(CACHE_MAX_DIM, CACHE_MAX_DIM, image::imageops::FilterType::Lanczos3);
    let rgb = scaled.to_rgb8();
    // Implicit contrast lift on the decoded buffer before it is committed to cache.
    Some(image::imageops::contrast(&rgb, 15.0))
}

/// Resolve the `images/` asset directory: next to the executable (shipping layout),
/// falling back to the crate source tree (dev). Shared by every image consumer.
pub fn images_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("images");
            if candidate.is_dir() {
                return candidate;
            }
        }
    }
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/images"))
}

/// Warm the cache for an asset ahead of time (call when a cinematic state activates),
/// so the first frame of the scene never pays the decode cost mid-draw.
pub fn preload(path: &Path) {
    let _ = load_cached(path);
}

/// Cheap, deterministic per-cell hash — drives the analog-noise jitter without any RNG
/// state or allocation, so the same `(x, y, frame)` always glitches identically.
fn hash(x: u32, y: u32, f: u64) -> u32 {
    let mut h = x
        .wrapping_mul(374_761_393)
        .wrapping_add(y.wrapping_mul(668_265_263))
        .wrapping_add(f as u32);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^ (h >> 16)
}

/// Nudge an RGB channel by a signed jitter, saturating at the 0..=255 boundary.
fn jitter(c: u8, delta: i32) -> u8 {
    (c as i32 + delta).clamp(0, 255) as u8
}

/// The faint static glyphs sprinkled along an unstable transmission line ("Cızırtı").
const STATIC_TOKENS: [char; 4] = ['§', '░', '▒', '#'];

/// Render `image_path` into `area` as TrueColor half-block pixel art, with an analog
/// noise overlay scaled by `glitch_intensity` (0.0 = pristine, 1.0 = heavy interference)
/// and animated by `frame_count`. A missing or corrupt asset falls back to a clean,
/// centred typewriter placeholder instead of panicking.
pub fn draw_pixel_art(
    f: &mut Frame,
    area: Rect,
    image_path: &Path,
    glitch_intensity: f32,
    frame_count: u64,
) {
    render_image(f, area, image_path, glitch_intensity, 1.0, frame_count, true);
}

/// Project `image_path` as a dimmed full-area backdrop (RGB × `brightness`), with no
/// glitch and — crucially — no "SIGNAL LOST" card on failure: returns `false` instead,
/// so a caller (the menu) can fall back to its own wash. Returns `true` once drawn.
pub fn draw_backdrop(
    f: &mut Frame,
    area: Rect,
    image_path: &Path,
    brightness: f32,
    frame_count: u64,
) -> bool {
    if area.width == 0 || area.height == 0 || load_cached(image_path).is_none() {
        return false;
    }
    render_image(f, area, image_path, 0.0, brightness, frame_count, false);
    true
}

/// Core half-block projector. `brightness` scales every channel (1.0 = true colour,
/// 0.25 = the dimmed menu backdrop). `fallback` selects whether a missing asset draws
/// the placeholder card (portraits) or is silently skipped (backdrops, handled by the
/// caller). The **widescreen cover-crop fill** rule applies: the image is scaled to fill
/// the full width, with vertical excess cropped top-heavy so the subject's face is never
/// truncated — never letterboxed with side bars.
fn render_image(
    f: &mut Frame,
    area: Rect,
    image_path: &Path,
    glitch_intensity: f32,
    brightness: f32,
    frame_count: u64,
    fallback: bool,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let img = match load_cached(image_path) {
        Some(i) => i,
        None => {
            if fallback {
                draw_fallback(f, area, image_path);
            }
            return;
        }
    };

    let buf = f.buffer_mut();
    let cols = area.width;
    let rows = area.height;
    let (sw, sh) = (img.width() as f32, img.height() as f32);
    if sw < 1.0 || sh < 1.0 {
        return;
    }

    // ── Widescreen cover-crop ──────────────────────────────────────────────────
    // Cover the whole W×(2H) pixel grid (no side bars): scale by the LARGER ratio so
    // both axes are filled, then crop the overflow. Horizontal overflow is centred;
    // vertical overflow is cropped top-heavy (20% off the top, 80% off the bottom) so
    // a portrait's face — usually upper-centre — is preserved.
    let gw = cols as f32;
    let gh = (rows as f32) * 2.0;
    let scale = (gw / sw).max(gh / sh);
    let dw = sw * scale;
    let dh = sh * scale;
    let ox = (gw - dw) * 0.5; // centre horizontally (≤ 0)
    let oy = -(dh - gh) * 0.20; // top-heavy vertical crop (≤ 0)
    let bright = brightness.clamp(0.0, 1.0);
    let dim = |c: u8| (c as f32 * bright) as u8;

    let g = glitch_intensity.clamp(0.0, 1.0);
    // A single horizontal scanline crawls down the frame; on it, interference is hottest.
    let scan = if rows > 0 { ((frame_count / 2) % (rows as u64 * 2)) as i64 } else { -1 };

    // Sample the covered source at grid pixel (gx, gy), clamping to edges so the cover
    // crop never reads out of bounds, then apply the brightness scale.
    let sample = |gx: f32, gy: f32| -> [u8; 3] {
        let ix = ((gx - ox) / scale).clamp(0.0, sw - 1.0);
        let iy = ((gy - oy) / scale).clamp(0.0, sh - 1.0);
        let p = img.get_pixel(ix as u32, iy as u32);
        [dim(p.0[0]), dim(p.0[1]), dim(p.0[2])]
    };

    for ry in 0..rows {
        for rx in 0..cols {
            let gx = rx as f32 + 0.5;
            let py_top = (ry as u32 * 2) as f32 + 0.5;
            let py_bot = (ry as u32 * 2 + 1) as f32 + 0.5;
            let mut top = sample(gx, py_top); // upper pixel → cell background
            let mut bot = sample(gx, py_bot); // lower pixel → cell foreground (the ▄)

            // ── Analog interference ───────────────────────────────────────────
            let mut ch = '\u{2584}'; // ▄ lower half block
            if g > 0.0 {
                let h = hash(rx as u32, ry as u32, frame_count);
                let on_scan = (ry as i64 * 2) == scan || (ry as i64 * 2 + 1) == scan;
                let local = if on_scan { g * 2.2 } else { g };

                // RGB channel jitter — a faint chroma shimmer over the whole grid.
                let amp = (local * 26.0) as i32;
                if amp > 0 {
                    let d1 = (h % (amp as u32 * 2 + 1)) as i32 - amp;
                    let d2 = ((h >> 8) % (amp as u32 * 2 + 1)) as i32 - amp;
                    top = [jitter(top[0], d1), jitter(top[1], d2), jitter(top[2], d1)];
                    bot = [jitter(bot[0], d2), jitter(bot[1], d1), jitter(bot[2], d2)];
                }

                // Sparse static tokens — isolated cells overwritten with grid noise.
                let thresh = (local * 55.0) as u32;
                if (h >> 16) % 1000 < thresh {
                    ch = STATIC_TOKENS[(h % STATIC_TOKENS.len() as u32) as usize];
                    let v = 120 + (h % 90) as u8;
                    bot = [v, (v as u16 * 7 / 10) as u8, (v as u16 * 4 / 10) as u8];
                }
            }

            if rx + area.x >= buf.area().width || ry + area.y >= buf.area().height {
                continue;
            }
            let cell = buf.get_mut(area.x + rx, area.y + ry);
            cell.set_char(ch);
            cell.fg = Color::Rgb(bot[0], bot[1], bot[2]);
            cell.bg = Color::Rgb(top[0], top[1], top[2]);
        }
    }
}

/// Clean, panic-free placeholder when an asset is missing or corrupt: a dim framed
/// card with a typewriter status line built from the asset's file stem.
fn draw_fallback(f: &mut Frame, area: Rect, image_path: &Path) {
    let buf = f.buffer_mut();
    let bg = Color::Rgb(8, 6, 4);
    let frame_col = Color::Rgb(70, 56, 36);
    let ink = Color::Rgb(150, 120, 78);

    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let cell = buf.get_mut(x, y);
            cell.set_char(' ');
            cell.bg = bg;
            cell.fg = bg;
        }
    }
    // Thin border so the empty frame still reads as a "screen".
    let right = area.x + area.width.saturating_sub(1);
    let bottom = area.y + area.height.saturating_sub(1);
    for x in area.x..area.x + area.width {
        put(buf, x, area.y, '─', frame_col, bg);
        put(buf, x, bottom, '─', frame_col, bg);
    }
    for y in area.y..area.y + area.height {
        put(buf, area.x, y, '│', frame_col, bg);
        put(buf, right, y, '│', frame_col, bg);
    }

    let stem = image_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("transmission");
    let lines = ["⌁  SIGNAL LOST  ⌁", "[ portrait unavailable ]"];
    let mid = area.y + area.height / 2;
    for (i, s) in lines.iter().enumerate() {
        let y = mid + i as u16;
        let x = area.x + area.width.saturating_sub(s.chars().count() as u16) / 2;
        put_str(buf, x, y, s, ink);
    }
    let tag = format!("· {} ·", stem);
    let ty = mid + 2;
    let tx = area.x + area.width.saturating_sub(tag.chars().count() as u16) / 2;
    put_str(buf, tx, ty, &tag, frame_col);
}

fn put(buf: &mut Buffer, x: u16, y: u16, ch: char, fg: Color, bg: Color) {
    let Rect { x: ax, y: ay, width: aw, height: ah } = *buf.area();
    if x >= ax && x < ax + aw && y >= ay && y < ay + ah {
        let cell = buf.get_mut(x, y);
        cell.set_char(ch);
        cell.fg = fg;
        cell.bg = bg;
    }
}

fn put_str(buf: &mut Buffer, mut x: u16, y: u16, s: &str, fg: Color) {
    let bg = Color::Rgb(8, 6, 4);
    for ch in s.chars() {
        put(buf, x, y, ch, fg, bg);
        x = x.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn missing_asset_falls_back_without_panic() {
        let mut term = Terminal::new(TestBackend::new(60, 30)).unwrap();
        let path = Path::new("/nonexistent/portrait.png");
        term.draw(|f| {
            let a = f.size();
            draw_pixel_art(f, a, path, 0.2, 7);
        })
        .unwrap();
        // The failure is now memoised — a second draw must also be panic-free.
        term.draw(|f| {
            let a = f.size();
            draw_pixel_art(f, a, path, 0.0, 8);
        })
        .unwrap();
    }

    #[test]
    fn renders_real_asset_at_many_sizes_with_glitch() {
        // Best-effort: only exercises pixel paths if the dev asset is present, but must
        // never panic regardless (degenerate sizes, heavy glitch).
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/images/jacquardAct1.png");
        let path = Path::new(p);
        for (w, h, g) in [(80u16, 40u16, 0.0f32), (40, 20, 0.5), (8, 4, 1.0), (1, 1, 0.9)] {
            let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
            term.draw(|f| {
                let a = f.size();
                draw_pixel_art(f, a, path, g, 13);
            })
            .unwrap();
        }
    }
}
