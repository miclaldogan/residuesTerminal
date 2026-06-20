use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Color,
    Frame,
};

use super::state::{Act, GlobalStateContext};

// ─────────────────────────────────────────────────────────────────────────────
// 24-bit candlelight gradient anchors (the light-degradation engine)
//
//   flame  #FFF0D0  brilliant white-amber  (the source)
//     ↓    #D4AF37  rich amber gold        (warm body of the light)
//     ↓    #4A0E17  deep wax crimson       (the dying edge of the radius)
//     ↓    #121212  pitch darkness         (the desk baseline / void)
// ─────────────────────────────────────────────────────────────────────────────
const FLAME_WHITE: Color = Color::Rgb(255, 240, 208);
const AMBER_GOLD:  Color = Color::Rgb(212, 175, 55);
const WAX_CRIMSON: Color = Color::Rgb(74, 14, 23);
const PITCH:       Color = Color::Rgb(18, 18, 18);

// Charcoal silhouette tones for the cyanide apple
const SHADOW_DARK: Color = Color::Rgb(42, 42, 42);
const SHADOW_MID:  Color = Color::Rgb(64, 64, 64);
const SHADOW_RUBBLE: Color = Color::Rgb(34, 34, 34);

// The haunting, desaturated hue of a solved act's ghost
const GHOST_HUE: Color = Color::Rgb(26, 26, 26);

// Drop-shadow ink for the overlapping paper stack
const SHEET_SHADOW: Color = Color::Rgb(6, 5, 4);

// Parchment substrates
const SHEET_ACTIVE_BG: Color = Color::Rgb(48, 41, 31);
const SHEET_UNDER_BG:  Color = Color::Rgb(30, 26, 20);
const SHEET_BORDER:    Color = Color::Rgb(120, 105, 80);
const PAPER_FG:        Color = Color::Rgb(222, 202, 162);

// Pills
const PILL_COLOR: Color = Color::Rgb(150, 142, 130);

// ─────────────────────────────────────────────────────────────────────────────
// Sub-cell Braille portraits — parametric face profiles
//
// The old large-block dither arrays (█ ▓ ▒ ░) are gone. Each historical figure is
// now described by a small parameter set that an analytic intensity field turns
// into engraved contours; that field is then Bayer-dithered into the 2x4 dot
// matrix of Braille glyphs, multiplying the effective pixel resolution by 8x
// inside the same character rectangle.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct FaceParams {
    head_rx: f32,   // head half-width  (normalised)
    head_ry: f32,   // head half-height (normalised)
    hairline: f32,  // ny where the hair meets the forehead
    hair_fill: f32, // hair density 0..1
    side_hair: f32, // ny depth the hair runs down the sides
    beard: f32,     // lower-face beard coverage 0..1
    mustache: bool,
    glasses: bool,
}

const JACQUARD_FACE: FaceParams = FaceParams {
    head_rx: 0.33, head_ry: 0.42, hairline: 0.30, hair_fill: 0.85,
    side_hair: 0.58, beard: 0.0, mustache: false, glasses: false,
};
const BABBAGE_FACE: FaceParams = FaceParams {
    head_rx: 0.34, head_ry: 0.42, hairline: 0.25, hair_fill: 0.50,
    side_hair: 0.72, beard: 0.78, mustache: true, glasses: false,
};
const LOVELACE_FACE: FaceParams = FaceParams {
    head_rx: 0.31, head_ry: 0.43, hairline: 0.31, hair_fill: 0.95,
    side_hair: 0.96, beard: 0.0, mustache: false, glasses: false,
};
const BOOLE_FACE: FaceParams = FaceParams {
    head_rx: 0.34, head_ry: 0.42, hairline: 0.23, hair_fill: 0.58,
    side_hair: 0.60, beard: 0.55, mustache: true, glasses: false,
};
const SHANNON_FACE: FaceParams = FaceParams {
    head_rx: 0.33, head_ry: 0.42, hairline: 0.33, hair_fill: 0.80,
    side_hair: 0.50, beard: 0.0, mustache: false, glasses: true,
};
/// Turing himself — the player's own ghost, should the final act ever seat him.
const TURING_FACE: FaceParams = FaceParams {
    head_rx: 0.32, head_ry: 0.42, hairline: 0.34, hair_fill: 0.85,
    side_hair: 0.46, beard: 0.0, mustache: false, glasses: false,
};

// ─────────────────────────────────────────────────────────────────────────────
// Paper metadata per act
// ─────────────────────────────────────────────────────────────────────────────

struct PaperMeta {
    act: Act,
    title: &'static str,
    year: &'static str,
    subtitle: &'static str,
    signature: &'static str,
    face: FaceParams,
}

