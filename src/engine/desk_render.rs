use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Color,
    Frame,
};

use super::state::{Act, GlobalStateContext};

// ─────────────────────────────────────────────────────────────────────────────
// Amber phosphor palette for the archive column. The old candlelit desk — the
// overlapping parchment stack, the Braille-engraved face portraits, the candle
// light-degradation engine and the ghost jury — has been retired in favour of a
// clean, flat, grid-aligned archive that cannot shear or overlap its neighbours.
// ─────────────────────────────────────────────────────────────────────────────
const PITCH:     Color = Color::Rgb(10, 8, 0);    // void / panel background
const BORDER:    Color = Color::Rgb(92, 68, 0);   // card + box frame
const TITLE:     Color = Color::Rgb(255, 213, 102);
const AMBER:     Color = Color::Rgb(255, 176, 0); // bright values
const LABEL:     Color = Color::Rgb(153, 104, 10); // dim labels
const DIMA:      Color = Color::Rgb(74, 50, 5);   // faint, layered-edge cards
const SHADE_LO:  Color = Color::Rgb(60, 44, 5);   // ░ unpunched
const SHADE_MID: Color = Color::Rgb(150, 104, 10);// ▒ half
const SHADE_HI:  Color = Color::Rgb(255, 176, 0); // █ punched

// Front card geometry. Two further cards peek from behind it, each offset by one
// row + two columns, producing the layered-stack edge lines.
const CARD_W_MAX: u16 = 26;
const CARD_H: u16 = 7;
const STACK_DEPTH: u16 = 2;
const CARD_TOTAL_H: u16 = CARD_H + STACK_DEPTH; // front card + peeking back rows

struct PaperMeta {
    act: Act,
    title: &'static str,
    year: &'static str,
    punch: &'static str,
    signature: &'static str,
}

const PAPERS: &[PaperMeta] = &[
    PaperMeta { act: Act::Jacquard1804, title: "JACQUARD", year: "1804", punch: "24-COL / 80-ROW", signature: "J.M. Jacquard" },
    PaperMeta { act: Act::Babbage1837,  title: "BABBAGE",  year: "1837", punch: "31-COL / 50-ROW", signature: "Charles Babbage" },
    PaperMeta { act: Act::Lovelace1843, title: "LOVELACE", year: "1843", punch: "26-COL / 64-ROW", signature: "A. A. Lovelace" },
    PaperMeta { act: Act::Boole1854,    title: "BOOLE",    year: "1854", punch: "02-COL / 16-ROW", signature: "George Boole" },
    PaperMeta { act: Act::Shannon1937,  title: "SHANNON",  year: "1937", punch: "16-COL / 32-ROW", signature: "Claude E. Shannon" },
];

fn paper_for(act: Act) -> &'static PaperMeta {
    PAPERS.iter().find(|p| p.act == act).unwrap_or(&PAPERS[0])
}

// ─────────────────────────────────────────────────────────────────────────────
// Safe buffer write helpers
// ─────────────────────────────────────────────────────────────────────────────

fn in_bounds(buf: &Buffer, x: u16, y: u16) -> bool {
    let Rect { x: ax, y: ay, width: aw, height: ah } = *buf.area();
    x >= ax && x < ax + aw && y >= ay && y < ay + ah
}

fn put(buf: &mut Buffer, x: u16, y: u16, ch: char, fg: Color, bg: Color) {
    if in_bounds(buf, x, y) {
        let c = buf.get_mut(x, y);
        c.set_char(ch);
        c.fg = fg;
        c.bg = bg;
    }
}

fn put_str(buf: &mut Buffer, x: u16, y: u16, s: &str, fg: Color, bg: Color) {
    let mut col = x;
    for ch in s.chars() {
        put(buf, col, y, ch, fg, bg);
        col = col.saturating_add(1);
    }
}

