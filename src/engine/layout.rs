use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Color;

// ─────────────────────────────────────────────────────────────────────────────
// Shared workspace texture palette — keeps the left quadrant from collapsing into
// a flat dead-black void when a puzzle is sparse or empty.
// ─────────────────────────────────────────────────────────────────────────────
const GRID_DOT:   Color = Color::Rgb(0x1A, 0x14, 0x00); // faint amber dot
const GRID_CROSS: Color = Color::Rgb(0x2A, 0x20, 0x00); // brighter amber node
const SHADOW_BG:  Color = Color::Rgb(0x08, 0x06, 0x00); // drop-shadow ink
const GAUGE_FG:   Color = Color::Rgb(0x99, 0x68, 0x0A); // amber gauge caps
const GAUGE_DIM:  Color = Color::Rgb(0x4A, 0x32, 0x05); // dim amber ticks

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
    pub top_bar_rect: Rect,     // row 0          — 1-row status header (preserved)
    pub mind_rect: Rect,        // left column    — Narrative Log / The Mind
    pub workspace_rect: Rect,   // center column  — The Loom Engine
    pub desk_rect: Rect,        // right column   — Archive / The Desk
    pub bottom_bar_rect: Rect,  // bottom 3 rows  — telemetry + shell legend
}

impl EngineLayout {
    /// Carve the screen into the agreed 3-column grid with a consolidated bottom bar.
    ///
    /// Vertical:
    ///   top_bar_area    Length(1)  — kept for the act/clock header
    ///   main_game_area  Min(10)    — the three parallel play columns
    ///   bottom_bar_area Length(3)  — 2 rows of telemetry + a separator rule
    ///
    /// Horizontal (within main_game_area):
    ///   left_col   Percentage(35) — Narrative Log
    ///   center_col Percentage(35) — The Loom Engine
    ///   right_col  Percentage(30) — Archive / Desk
    pub fn compute(screen_rect: Rect) -> Self {
        let vchunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // top_bar_area
                Constraint::Min(10),    // main_game_area
                Constraint::Length(3),  // bottom_bar_area
            ])
            .split(screen_rect);

        let top_bar_rect = vchunks[0];
        let main_game_area = vchunks[1];
        let bottom_bar_rect = vchunks[2];

        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(35), // left_col   — Narrative Log
                Constraint::Percentage(35), // center_col — The Loom Engine
                Constraint::Percentage(30), // right_col  — Archive / Desk
            ])
            .split(main_game_area);

        Self {
            top_bar_rect,
            mind_rect: cols[0],
            workspace_rect: cols[1],
            desk_rect: cols[2],
            bottom_bar_rect,
        }
    }
}