const PAPERS: &[PaperMeta] = &[
    PaperMeta {
        act: Act::Jacquard1804,
        title: "JACQUARD",
        year: "1804",
        subtitle: "Separate the instruction from the loom.",
        signature: "Hand-signed: J.M. Jacquard",
        face: JACQUARD_FACE,
    },
    PaperMeta {
        act: Act::Babbage1837,
        title: "BABBAGE",
        year: "1837",
        subtitle: "The machine only adds, but memory carries.",
        signature: "Hand-signed: Charles Babbage",
        face: BABBAGE_FACE,
    },
    PaperMeta {
        act: Act::Lovelace1843,
        title: "LOVELACE",
        year: "1843",
        subtitle: "The cards must decide and repeat.",
        signature: "Hand-signed: Augusta Ada Lovelace",
        face: LOVELACE_FACE,
    },
    PaperMeta {
        act: Act::Boole1854,
        title: "BOOLE",
        year: "1854",
        subtitle: "Two values. Three operations. The minimum.",
        signature: "Hand-signed: George Boole",
        face: BOOLE_FACE,
    },
    PaperMeta {
        act: Act::Shannon1937,
        title: "SHANNON",
        year: "1937",
        subtitle: "A switch open or shut, one or zero.",
        signature: "Hand-signed: Claude E. Shannon",
        face: SHANNON_FACE,
    },
];

fn face_for(act: Act) -> FaceParams {
    PAPERS
        .iter()
        .find(|p| p.act == act)
        .map(|p| p.face)
        .unwrap_or(TURING_FACE)
}

// ─────────────────────────────────────────────────────────────────────────────
// Sub-cell Braille rasterizer
// ─────────────────────────────────────────────────────────────────────────────

/// Ordered 4x4 Bayer matrix (values pre-normalised to [0,1]) for tonal dithering
/// of the analytic intensity field into discrete Braille dots.
const BAYER4: [[f32; 4]; 4] = [
    [0.0625, 0.5625, 0.1875, 0.6875],
    [0.8125, 0.3125, 0.9375, 0.4375],
    [0.2500, 0.7500, 0.1250, 0.6250],
    [1.0000, 0.5000, 0.8750, 0.3750],
];

/// Dot-bit weight for a sub-cell coordinate within a Braille glyph (2 wide x 4 tall).
fn braille_bit(dx: usize, dy: usize) -> u8 {
    match (dx, dy) {
        (0, 0) => 0x01, (0, 1) => 0x02, (0, 2) => 0x04, (0, 3) => 0x40,
        (1, 0) => 0x08, (1, 1) => 0x10, (1, 2) => 0x20, (1, 3) => 0x80,
        _ => 0,
    }
}

/// Soft symmetric band around `center`, smoothstep-shaped, peaking at 1.0.
fn smooth_band(v: f32, center: f32, halfwidth: f32) -> f32 {
    let d = (v - center).abs();
    if d >= halfwidth {
        0.0
    } else {
        let t = 1.0 - d / halfwidth;
        t * t * (3.0 - 2.0 * t)
    }
}

/// The analytic face engraving field. Returns ink intensity in [0,1] at normalised
/// coordinates (nx, ny) ∈ [0,1]². Built from layered contour primitives — head
/// ellipse, hairline, eyes with pupils, brows, nose, mouth, optional beard/glasses
/// and shoulders — so faces read as fine engraved line-art rather than block dither.
fn face_intensity(p: &FaceParams, nx: f32, ny: f32) -> f32 {
    let cx = 0.5;
    let cy = 0.47;
    let ex = (nx - cx) / p.head_rx;
    let ey = (ny - cy) / p.head_ry;
    let e = (ex * ex + ey * ey).sqrt();
    let inside = e < 1.0;

    let mut v: f32 = 0.0;

    // Head / jaw contour (the engraved outline).
    v = v.max(smooth_band(e, 1.0, 0.08) * 0.95);
    // Faint inner cheek/jaw shadow line.
    v = v.max(smooth_band(e, 0.82, 0.05) * 0.22);

    // Hair cap above the (wavy) hairline.
    let fringe = p.hairline + 0.02 * (nx * 17.0).sin();
    if inside && ny < fringe {
        let strand = (nx * 38.0).sin() * 0.5 + 0.5;
        v = v.max(p.hair_fill * (0.55 + 0.45 * strand));
    }
    // Long side hair framing the face.
    if inside && ny < p.side_hair && ex.abs() > 0.62 {
        v = v.max(p.hair_fill * 0.9);
    }

    // Eyes — explicit ring + pupil, optional spectacles.
    let eye_y = 0.45;
    let eye_dx = 0.17;
    for s in [-1.0f32, 1.0] {
        let ecx = cx + s * eye_dx;
        let edx = (nx - ecx) / 0.085;
        let edy = (ny - eye_y) / 0.05;
        let ed = (edx * edx + edy * edy).sqrt();
        v = v.max(smooth_band(ed, 1.0, 0.32) * 0.9); // eyelid ring
        if ed < 0.5 {
            v = v.max(1.0); // pupil
        }
        // Brow stroke above the eye.
        if (ny - (eye_y - 0.075)).abs() < 0.018 && (nx - ecx).abs() < 0.10 {
            v = v.max(0.95);
        }
        if p.glasses {
            v = v.max(smooth_band(ed, 1.4, 0.12) * 0.9); // lens rim
        }
    }
    if p.glasses && (ny - eye_y).abs() < 0.012 && (nx - cx).abs() < eye_dx {
        v = v.max(0.7); // nose bridge of the spectacles
    }

    // Nose ridge + base.
    if (nx - cx).abs() < 0.013 && ny > eye_y + 0.03 && ny < eye_y + 0.16 {
        v = v.max(0.55);
    }
    if (ny - (eye_y + 0.16)).abs() < 0.02 && (nx - cx).abs() < 0.05 {
        v = v.max(0.6);
    }

    // Mouth.
    if (ny - 0.66).abs() < 0.012 && (nx - cx).abs() < 0.11 {
        v = v.max(0.8);
    }
    // Mustache.
    if p.mustache && (ny - 0.63).abs() < 0.022 && (nx - cx).abs() < 0.13 {
        v = v.max(0.85);
    }
    // Beard — denser toward the chin, hatched for texture.
    if p.beard > 0.0 && inside && ny > 0.58 {
        let amt = p.beard * ((ny - 0.58) / 0.40).clamp(0.0, 1.0);
        let strand = ((nx * 34.0).sin() * 0.5 + 0.5) * ((ny * 28.0).sin() * 0.5 + 0.5);
        v = v.max(amt * (0.5 + 0.5 * strand));
    }

    // Collar / shoulders below the head.
    if ny > 0.9 {
        let shoulder = smooth_band((ny - 0.9) - (nx - cx).abs() * 0.4, 0.04, 0.05);
        v = v.max(shoulder * 0.6);
        if ((nx - cx).abs() - 0.12).abs() < 0.02 && ny > 0.86 {
            v = v.max(0.7);
        }
    }

    v.clamp(0.0, 1.0)
}