/// Truncate to at most `max` characters (multibyte-safe).
fn clip(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

// ═════════════════════════════════════════════════════════════════════════════
// PUBLIC ENTRY POINT
// ═════════════════════════════════════════════════════════════════════════════

/// Render the right column [ARCHIVE · THE DESK]: a clean, bordered panel holding a
/// layered punch-card stack drawn with crisp edge lines, and — beneath it — a
/// typewriter-style document block describing the active reel. Everything is flat
/// and grid-aligned, so it never overlaps the centre column or shears columns.
pub fn render_desk(f: &mut Frame, area: Rect, state: &GlobalStateContext) {
    let buf = f.buffer_mut();
    if area.width < 12 || area.height < 10 {
        return;
    }

    // 1. Void fill
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let c = buf.get_mut(x, y);
            c.set_char(' ');
            c.fg = PITCH;
            c.bg = PITCH;
        }
    }

    // 2. Panel border + title
    draw_box(buf, area, BORDER);
    put_str(buf, area.x + 2, area.y, " [03] ARCHIVE · THE DESK ", TITLE, PITCH);

    let paper = paper_for(state.current_act);
    let solved = state.acts_completed.contains(&state.current_act);

    let inner_x = area.x + 2;
    let inner_w = area.width.saturating_sub(4);

    // 3. Layered punch-card stack
    draw_card_stack(buf, inner_x, area.y + 2, inner_w, paper);

    // 4. Typewriter documentation, padded neatly inside the boundary
    let doc_y = area.y + 2 + CARD_TOTAL_H + 1;
    draw_document(buf, inner_x, doc_y, inner_w, paper, solved);
}

// ═════════════════════════════════════════════════════════════════════════════
// The layered punch-card stack
// ═════════════════════════════════════════════════════════════════════════════

/// Draw a hollow rectangular frame (border only).
fn draw_box(buf: &mut Buffer, area: Rect, col: Color) {
    let x1 = area.x + area.width - 1;
    let y1 = area.y + area.height - 1;
    put(buf, area.x, area.y, '┌', col, PITCH);
    put(buf, x1, area.y, '┐', col, PITCH);
    put(buf, area.x, y1, '└', col, PITCH);
    put(buf, x1, y1, '┘', col, PITCH);
    for i in 1..area.width - 1 {
        put(buf, area.x + i, area.y, '─', col, PITCH);
        put(buf, area.x + i, y1, '─', col, PITCH);
    }
    for j in 1..area.height - 1 {
        put(buf, area.x, area.y + j, '│', col, PITCH);
        put(buf, x1, area.y + j, '│', col, PITCH);
    }
}

/// A bordered card outline at (x, y) sized w×h, filling its interior with the void
/// so it cleanly masks anything drawn behind it.
fn card_outline(buf: &mut Buffer, x: u16, y: u16, w: u16, h: u16, col: Color, fill: bool) {
    if w < 2 || h < 2 {
        return;
    }
    if fill {
        for j in 0..h {
            for i in 0..w {
                put(buf, x + i, y + j, ' ', col, PITCH);
            }
        }
    }
    let x1 = x + w - 1;
    let y1 = y + h - 1;
    put(buf, x, y, '┌', col, PITCH);
    put(buf, x1, y, '┐', col, PITCH);
    put(buf, x, y1, '└', col, PITCH);
    put(buf, x1, y1, '┘', col, PITCH);
    for i in 1..w - 1 {
        put(buf, x + i, y, '─', col, PITCH);
        put(buf, x + i, y1, '─', col, PITCH);
    }
    for j in 1..h - 1 {
        put(buf, x, y + j, '│', col, PITCH);
        put(buf, x1, y + j, '│', col, PITCH);
    }
}

/// Colour a punch-card shading run: ░ dim, ▒ mid, █ bright — the phosphor depth
/// gradient that reads as punched / half / unpunched columns.
fn put_shade(buf: &mut Buffer, x: u16, y: u16, s: &str) {
    let mut col = x;
    for ch in s.chars() {
        match ch {
            '░' => put(buf, col, y, ch, SHADE_LO, PITCH),
            '▒' => put(buf, col, y, ch, SHADE_MID, PITCH),
            '█' => put(buf, col, y, ch, SHADE_HI, PITCH),
            _ => {} // spaces stay void
        }
        col = col.saturating_add(1);
    }
}

