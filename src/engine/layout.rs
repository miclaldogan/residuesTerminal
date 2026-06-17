use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

// ─────────────────────────────────────────────────────────────────────────────
// Shared workspace texture palette — keeps the left quadrant from collapsing into
// a flat dead-black void when a puzzle is sparse or empty.
// ─────────────────────────────────────────────────────────────────────────────
const GRID_DOT:   Color = Color::Rgb(0x1c, 0x1c, 0x20); // faint blueprint dot
const GRID_CROSS: Color = Color::Rgb(0x26, 0x26, 0x2e); // brighter node at majors
const SHADOW_BG:  Color = Color::Rgb(0x0d, 0x0d, 0x0d); // drop-shadow ink
const GAUGE_FG:   Color = Color::Rgb(0x8a, 0x78, 0x52); // brass gauge caps
const GAUGE_DIM:  Color = Color::Rgb(0x46, 0x3e, 0x2c); // engraved ruler ticks

fn cell_in(buf: &Buffer, x: u16, y: u16) -> bool {
    let a = buf.area();
    x >= a.x && x < a.x + a.width && y >= a.y && y < a.y + a.height
}

/// Fill `area` with the workspace base colour and lay a very faint, desaturated
/// engineering dot-matrix / blueprint grid over it. This is a background texture
/// sheet: puzzle elements are drawn *after* this and simply overwrite the cells they
/// occupy, so empty regions keep the grid instead of becoming flat black.
pub fn draw_workspace_grid(buf: &mut Buffer, area: Rect, base: Color) {
    for y in area.y..area.y.saturating_add(area.height) {
        for x in area.x..area.x.saturating_add(area.width) {
            if !cell_in(buf, x, y) {
                continue;
            }
            let lx = (x - area.x) as u32;
            let ly = (y - area.y) as u32;
            let cell = buf.get_mut(x, y);
            cell.set_char(' ');
            cell.fg = base;
            cell.bg = base;
            if lx % 6 == 0 && ly % 4 == 0 {
                cell.set_char('┼');
                cell.fg = GRID_CROSS;
            } else if lx % 3 == 0 && ly % 2 == 0 {
                cell.set_char('·');
                cell.fg = GRID_DOT;
            }
        }
    }
}

/// Cast a one-cell drop shadow down the right and bottom edges of an element rect by
/// darkening the background of the grid cells just outside it — giving the puzzle
/// container a physical depth-drop against the blueprint sheet.
pub fn draw_drop_shadow(buf: &mut Buffer, rect: Rect) {
    if rect.width == 0 || rect.height == 0 {
        return;
    }
    let rx = rect.x.saturating_add(rect.width);
    let by = rect.y.saturating_add(rect.height);
    for y in rect.y.saturating_add(1)..=by {
        if cell_in(buf, rx, y) {
            buf.get_mut(rx, y).bg = SHADOW_BG;
        }
    }
    for x in rect.x.saturating_add(1)..=rx {
        if cell_in(buf, x, by) {
            buf.get_mut(x, by).bg = SHADOW_BG;
        }
    }
}

/// Render a single status line at the bottom of `area`, framed like an engraved
/// brass measuring gauge: a ticked ruler along the bottom edge with sliding-gauge
/// end-caps around the text. Replaces the bare "game help" footer.
pub fn draw_gauge_footer(buf: &mut Buffer, area: Rect, label: &str, fg: Color) {
    if area.width < 8 || area.height < 4 {
        return;
    }
    let x0 = area.x;
    let w = area.width;
    let rule_y = area.y + area.height - 1; // along the bottom border edge
    let text_y = area.y + area.height - 2;

    // Engraved ruler with tick marks every four cells (corners left intact).
    for i in 1..w - 1 {
        let x = x0 + i;
        if !cell_in(buf, x, rule_y) {
            continue;
        }
        let ch = if i % 4 == 0 { '┴' } else { '─' };
        let c = buf.get_mut(x, rule_y);
        c.set_char(ch);
        c.fg = GAUGE_DIM;
    }

    // Sliding-gauge end-caps bracketing the status text.
    let lcap = x0 + 1;
    let rcap = x0 + w - 2;
    if cell_in(buf, lcap, text_y) {
        let c = buf.get_mut(lcap, text_y);
        c.set_char('╟');
        c.fg = GAUGE_FG;
    }
    if cell_in(buf, rcap, text_y) {
        let c = buf.get_mut(rcap, text_y);
        c.set_char('╢');
        c.fg = GAUGE_FG;
    }

    // The status text itself, between the caps.
    let tx = lcap + 2;
    let max = rcap.saturating_sub(tx) as usize;
    let mut x = tx;
    for ch in label.chars().take(max) {
        if cell_in(buf, x, text_y) {
            let c = buf.get_mut(x, text_y);
            c.set_char(ch);
            c.fg = fg;
        }
        x += 1;
    }
}

pub struct EngineLayout {
    pub mind_rect: Rect,      // Üst Panel: Zihin Akışı ve Loglar
    pub workspace_rect: Rect, // Alt Sol: Etkin Bulmaca ve Kodlama Alanı
    pub desk_rect: Rect,      // Alt Sağ: Kaotik Masa, Kağıtlar ve Objeler
}

impl EngineLayout {
    /// Gelen terminal ekran boyutuna göre asenkron 3 bölgeyi hesaplar.
    /// Negatif boyut veya taşmaları engellemek için saturating aritmetik kullanır.
    pub fn compute(terminal_area: Rect) -> Self {
        let area_x = terminal_area.x;
        let area_y = terminal_area.y;
        let area_w = terminal_area.width;
        let area_h = terminal_area.height;

        let mind_h = 5u16;
        let actual_mind_h = mind_h.min(area_h);

        let mind_rect = Rect::new(
            area_x,
            area_y,
            area_w,
            actual_mind_h,
        );

        let remaining_h = area_h.saturating_sub(actual_mind_h);
        let bottom_y = area_y.saturating_add(actual_mind_h);

        let workspace_w = ((area_w as u32 * 55) / 100) as u16;
        let desk_w = area_w.saturating_sub(workspace_w);

        let workspace_rect = Rect::new(
            area_x,
            bottom_y,
            workspace_w,
            remaining_h,
        );

        let desk_rect = Rect::new(
            area_x.saturating_add(workspace_w),
            bottom_y,
            desk_w,
            remaining_h,
        );

        Self {
            mind_rect,
            workspace_rect,
            desk_rect,
        }
    }
}