/// Rasterize a face into a grid of Braille cells `cw` wide × `chh` tall. Each cell
/// returns its glyph and the average sub-cell intensity (for depth shading). The
/// sub-cell dot grid is `cw*2 × chh*4`, i.e. 8x the cell resolution.
fn render_face(p: &FaceParams, cw: usize, chh: usize) -> Vec<Vec<(char, f32)>> {
    let dot_w = cw * 2;
    let dot_h = chh * 4;
    if dot_w == 0 || dot_h == 0 {
        return Vec::new();
    }

    // Analytic intensity field, sampled once per sub-cell dot.
    let mut field = vec![0.0f32; dot_w * dot_h];
    for y in 0..dot_h {
        let ny = (y as f32 + 0.5) / dot_h as f32;
        for x in 0..dot_w {
            let nx = (x as f32 + 0.5) / dot_w as f32;
            field[y * dot_w + x] = face_intensity(p, nx, ny);
        }
    }

    // Pack into Braille glyphs with ordered dithering.
    let mut out = Vec::with_capacity(chh);
    for cyc in 0..chh {
        let mut row = Vec::with_capacity(cw);
        for cxc in 0..cw {
            let mut mask = 0u8;
            let mut sum = 0.0f32;
            for dy in 0..4 {
                for dx in 0..2 {
                    let gx = cxc * 2 + dx;
                    let gy = cyc * 4 + dy;
                    let it = field[gy * dot_w + gx];
                    sum += it;
                    let thr = 0.05 + BAYER4[gy & 3][gx & 3] * 0.9;
                    if it > thr {
                        mask |= braille_bit(dx, dy);
                    }
                }
            }
            let ch = if mask == 0 {
                ' '
            } else {
                char::from_u32(0x2800 + mask as u32).unwrap_or(' ')
            };
            row.push((ch, sum / 8.0));
        }
        out.push(row);
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Simple LCG pseudo-random generator (no external deps)
// ─────────────────────────────────────────────────────────────────────────────

struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed.wrapping_add(1) }
    }

    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.state
    }

    fn next_bounded(&mut self, bound: u64) -> u64 {
        if bound == 0 { return 0; }
        self.next() % bound
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Color math
// ─────────────────────────────────────────────────────────────────────────────

/// Interpolate between two RGB colors by factor t in [0.0, 1.0].
fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    if let (Color::Rgb(ar, ag, ab), Color::Rgb(br, bg, bb)) = (a, b) {
        let t = t.clamp(0.0, 1.0);
        Color::Rgb(
            (ar as f32 + (br as f32 - ar as f32) * t) as u8,
            (ag as f32 + (bg as f32 - ag as f32) * t) as u8,
            (ab as f32 + (bb as f32 - ab as f32) * t) as u8,
        )
    } else {
        a
    }
}

/// Fade an RGB color toward pitch darkness by light factor `f` (1.0 = full, 0.0 = void).
/// Non-RGB colors pass through untouched (they have no luminance to multiply).
fn dim_to_pitch(c: Color, f: f32) -> Color {
    match c {
        Color::Rgb(..) => lerp_color(PITCH, c, f),
        other => other,
    }
}

/// Smooth organic flicker intensity in [0.0, 1.0] driven by the frame counter.
fn flame_pulse(frame: u64) -> f32 {
    let fast = ((frame as f64 * 0.30).sin() * 0.18) as f32;
    let slow = ((frame as f64 * 0.07).sin() * 0.10) as f32;
    let jitter = if frame % 11 == 0 { -0.12 } else { 0.0 };
    (0.78 + fast + slow + jitter).clamp(0.30, 1.0)
}

// ─────────────────────────────────────────────────────────────────────────────
// Safe buffer write helpers
// ─────────────────────────────────────────────────────────────────────────────

fn in_bounds(buf: &Buffer, x: u16, y: u16) -> bool {
    let Rect { x: ax, y: ay, width: aw, height: ah } = *buf.area();
    x >= ax && x < ax + aw && y >= ay && y < ay + ah
}