fn draw_card_stack(buf: &mut Buffer, ox: u16, oy: u16, avail_w: u16, paper: &PaperMeta) {
    let w = avail_w.saturating_sub(STACK_DEPTH * 2).min(CARD_W_MAX).max(12);

    // Back cards first (drawn deepest-first), each offset down-right so their lower
    // edges peek out beneath/right of the card on top — the layered-stack look.
    for k in (1..=STACK_DEPTH).rev() {
        card_outline(buf, ox + k * 2, oy + k, w, CARD_H, DIMA, false);
    }

    // The front card, fully detailed on top.
    card_outline(buf, ox, oy, w, CARD_H, BORDER, true);

    let inner = w.saturating_sub(4) as usize; // shading field width
    let pattern = "░▒█ ▒█░ █░▒ ░▒█ ▒█░ █░▒ ";
    let phases = [0usize, 2, 4];
    for r in 0..3u16 {
        let off = phases[r as usize % phases.len()];
        let s: String = pattern.chars().cycle().skip(off).take(inner).collect();
        put_shade(buf, ox + 2, oy + 1 + r, &s);
    }

    // The '───┤' index-tab divider, then the reel id, inside the card.
    let x1 = ox + w - 1;
    put(buf, ox, oy + 4, '├', BORDER, PITCH);
    for i in 1..w - 1 {
        put(buf, ox + i, oy + 4, '─', BORDER, PITCH);
    }
    put(buf, x1, oy + 4, '┤', BORDER, PITCH);

    let id = clip(&format!("REEL #{} · A1", paper.year), inner);
    put_str(buf, ox + 2, oy + 5, &id, AMBER, PITCH);
}

// ═════════════════════════════════════════════════════════════════════════════
// The typewriter document block
// ═════════════════════════════════════════════════════════════════════════════

fn draw_document(buf: &mut Buffer, ox: u16, oy: u16, avail_w: u16, paper: &PaperMeta, solved: bool) {
    let w = avail_w.min(30).max(16);
    let inner = w.saturating_sub(4) as usize;
    let x1 = ox + w - 1;

    let body = [
        format!("SOURCE : {} REEL", paper.title),
        format!("         #{}", paper.year),
        format!("CARD ID: #{}-A1", paper.year),
        format!("PUNCH  : {}", paper.punch),
        format!("STATUS : {}", if solved { "VERIFIED [OK]" } else { "ACTIVE" }),
    ];

    // Top edge
    put(buf, ox, oy, '┌', BORDER, PITCH);
    put(buf, x1, oy, '┐', BORDER, PITCH);
    for i in 1..w - 1 {
        put(buf, ox + i, oy, '─', BORDER, PITCH);
    }

    let mut ly = oy + 1;
    for line in &body {
        put(buf, ox, ly, '│', BORDER, PITCH);
        put(buf, x1, ly, '│', BORDER, PITCH);
        draw_doc_line(buf, ox + 2, ly, &clip(line, inner));
        ly += 1;
    }

    // Separator
    put(buf, ox, ly, '├', BORDER, PITCH);
    for i in 1..w - 1 {
        put(buf, ox + i, ly, '─', BORDER, PITCH);
    }
    put(buf, x1, ly, '┤', BORDER, PITCH);
    ly += 1;

    // Hand-signed authentication line
    put(buf, ox, ly, '│', BORDER, PITCH);
    put(buf, x1, ly, '│', BORDER, PITCH);
    let auth = clip(&format!("auth.  {}", paper.signature), inner);
    put_str(buf, ox + 2, ly, &auth, LABEL, PITCH);
    ly += 1;

    // Bottom edge
    put(buf, ox, ly, '└', BORDER, PITCH);
    put(buf, x1, ly, '┘', BORDER, PITCH);
    for i in 1..w - 1 {
        put(buf, ox + i, ly, '─', BORDER, PITCH);
    }
}

/// Render a "LABEL : value" document line with the label dimmed and the value bright.
fn draw_doc_line(buf: &mut Buffer, x: u16, y: u16, line: &str) {
    if let Some(idx) = line.find(':') {
        let (label, value) = line.split_at(idx + 1);
        put_str(buf, x, y, label, LABEL, PITCH);
        put_str(buf, x + label.chars().count() as u16, y, value, AMBER, PITCH);
    } else {
        put_str(buf, x, y, line, AMBER, PITCH);
    }
}