fn buf_set_fg(buf: &mut Buffer, x: u16, y: u16, ch: char, fg: Color) {
    if in_bounds(buf, x, y) {
        let cell = buf.get_mut(x, y);
        cell.set_char(ch);
        cell.fg = fg;
    }
}

fn buf_set_str_fg(buf: &mut Buffer, x: u16, y: u16, s: &str, fg: Color) {
    let mut col = x;
    for ch in s.chars() {
        buf_set_fg(buf, col, y, ch, fg);
        col = col.saturating_add(1);
    }
}

fn buf_set_bg(buf: &mut Buffer, x: u16, y: u16, bg: Color) {
    if in_bounds(buf, x, y) {
        buf.get_mut(x, y).bg = bg;
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// PUBLIC ENTRY POINT
// ═════════════════════════════════════════════════════════════════════════════

/// Render the [THE DESK & PAPERS] panel: an overlapping paper stack, a half-block
/// candle that acts as a true light source, the shadow apple, scattered pills, and
/// the ghost jury of solved acts. Everything is drawn flat, then bathed in a single
/// quadratic light-attenuation pass anchored on the candle flame.
pub fn render_desk(f: &mut Frame, area: Rect, state: &GlobalStateContext) {
    let buf = f.buffer_mut();
    if area.width < 6 || area.height < 6 { return; }

    // 1. The void
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let cell = buf.get_mut(x, y);
            cell.set_char(' ');
            cell.fg = PITCH;
            cell.bg = PITCH;
        }
    }

    // 2. The overlapping document stack with cell-shadow drop offsets
    draw_paper_stack(buf, area, state);

    // 3. The candle — returns its flame anchor so the light engine knows the source
    let (flame_x, flame_y, brightness) = draw_candle(buf, area, state);

    // 4. The shadow apple (no green, no labels)
    draw_apple(buf, area, state);

    // 5. Chemical drift — scattered stilboestrol
    draw_pills(buf, area, state);

    // 6. The light-degradation engine — a single attenuation sweep over the desk.
    //    `desk_reveal` ramps 0→1 on entry, pulsing the gradient up out of black.
    apply_lighting(buf, area, flame_x, flame_y, brightness, state.desk_reveal);

    // 7. The ghost jury — drawn AFTER lighting so the dead keep their own dim glow
    draw_ghost_jury(buf, area, state);
}

// ═════════════════════════════════════════════════════════════════════════════
// THE LIGHT-DEGRADATION ENGINE
// ═════════════════════════════════════════════════════════════════════════════

/// Quadratic radial attenuation from the flame. Each cell's foreground and
/// background are multiplied down toward pitch darkness by their distance from the
/// light, modulated by the flame's live brightness. Terminal cells are ~2:1 tall,
/// so vertical distance is weighted double to keep the halo circular.
fn apply_lighting(buf: &mut Buffer, area: Rect, fx: f32, fy: f32, brightness: f32, reveal: f32) {
    let radius = (area.width.max(area.height) as f32) * 0.95;
    if radius <= 0.0 { return; }
    let ambient = 0.15;
    let reveal = reveal.clamp(0.0, 1.0);

    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let dx = x as f32 - fx;
            let dy = (y as f32 - fy) * 2.0;
            let d = (dx * dx + dy * dy).sqrt();
            let mut a = 1.0 - d / radius;
            if a < 0.0 { a = 0.0; }
            a *= a; // quadratic falloff
            // The whole field is scaled by the reveal ramp so the desk fades up
            // from pure black rather than snapping in at full ambient.
            let light = (ambient + (1.0 - ambient) * brightness * a).clamp(0.0, 1.0) * reveal;

            let cell = buf.get_mut(x, y);
            cell.fg = dim_to_pitch(cell.fg, light);
            cell.bg = dim_to_pitch(cell.bg, light);
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// SUBSYSTEM 1: The Overlapping Document Stack (cell-shadow Z-index)
// ═════════════════════════════════════════════════════════════════════════════

/// Cast a one-cell drop shadow along the right and bottom edges of a sheet rect.
fn draw_drop_shadow(buf: &mut Buffer, rect: Rect) {
    let right = rect.x + rect.width;
    let bottom = rect.y + rect.height;
    for y in rect.y + 1..=bottom {
        buf_set_bg(buf, right, y, SHEET_SHADOW);
    }
    for x in rect.x + 1..=right {
        buf_set_bg(buf, x, bottom, SHEET_SHADOW);
    }
}

/// Fill a sheet rect with a parchment background and draw its border frame.
fn draw_sheet(buf: &mut Buffer, rect: Rect, bg: Color, border: Color) {
    if rect.width < 2 || rect.height < 2 { return; }
    for y in rect.y..rect.y + rect.height {
        for x in rect.x..rect.x + rect.width {
            if in_bounds(buf, x, y) {
                let cell = buf.get_mut(x, y);
                cell.set_char(' ');
                cell.bg = bg;
                cell.fg = border;
            }
        }
    }
    let x = rect.x;
    let y = rect.y;
    let w = rect.width;
    let h = rect.height;
    buf_set_with_bg(buf, x, y, '┌', border, bg);
    buf_set_with_bg(buf, x + w - 1, y, '┐', border, bg);
    buf_set_with_bg(buf, x, y + h - 1, '└', border, bg);
    buf_set_with_bg(buf, x + w - 1, y + h - 1, '┘', border, bg);
    for i in 1..w - 1 {
        buf_set_with_bg(buf, x + i, y, '─', border, bg);
        buf_set_with_bg(buf, x + i, y + h - 1, '─', border, bg);
    }
    for i in 1..h - 1 {
        buf_set_with_bg(buf, x, y + i, '│', border, bg);
        buf_set_with_bg(buf, x + w - 1, y + i, '│', border, bg);
    }
}

fn buf_set_with_bg(buf: &mut Buffer, x: u16, y: u16, ch: char, fg: Color, bg: Color) {
    if in_bounds(buf, x, y) {
        let cell = buf.get_mut(x, y);
        cell.set_char(ch);
        cell.fg = fg;
        cell.bg = bg;
    }
}

fn buf_set_str_on(buf: &mut Buffer, x: u16, y: u16, s: &str, fg: Color, bg: Color) {
    let mut col = x;
    for ch in s.chars() {
        buf_set_with_bg(buf, col, y, ch, fg, bg);
        col = col.saturating_add(1);
    }
}

fn draw_paper_stack(buf: &mut Buffer, area: Rect, state: &GlobalStateContext) {
    let active_idx = PAPERS.iter().position(|p| p.act == state.current_act).unwrap_or(0);

    let sheet_w = area.width.saturating_sub(4).min(40).max(12);
    let sheet_h = 14u16.min(area.height.saturating_sub(8)).max(8);
    let base_x = area.x + 2;
    let base_y = area.y + 1;

    // ── Under-sheets: two non-active dossiers peeking from beneath, each
    //    offset down-right and casting its own shadow → an organic stack. ──
    let mut under: Vec<&PaperMeta> = PAPERS
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != active_idx)
        .map(|(_, p)| p)
        .collect();
    under.truncate(2);

    // Draw farthest sheet first (largest offset), nearest last.
    for (k, paper) in under.iter().enumerate().rev() {
        let off = (k as u16 + 1) * 2;
        let rect = Rect::new(
            base_x + off,
            base_y + off / 2 + 1,
            sheet_w,
            sheet_h,
        );
        draw_drop_shadow(buf, rect);
        let bg = if state.acts_completed.contains(&paper.act) {
            SHEET_UNDER_BG
        } else {
            Color::Rgb(22, 19, 15)
        };
        draw_sheet(buf, rect, bg, Color::Rgb(70, 62, 48));
        // Faint peeking tab title
        let tab = format!(" {} ({}) ", paper.title, paper.year);
        buf_set_str_on(buf, rect.x + 2, rect.y, &tab, Color::Rgb(120, 105, 80), bg);
    }

    // ── The active dossier on top, fully detailed ──
    let active = Rect::new(base_x, base_y, sheet_w, sheet_h);
    draw_drop_shadow(buf, active);
    draw_sheet(buf, active, SHEET_ACTIVE_BG, SHEET_BORDER);
    draw_active_paper(buf, active, &PAPERS[active_idx], state);
}

fn draw_active_paper(buf: &mut Buffer, sheet: Rect, paper: &PaperMeta, state: &GlobalStateContext) {
    let bg = SHEET_ACTIVE_BG;
    let ix = sheet.x + 1;
    let solved = state.acts_completed.contains(&paper.act);

    // Title row
    let title_style = Color::Rgb(255, 214, 130);
    let title = format!(" ACT \u{2014} {} ({})", paper.title, paper.year);
    buf_set_str_on(buf, ix, sheet.y + 1, &title, title_style, bg);
    if solved {
        let seal = "[SEALED]";
        let sx = sheet.x + sheet.width.saturating_sub(seal.len() as u16 + 1);
        buf_set_str_on(buf, sx, sheet.y + 1, seal, Color::Rgb(120, 90, 70), bg);
    }

    // Separator
    let sep_w = sheet.width.saturating_sub(2) as usize;
    buf_set_with_bg(buf, sheet.x, sheet.y + 2, '├', SHEET_BORDER, bg);
    buf_set_str_on(buf, sheet.x + 1, sheet.y + 2, &"─".repeat(sep_w), SHEET_BORDER, bg);
    buf_set_with_bg(buf, sheet.x + sheet.width - 1, sheet.y + 2, '┤', SHEET_BORDER, bg);

    // Portrait — high-resolution Braille engraving, centered in the sheet body.
    let p_start = sheet.y + 3;
    let portrait_ch = (sheet.height.saturating_sub(6)).clamp(3, 8) as usize;
    let portrait_cw = (sheet.width.saturating_sub(4)).clamp(6, 14) as usize;
    let face = render_face(&paper.face, portrait_cw, portrait_ch);
    let pad = sheet.width.saturating_sub(2).saturating_sub(portrait_cw as u16) / 2;
    for (r, cells) in face.iter().enumerate() {
        let py = p_start + r as u16;
        if py >= sheet.y + sheet.height - 3 { break; }
        for (c, (ch, it)) in cells.iter().enumerate() {
            if *ch == ' ' { continue; }
            // Brighter ink where the engraving is denser → engraved depth.
            let col = lerp_color(Color::Rgb(96, 84, 62), PAPER_FG, 0.35 + 0.65 * it);
            buf_set_fg(buf, ix + pad + c as u16, py, *ch, col);
        }
    }

    // Subtitle + signature, anchored above the bottom border
    let sub_y = sheet.y + sheet.height - 3;
    let sig_y = sheet.y + sheet.height - 2;
    buf_set_str_on(buf, ix + 1, sub_y, paper.subtitle, Color::Rgb(180, 162, 124), bg);
    buf_set_str_on(buf, ix + 1, sig_y, paper.signature, Color::Rgb(150, 128, 96), bg);
}

// ═════════════════════════════════════════════════════════════════════════════
// SUBSYSTEM 2: The Candle (half-block light source + UI-less timer)
// ═════════════════════════════════════════════════════════════════════════════

/// A static sine displacement mask anchored to the absolute screen row, giving the
/// wax shaft an organic, hand-poured curve that stays spatially stable as it melts.
fn shaft_offset(row_y: u16) -> i16 {
    ((row_y as f32 * 0.55).sin() * 1.3).round() as i16
}

/// A descending wax-run channel. Driven entirely by `frame_counter` so it animates
/// without any mutable state, then hardens into a fixed deposit at its destination.
struct Drip {
    col_off: i16,  // column relative to the candle's left edge `cx`
    phase: u64,    // frame-counter offset so runs are out of sync
    dest_rows: u16, // how many rows below the rim the run hardens
    speed: u64,    // frames advanced per row (slow ooze)
}

const DRIPS: &[Drip] = &[
    Drip { col_off: -1, phase: 0,   dest_rows: 4, speed: 10 },
    Drip { col_off: 3,  phase: 150, dest_rows: 6, speed: 12 },
    Drip { col_off: 2,  phase: 320, dest_rows: 3, speed: 9 },
];

/// Quadrant-block lips for the asymmetrical, sagging melt crater under the flame.
const RIM_CRATER: [char; 3] = ['▄', '▖', '▗'];

/// Draw the candle: an organically curved, sub-cell wax shaft with a sagging melt
/// crater and dynamic liquid wax runs. Returns the flame anchor `(x, y, brightness)`
/// consumed by the light-degradation engine.
fn draw_candle(buf: &mut Buffer, area: Rect, state: &GlobalStateContext) -> (f32, f32, f32) {
    let cx = area.x + 4;          // left column of the 3-wide candle shaft
    let center = cx + 1;
    let holder_y = area.y + area.height.saturating_sub(2);

    let pulse = flame_pulse(state.frame_counter);

    // Map remaining life onto available vertical wax rows.
    let max_wax = area.height.saturating_sub(12).min(12).max(2);
    let wax_rows = ((state.candle_rows_remaining as u32 * max_wax as u32) / 120)
        .min(max_wax as u32) as u16;

    // Dark amber for liquid runs and hardened deposits.
    let wax_run = lerp_color(WAX_CRIMSON, AMBER_GOLD, 0.42);
    let wax_hard = lerp_color(WAX_CRIMSON, AMBER_GOLD, 0.22);

    // Fully melted: a crimson puddle, flame guttered to a faint ember.
    if wax_rows == 0 {
        buf_set_str_fg(buf, cx.saturating_sub(1), holder_y, "▗▄▄▄▖", WAX_CRIMSON);
        let ember_y = holder_y.saturating_sub(1);
        buf_set_fg(buf, center, ember_y, '·', lerp_color(WAX_CRIMSON, AMBER_GOLD, pulse));
        return (center as f32, ember_y as f32, 0.35 + pulse * 0.2);
    }

    // Geometry, drawn bottom → top.
    let wax_bottom = holder_y.saturating_sub(1);
    let wax_top = wax_bottom.saturating_sub(wax_rows - 1);

    // Holder / saucer (curved foot).
    buf_set_str_fg(buf, cx.saturating_sub(1), holder_y, "▟███▙", Color::Rgb(96, 78, 52));

    // ── Wax shaft: organic curve + vertical gradient. The top row is the melt
    //    crater (quadrant blocks); everything below it is solid, displaced body. ──
    let bx_of = |row_y: u16| -> u16 {
        (cx as i16 + shaft_offset(row_y)).max(area.x as i16) as u16
    };
    for i in 0..wax_rows {
        let row_y = wax_bottom.saturating_sub(i);
        let t_from_top = if wax_rows > 1 {
            (wax_rows - 1 - i) as f32 / (wax_rows - 1) as f32
        } else {
            0.0
        };
        let wax_color = if t_from_top < 0.5 {
            lerp_color(AMBER_GOLD, WAX_CRIMSON, t_from_top / 0.5)
        } else {
            lerp_color(WAX_CRIMSON, PITCH, (t_from_top - 0.5) / 0.5)
        };
        let bx = bx_of(row_y);
        if i == wax_rows - 1 {
            // Asymmetrical sagging crater catching the flame's underlight.
            let lip = lerp_color(wax_color, FLAME_WHITE, pulse * 0.35);
            for (k, ch) in RIM_CRATER.iter().enumerate() {
                buf_set_fg(buf, bx + k as u16, row_y, *ch, lip);
            }
        } else {
            buf_set_str_fg(buf, bx, row_y, "███", wax_color);
        }
    }

    // ── Dynamic liquid wax runs down the lateral boundaries. ──
    if area.width >= 12 {
        for drip in DRIPS {
            let dest = drip.dest_rows.min(wax_rows.saturating_sub(1));
            if dest == 0 { continue; }
            let cycle = dest as u64 * drip.speed + 130; // ooze, then hold/harden
            let t = (state.frame_counter + drip.phase) % cycle;

            // A permanent hardened deposit — the organic deformity left behind.
            let dest_y = wax_top + dest;
            let hard_x = (cx as i16 + drip.col_off + shaft_offset(dest_y)).max(area.x as i16) as u16;
            buf_set_fg(buf, hard_x, dest_y, '▐', wax_hard);

            let descend = dest as u64 * drip.speed;
            if t < descend {
                // The drop is still travelling: trail above, bright head at the tip.
                let head_row = (t / drip.speed) as u16;
                let head_y = wax_top + head_row;
                let trail_ch = if drip.col_off < 1 { '▌' } else if drip.col_off > 1 { '▐' } else { '█' };
                for r in 0..head_row {
                    let ry = wax_top + r;
                    let tx = (cx as i16 + drip.col_off + shaft_offset(ry)).max(area.x as i16) as u16;
                    buf_set_fg(buf, tx, ry, trail_ch, wax_hard);
                }
                let hx = (cx as i16 + drip.col_off + shaft_offset(head_y)).max(area.x as i16) as u16;
                buf_set_fg(buf, hx, head_y, '█', wax_run);
            }
        }
    }

    // Flame sits atop the curved rim — wick and flame follow the top displacement.
    let top_off = shaft_offset(wax_top);
    let fcx = (center as i16 + top_off).max(area.x as i16) as u16;
    let wick_y = wax_top.saturating_sub(1);
    let flame_body_y = wick_y.saturating_sub(1);
    let flame_tip_y = flame_body_y.saturating_sub(1);
    let glow_y = flame_tip_y.saturating_sub(1);

    // Wick — a dark thread between wax and flame.
    buf_set_fg(buf, fcx, wick_y, '█', Color::Rgb(38, 32, 28));

    // Flame body & tip — white-amber core fading to amber-gold with the pulse.
    let core = lerp_color(AMBER_GOLD, FLAME_WHITE, pulse);
    let outer = lerp_color(WAX_CRIMSON, AMBER_GOLD, pulse);
    buf_set_fg(buf, fcx.saturating_sub(1), flame_body_y, '▄', outer);
    buf_set_fg(buf, fcx, flame_body_y, '█', core);
    buf_set_fg(buf, fcx + 1, flame_body_y, '▄', outer);
    buf_set_fg(buf, fcx, flame_tip_y, '▀', core);

    // Rising glow ember, only at the crest of the flicker.
    if pulse > 0.80 {
        buf_set_fg(buf, fcx, glow_y, '·', lerp_color(WAX_CRIMSON, AMBER_GOLD, pulse));
    }

    let brightness = (0.70 + pulse * 0.30).clamp(0.0, 1.0);
    (fcx as f32, flame_body_y as f32, brightness)
}

// ═════════════════════════════════════════════════════════════════════════════
// SUBSYSTEM 3: The Shadow Apple (charcoal silhouette + Braille fracture)
// ═════════════════════════════════════════════════════════════════════════════

const APPLE_BODY: &[&str] = &[
    "  ▄▄  ",
    " ████ ",
    "██████",
    "██████",
    " ▀██▀ ",
];

const FRACTURE: &[char] = &['⠓', '⠙', '⠻', '⠵', '⠷', '⠾', '⠫', '⠭'];

/// Draw the cyanide apple as a faint matte silhouette. Each bite does not merely
/// erase text — it fractures Turing's storage matrices into Braille rubble from the
/// bitten edge inward, simulating a collapse of the structural character borders.
fn draw_apple(buf: &mut Buffer, area: Rect, state: &GlobalStateContext) {
    let width = 6u16;
    let ax = area.x + area.width.saturating_sub(width + 2);
    let ay = area.y + area.height.saturating_sub(7);

    // Stem
    buf_set_fg(buf, ax + 2, ay.saturating_sub(1), '╵', Color::Rgb(40, 36, 30));

    let bites = state.apple_bites_taken.min(4) as u16;
    let fracture_from = width.saturating_sub(bites); // columns >= this collapse
    let mut rng = Lcg::new(state.chemical_drift_seed ^ (bites as u64));

    for (r, row) in APPLE_BODY.iter().enumerate() {
        let y = ay + r as u16;
        for (c, ch) in row.chars().enumerate() {
            if ch == ' ' { continue; }
            let col = c as u16;
            let x = ax + col;
            if bites > 0 && col >= fracture_from {
                // Collapsed storage matrix → fractured Braille array
                let glyph = FRACTURE[rng.next_bounded(FRACTURE.len() as u64) as usize];
                buf_set_fg(buf, x, y, glyph, SHADOW_RUBBLE);
            } else {
                // Outer rim (half-blocks / edge columns) is the darker matte tone.
                let edge = ch == '▄' || ch == '▀' || col == 0 || col == width - 1;
                let tone = if edge { SHADOW_DARK } else { SHADOW_MID };
                buf_set_fg(buf, x, y, ch, tone);
            }
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// SUBSYSTEM 4: Chemical Drift — Scattered Stilboestrol
// ═════════════════════════════════════════════════════════════════════════════

fn draw_pills(buf: &mut Buffer, area: Rect, state: &GlobalStateContext) {
    if state.stilboestrol_ppm <= 0.0 { return; }

    let pill_count = ((state.stilboestrol_ppm * 0.3).ceil() as u16).min(30);
    let pill_chars: &[char] = &['o', '.', '·', '°'];
    let cluster_chars: &[&str] = &["oOo", "o.", ".o", "°·"];

    let mut rng = Lcg::new(state.chemical_drift_seed);

    let top = area.y + area.height / 2;
    let left = area.x + 1;
    let right = area.x + area.width.saturating_sub(2);
    let bottom = area.y + area.height.saturating_sub(1);

    for _ in 0..pill_count {
        let x = left + rng.next_bounded((right.saturating_sub(left)).max(1) as u64) as u16;
        let y = top + rng.next_bounded((bottom.saturating_sub(top)).max(1) as u64) as u16;
        if !in_bounds(buf, x, y) { continue; }

        if state.stilboestrol_ppm > 50.0 && rng.next_bounded(3) == 0 {
            let cluster = cluster_chars[rng.next_bounded(cluster_chars.len() as u64) as usize];
            buf_set_str_fg(buf, x, y, cluster, PILL_COLOR);
        } else {
            let ch = pill_chars[rng.next_bounded(pill_chars.len() as u64) as usize];
            let off = (rng.next_bounded(40) as i16) - 20;
            let r = (150i16 + off).clamp(90, 200) as u8;
            buf_set_fg(buf, x, y, ch, Color::Rgb(r, (r as i16 - 8).max(0) as u8, (r as i16 - 20).max(0) as u8));
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// SUBSYSTEM 5: The Ghost Jury
// ═════════════════════════════════════════════════════════════════════════════

/// Solved acts become a silent jury: each dithered portrait fades into a deeply
/// desaturated hue in a dark corner of the desk. When the user shifts focus to the
/// desk (`lookup_active`), the ghosts' gaze (`○○`) glows and flickers in time with
/// the heartbeat metronome.
fn draw_ghost_jury(buf: &mut Buffer, area: Rect, state: &GlobalStateContext) {
    if state.acts_completed.is_empty() { return; }
    // The jury fades up out of black with the rest of the desk on first reveal.
    let reveal = state.desk_reveal.clamp(0.0, 1.0);

    // Heartbeat-synced glow phase: convert bpm into a per-frame oscillation.
    let bpm = state.base_heartbeat_bpm as f32;
    let beats_per_frame = (bpm / 60.0) / 62.5; // ~62.5 fps at the 16ms tick
    let phase = state.frame_counter as f32 * beats_per_frame * std::f32::consts::TAU;
    let beat = ((phase.sin() + 1.0) * 0.5).powi(3); // sharp systolic pulse

    let pw = 12u16; // portrait width
    let ph = 8u16;  // portrait height

    // Dark-corner anchors (top-right, top-left, mid-right, mid-left, bottom edge).
    let anchors: [(u16, u16); 5] = [
        (area.x + area.width.saturating_sub(pw + 1), area.y + 1),
        (area.x + 1, area.y + 1),
        (area.x + area.width.saturating_sub(pw + 1), area.y + area.height / 2),
        (area.x + 1, area.y + area.height / 2),
        (area.x + 1, area.y + area.height.saturating_sub(ph + 1)),
    ];

    for (i, act) in state.acts_completed.iter().enumerate() {
        if i >= anchors.len() { break; }
        let (gx, gy) = anchors[i];
        let face = render_face(&face_for(*act), pw as usize, ph as usize);

        // The portrait itself: a fine Braille engraving, barely surfacing from the
        // dark in a deeply desaturated haunting hue — denser contours glow faintly.
        for (r, cells) in face.iter().enumerate() {
            let y = gy + r as u16;
            for (c, (ch, it)) in cells.iter().enumerate() {
                if *ch == ' ' { continue; }
                let col = lerp_color(Color::Rgb(14, 14, 15), Color::Rgb(52, 52, 56), *it);
                buf_set_fg(buf, gx + c as u16, y, *ch, dim_to_pitch(col, reveal));
            }
        }

        // The gaze — two hollow eyes on the portrait's eye row.
        let eye_y = gy + 3;
        let eye_color = if state.lookup_active {
            // Flicker in sync with the metronome, with a little per-ghost desync.
            let g = (beat * (0.7 + 0.3 * ((i as f32 * 1.7).sin().abs()))).clamp(0.0, 1.0);
            lerp_color(GHOST_HUE, Color::Rgb(200, 150, 90), g)
        } else {
            GHOST_HUE
        };
        let eye_color = dim_to_pitch(eye_color, reveal);
        buf_set_fg(buf, gx + 3, eye_y, '○', eye_color);
        buf_set_fg(buf, gx + pw.saturating_sub(5), eye_y, '○', eye_color);
    }
}
