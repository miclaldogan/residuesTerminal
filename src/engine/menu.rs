use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::Frame;

use super::image_engine;
use super::save::SaveState;
use super::state::Act;

/// The menu backdrop asset — the candlelit antique desk, projected dimmed beneath the
/// text layers via the half-block image engine.
const MENU_BACKDROP: &str = "openingBg.png";

// ─────────────────────────────────────────────────────────────────────────────
// The Main Menu — phase 0, drawn before a single byte of simulation runs.
//
// A self-contained, dependency-free TUI state: it owns only its cursor, the audio
// toggle, and the snapshot of save progress needed to label the first option. The
// composition mirrors the candlelit antique desk of the reference art, rendered in
// strict TrueColor block glyphs + box-drawing so it lands identically on Windows
// Terminal and any Linux shell — no emoji, no protocol extensions, nothing that
// fragments. Every draw is bounds-clipped, so it degrades gracefully on a 20-column
// shell instead of panicking or smearing.
// ─────────────────────────────────────────────────────────────────────────────

/// What committing a given row does. Decouples input handling from the on-screen row
/// order, so the same handler works for the 3-row fresh layout and the 4-row save
/// layout without any brittle index arithmetic.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuAction {
    /// Open the act picker to re-enter any unlocked act.
    SelectAct,
    /// Wipe any existing save and boot the cinematic prelude from Act I.
    NewGame,
    /// Flip the audio master mute.
    AudioToggle,
    /// Quit to the shell.
    Exit,
}

/// Which screen the menu is currently showing: the top-level choices, or the act
/// picker reached from "RE-ENTER THE MIND".
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuMode {
    Main,
    ActSelect,
}

/// Canonical act order, used to enumerate the unlocked acts in the picker.
const ACT_ORDER: [Act; 6] = [
    Act::Jacquard1804,
    Act::Babbage1837,
    Act::Lovelace1843,
    Act::Boole1854,
    Act::Shannon1937,
    Act::Turing1936_1950,
];

/// A short one-line act label for the picker rows.
fn act_title(act: Act) -> &'static str {
    match act {
        Act::Jacquard1804 => "ACT I    \u{00B7}  JACQUARD  \u{00B7}  1804",
        Act::Babbage1837 => "ACT II   \u{00B7}  BABBAGE   \u{00B7}  1837",
        Act::Lovelace1843 => "ACT III  \u{00B7}  LOVELACE  \u{00B7}  1843",
        Act::Boole1854 => "ACT IV   \u{00B7}  BOOLE     \u{00B7}  1854",
        Act::Shannon1937 => "ACT V    \u{00B7}  SHANNON   \u{00B7}  1937",
        Act::Turing1936_1950 => "ACT VI   \u{00B7}  TURING    \u{00B7}  1936",
    }
}

pub struct MainMenu {
    pub selected: usize,
    /// True when a valid checkpoint exists — drives the Continue vs. New Game label.
    pub has_save: bool,
    /// The furthest act reached — the resume point and the cap on the act picker.
    pub resume_act: Act,
    /// Restored progress vector, handed back to the global state on re-entry.
    pub acts_completed: Vec<Act>,
    /// The audio master toggle the player flips from this screen.
    pub audio_on: bool,
    /// Top-level choices vs. the act picker.
    pub mode: MenuMode,
    /// Cursor within the act picker (0-based over the unlocked acts).
    pub act_cursor: usize,
}

impl MainMenu {
    /// Build the menu from an optional loaded checkpoint. With a save present, option
    /// one becomes "RE-ENTER THE MIND"; without it, "INITIATE CORE ENGINE".
    pub fn new(save: Option<&SaveState>) -> Self {
        match save {
            Some(s) => Self {
                selected: 0,
                has_save: true,
                resume_act: s.current_act,
                acts_completed: s.acts_completed.clone(),
                audio_on: true,
                mode: MenuMode::Main,
                act_cursor: 0,
            },
            None => Self {
                selected: 0,
                has_save: false,
                resume_act: Act::Jacquard1804,
                acts_completed: Vec::new(),
                audio_on: true,
                mode: MenuMode::Main,
                act_cursor: 0,
            },
        }
    }

    /// How many acts the player has unlocked (1-based count up to the furthest reached).
    pub fn unlocked_count(&self) -> usize {
        act_index(self.resume_act) as usize
    }

    /// The unlocked acts, oldest first — the rows shown in the picker.
    pub fn unlocked_acts(&self) -> Vec<Act> {
        ACT_ORDER[..self.unlocked_count().min(ACT_ORDER.len())].to_vec()
    }

    /// Enter the act picker, parking the cursor on the furthest (latest) act so a quick
    /// double-Enter behaves exactly like the old "continue at the latest act".
    pub fn open_act_select(&mut self) {
        self.mode = MenuMode::ActSelect;
        self.act_cursor = self.unlocked_count().saturating_sub(1);
    }

    /// The act the picker cursor is on.
    pub fn selected_act(&self) -> Act {
        let acts = self.unlocked_acts();
        acts[self.act_cursor.min(acts.len().saturating_sub(1))]
    }

    /// Picker cursor up, wrapping against the unlocked-act count.
    pub fn act_up(&mut self) {
        let n = self.unlocked_count().max(1);
        self.act_cursor = (self.act_cursor + n - 1) % n;
    }

    /// Picker cursor down, wrapping against the unlocked-act count.
    pub fn act_down(&mut self) {
        let n = self.unlocked_count().max(1);
        self.act_cursor = (self.act_cursor + 1) % n;
    }

    /// Number of selectable rows: 4 when a save exists (Continue + Erase + Audio +
    /// Exit), otherwise 3 (New Game + Audio + Exit). Single source of truth for every
    /// bound check, so the cursor can never index past the active list.
    pub fn option_count(&self) -> usize {
        if self.has_save { 4 } else { 3 }
    }

    /// The active rows as `(label, action)` pairs, branched on save presence. Rebuilt on
    /// demand so the AUDIO label and the Continue act number always reflect live state.
    pub fn options(&self) -> Vec<(String, MenuAction)> {
        let audio = format!("AUDIO  [{}]", if self.audio_on { "ON" } else { "OFF" });
        if self.has_save {
            vec![
                ("[1] RE-ENTER THE MIND  (Choose Act)".to_string(), MenuAction::SelectAct),
                ("[2] ERASE MEMORY CORE  (Restart / New Game)".to_string(), MenuAction::NewGame),
                (format!("[3] {}", audio), MenuAction::AudioToggle),
                ("[4] EXIT TO SHELL".to_string(), MenuAction::Exit),
            ]
        } else {
            vec![
                ("[1] INITIATE CORE ENGINE  (New Game)".to_string(), MenuAction::NewGame),
                (format!("[2] {}", audio), MenuAction::AudioToggle),
                ("[3] EXIT TO SHELL".to_string(), MenuAction::Exit),
            ]
        }
    }

    /// The action the currently-highlighted row commits to.
    pub fn selected_action(&self) -> MenuAction {
        let opts = self.options();
        opts[self.selected.min(opts.len() - 1)].1
    }

    /// Move the cursor up, wrapping around the top of the active list.
    pub fn up(&mut self) {
        let n = self.option_count();
        self.selected = (self.selected + n - 1) % n;
    }

    /// Move the cursor down, wrapping around the bottom of the active list.
    pub fn down(&mut self) {
        let n = self.option_count();
        self.selected = (self.selected + 1) % n;
    }
}

/// One-based act number, used for the "Continue (Act N)" readout.
fn act_index(act: Act) -> u8 {
    match act {
        Act::Jacquard1804 => 1,
        Act::Babbage1837 => 2,
        Act::Lovelace1843 => 3,
        Act::Boole1854 => 4,
        Act::Shannon1937 => 5,
        Act::Turing1936_1950 => 6,
    }
}

// ── Palette ─────────────────────────────────────────────────────────────────
// All warm browns and candle amber, lifted from the reference desk so the menu
// establishes the historical-computing mood instantly.
const BG_TOP_RGB: (u8, u8, u8) = (12, 9, 6);
const BG_BOTTOM_RGB: (u8, u8, u8) = (3, 2, 1);
const TITLE_HOT: (u8, u8, u8) = (205, 127, 50); // #CD7F32 — soft amber/bronze
const TITLE_COLD: (u8, u8, u8) = (70, 62, 44); // #463E2C — settled into shadow
const SUBTITLE: Color = Color::Rgb(120, 96, 60);
const BORDER: Color = Color::Rgb(96, 76, 50);
const ITEM_DIM: Color = Color::Rgb(140, 112, 74);
const ITEM_HOT: Color = Color::Rgb(255, 196, 120);
const SELECT_BG: Color = Color::Rgb(28, 20, 10);
const PANEL_FILL: Color = Color::Rgb(12, 9, 5); // near-opaque card under the choices
const FOOTER: Color = Color::Rgb(110, 88, 58);

/// Linear interpolate two 8-bit RGB triples at `t` in `0.0..=1.0`.
fn lerp(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color::Rgb(f(a.0, b.0), f(a.1, b.1), f(a.2, b.2))
}

/// Bounds-clipped single-cell write — the foundation every other draw rests on, so
/// nothing can ever index outside the buffer no matter how narrow the terminal.
fn put(buf: &mut Buffer, x: u16, y: u16, ch: char, fg: Color, bg: Color) {
    let Rect { x: ax, y: ay, width: aw, height: ah } = *buf.area();
    if x >= ax && x < ax + aw && y >= ay && y < ay + ah {
        let cell = buf.get_mut(x, y);
        cell.set_char(ch);
        cell.fg = fg;
        cell.bg = bg;
    }
}

/// Bounds-clipped string write on a transparent background (keeps the gradient).
fn put_str(buf: &mut Buffer, mut x: u16, y: u16, s: &str, fg: Color) {
    for ch in s.chars() {
        let bg = bg_at(buf, x, y);
        put(buf, x, y, ch, fg, bg);
        x = x.saturating_add(1);
    }
}

/// Sample the existing background colour at a cell so text overlays without punching
/// a black hole in the gradient.
fn bg_at(buf: &Buffer, x: u16, y: u16) -> Color {
    let Rect { x: ax, y: ay, width: aw, height: ah } = *buf.area();
    if x >= ax && x < ax + aw && y >= ay && y < ay + ah {
        buf.get(x, y).bg
    } else {
        Color::Rgb(BG_BOTTOM_RGB.0, BG_BOTTOM_RGB.1, BG_BOTTOM_RGB.2)
    }
}

// ── Title block font — 5 rows tall, every glyph 4 columns wide ───────────────
fn glyph(c: char) -> [&'static str; 5] {
    match c {
        'R' => ["███ ", "█  █", "███ ", "█ █ ", "█  █"],
        'E' => ["████", "█   ", "███ ", "█   ", "████"],
        'S' => ["████", "█   ", "████", "   █", "████"],
        'I' => ["████", " ██ ", " ██ ", " ██ ", "████"],
        'D' => ["███ ", "█  █", "█  █", "█  █", "███ "],
        'U' => ["█  █", "█  █", "█  █", "█  █", "████"],
        _ => ["    ", "    ", "    ", "    ", "    "],
    }
}

const TITLE: &str = "RESIDUES";
const GLYPH_W: u16 = 4;
const GLYPH_GAP: u16 = 2;

/// Pixel width of the full title block (glyphs + inter-glyph gaps).
fn title_width() -> u16 {
    let n = TITLE.chars().count() as u16;
    n * GLYPH_W + n.saturating_sub(1) * GLYPH_GAP
}

/// Paint the block-art title centred on `cx`, top row at `y`, with a vertical amber
/// gradient (hot at the crown, cooling into shadow at the base).
fn draw_title(buf: &mut Buffer, cx: u16, y: u16) {
    let start_x = cx.saturating_sub(title_width() / 2);
    for (row, _) in (0..5).enumerate() {
        let color = lerp(TITLE_HOT, TITLE_COLD, row as f32 / 4.0);
        let mut gx = start_x;
        for ch in TITLE.chars() {
            let pattern = glyph(ch)[row];
            for (i, pc) in pattern.chars().enumerate() {
                if pc == '█' {
                    put(buf, gx + i as u16, y + row as u16, '█', color, bg_at(buf, gx + i as u16, y + row as u16));
                }
            }
            gx += GLYPH_W + GLYPH_GAP;
        }
    }
}

/// Render the whole menu. Self-guarding: a generous full composition above a width/
/// height threshold, a clean stacked fallback below it — never a panic.
pub fn render_main_menu(f: &mut Frame, area: Rect, menu: &MainMenu, frame: u64) {
    // 1. Backdrop: the candlelit desk photo, dimmed to 25% brightness so it reads as a
    //    faint, low-contrast silhouette beneath the text. If the asset is unavailable we
    //    fall back to the original vertical gradient wash — never a "SIGNAL LOST" card.
    let bg_path = image_engine::images_dir().join(MENU_BACKDROP);
    let has_backdrop = image_engine::draw_backdrop(f, area, &bg_path, 0.25, frame);

    let buf = f.buffer_mut();
    if !has_backdrop {
        // Vertical gradient wash — the dim, candle-warmed desk surface (fallback).
        let h = area.height.max(1);
        for y in area.y..area.y + area.height {
            let t = (y - area.y) as f32 / h as f32;
            let bg = lerp((BG_TOP_RGB.0, BG_TOP_RGB.1, BG_TOP_RGB.2), (BG_BOTTOM_RGB.0, BG_BOTTOM_RGB.1, BG_BOTTOM_RGB.2), t);
            for x in area.x..area.x + area.width {
                let cell = buf.get_mut(x, y);
                cell.set_char(' ');
                cell.bg = bg;
                cell.fg = bg;
            }
        }
    }

    let cx = area.x + area.width / 2;
    let compact = area.width < 50 || area.height < 22;

    // The act picker is its own composition, drawn over the same backdrop.
    if menu.mode == MenuMode::ActSelect {
        render_act_select(buf, area, menu, cx, compact);
        return;
    }

    // 2. Dynamic option list — 3 rows fresh, 4 rows with a save (the extra ERASE row).
    let options = menu.options();

    if compact {
        // ── Stacked fallback for narrow/short terminals ──
        let mut y = area.y + area.height / 2;
        y = y.saturating_sub(2 + options.len() as u16 / 2);
        let title = "R E S I D U E S";
        put_str(buf, cx.saturating_sub(title.chars().count() as u16 / 2), y, title, lerp(TITLE_HOT, TITLE_COLD, 0.0));
        y += 2;
        for (i, (label, _)) in options.iter().enumerate() {
            let sel = i == menu.selected;
            let prefix = if sel { "> " } else { "  " };
            let line = format!("{}{}", prefix, label);
            let color = if sel { ITEM_HOT } else { ITEM_DIM };
            put_str(buf, cx.saturating_sub(line.chars().count() as u16 / 2), y + i as u16, &line, color);
        }
        return;
    }

    // ── Full composition ──
    // The panel grows with the option count (2 rows of frame + 2 rows per choice).
    let panel_h: u16 = options.len() as u16 * 2 + 3;
    // Vertically centre the title + subtitle + panel cluster, biased slightly up.
    let cluster_h: u16 = 5 + 2 + 1 + 2 + panel_h; // title, gap, subtitle, gap, panel
    let start_y = area.y + area.height.saturating_sub(cluster_h) / 2;
    let start_y = start_y.saturating_sub(1).max(area.y + 1);

    // 4. Title block (the real desk silhouette now lives in the backdrop layer).
    draw_title(buf, cx, start_y);

    // 5. Subtitle rule.
    let subtitle = "— A N   E N G I N E   O F   R E S I D U A L   M I N D S —";
    put_str(buf, cx.saturating_sub(subtitle.chars().count() as u16 / 2), start_y + 7, subtitle, SUBTITLE);

    // 6. Menu panel — a rounded amber-bordered box framing the choices. Its interior is
    //    filled with a near-opaque dark card so the options read cleanly over the
    //    dimmed desk backdrop behind it.
    let panel_w: u16 = 54.min(area.width.saturating_sub(6));
    let panel_x = cx.saturating_sub(panel_w / 2);
    let panel_y = start_y + 10;
    for y in panel_y..panel_y + panel_h {
        for x in panel_x..panel_x + panel_w {
            put(buf, x, y, ' ', PANEL_FILL, PANEL_FILL);
        }
    }
    draw_panel(buf, panel_x, panel_y, panel_w, panel_h);

    // 7. The choices, single-spaced inside the panel.
    let inner_x = panel_x + 1;
    let inner_w = panel_w.saturating_sub(2);
    for (i, (label, _)) in options.iter().enumerate() {
        let row_y = panel_y + 2 + i as u16 * 2;
        let sel = i == menu.selected;
        let (fg, bg, marker) = if sel {
            (ITEM_HOT, SELECT_BG, '▶')
        } else {
            (ITEM_DIM, bg_at(buf, inner_x, row_y), ' ')
        };
        // Paint the selection band across the inner width first.
        for x in inner_x..inner_x + inner_w {
            put(buf, x, row_y, ' ', fg, bg);
        }
        put(buf, inner_x + 1, row_y, marker, fg, bg);
        for (j, ch) in label.chars().enumerate() {
            put(buf, inner_x + 3 + j as u16, row_y, ch, fg, bg);
        }
    }

    // 8. Footer control legend, pinned to the lower margin (range tracks the list size).
    let footer = format!("↑/↓ or 1–{} · ENTER confirm · ESC quit", options.len());
    let fy = area.y + area.height.saturating_sub(2);
    put_str(buf, cx.saturating_sub(footer.chars().count() as u16 / 2), fy, &footer, FOOTER);
}

/// The act picker reached from "RE-ENTER THE MIND": a bordered card listing every
/// unlocked act (locked future acts are simply absent), with a ✓ on completed acts and
/// the cursor parked on the latest. Selecting one re-enters that act's desk.
fn render_act_select(buf: &mut Buffer, area: Rect, menu: &MainMenu, cx: u16, compact: bool) {
    let acts = menu.unlocked_acts();
    let cursor = menu.act_cursor.min(acts.len().saturating_sub(1));

    // Build the row labels once: "[n] ACT … 1854   ✓".
    let rows: Vec<String> = acts
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let done = if menu.acts_completed.contains(a) { "  ✓" } else { "" };
            format!("[{}] {}{}", i + 1, act_title(*a), done)
        })
        .collect();

    if compact {
        let mut y = area.y + area.height / 2;
        y = y.saturating_sub(2 + rows.len() as u16 / 2);
        let title = "SELECT ACT";
        put_str(buf, cx.saturating_sub(title.chars().count() as u16 / 2), y, title, ITEM_HOT);
        y += 2;
        for (i, label) in rows.iter().enumerate() {
            let sel = i == cursor;
            let line = format!("{}{}", if sel { "> " } else { "  " }, label);
            let color = if sel { ITEM_HOT } else { ITEM_DIM };
            put_str(buf, cx.saturating_sub(line.chars().count() as u16 / 2), y + i as u16, &line, color);
        }
        return;
    }

    // Full composition: a heading and a panel that grows with the unlocked-act count.
    let panel_h: u16 = rows.len() as u16 * 2 + 3;
    let cluster_h: u16 = 1 + 2 + panel_h; // heading, gap, panel
    let start_y = area.y + area.height.saturating_sub(cluster_h) / 2;
    let start_y = start_y.max(area.y + 1);

    let heading = "— SELECT THE ACT TO RE-ENTER —";
    put_str(buf, cx.saturating_sub(heading.chars().count() as u16 / 2), start_y, heading, SUBTITLE);

    let panel_w: u16 = 46.min(area.width.saturating_sub(6));
    let panel_x = cx.saturating_sub(panel_w / 2);
    let panel_y = start_y + 3;
    for y in panel_y..panel_y + panel_h {
        for x in panel_x..panel_x + panel_w {
            put(buf, x, y, ' ', PANEL_FILL, PANEL_FILL);
        }
    }
    draw_panel(buf, panel_x, panel_y, panel_w, panel_h);

    let inner_x = panel_x + 1;
    let inner_w = panel_w.saturating_sub(2);
    for (i, label) in rows.iter().enumerate() {
        let row_y = panel_y + 2 + i as u16 * 2;
        let sel = i == cursor;
        let (fg, bg, marker) = if sel {
            (ITEM_HOT, SELECT_BG, '▶')
        } else {
            (ITEM_DIM, bg_at(buf, inner_x, row_y), ' ')
        };
        for x in inner_x..inner_x + inner_w {
            put(buf, x, row_y, ' ', fg, bg);
        }
        put(buf, inner_x + 1, row_y, marker, fg, bg);
        for (j, ch) in label.chars().enumerate() {
            put(buf, inner_x + 3 + j as u16, row_y, ch, fg, bg);
        }
    }

    let footer = format!("↑/↓ or 1–{} · ENTER play · ESC back", rows.len());
    let fy = area.y + area.height.saturating_sub(2);
    put_str(buf, cx.saturating_sub(footer.chars().count() as u16 / 2), fy, &footer, FOOTER);
}

/// Draw a rounded box of the given size with the amber border colour.
fn draw_panel(buf: &mut Buffer, x: u16, y: u16, w: u16, h: u16) {
    if w < 2 || h < 2 {
        return;
    }
    let right = x + w - 1;
    let bottom = y + h - 1;
    for cx in x + 1..right {
        put(buf, cx, y, '─', BORDER, bg_at(buf, cx, y));
        put(buf, cx, bottom, '─', BORDER, bg_at(buf, cx, bottom));
    }
    for cy in y + 1..bottom {
        put(buf, x, cy, '│', BORDER, bg_at(buf, x, cy));
        put(buf, right, cy, '│', BORDER, bg_at(buf, right, cy));
    }
    put(buf, x, y, '╭', BORDER, bg_at(buf, x, y));
    put(buf, right, y, '╮', BORDER, bg_at(buf, right, y));
    put(buf, x, bottom, '╰', BORDER, bg_at(buf, x, bottom));
    put(buf, right, bottom, '╯', BORDER, bg_at(buf, right, bottom));
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn smoke(w: u16, h: u16, save: bool) {
        let sv = SaveState {
            current_act: Act::Boole1854,
            acts_completed: vec![Act::Jacquard1804],
        };
        let menu = MainMenu::new(if save { Some(&sv) } else { None });
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| {
            let size = f.size();
            render_main_menu(f, size, &menu, 0);
        })
        .unwrap();
    }

    #[test]
    fn renders_at_various_sizes() {
        smoke(120, 40, true);
        smoke(80, 24, false);
        smoke(50, 22, true);
        smoke(40, 14, false); // compact fallback
        smoke(20, 8, true); // degenerate — must not panic
    }

    #[test]
    fn cursor_wraps_against_the_active_list_length() {
        // With a save → 4 rows; up from the top wraps to row 3.
        let sv = SaveState { current_act: Act::Babbage1837, acts_completed: vec![] };
        let mut m = MainMenu::new(Some(&sv));
        assert_eq!(m.option_count(), 4);
        assert_eq!(m.selected, 0);
        m.up();
        assert_eq!(m.selected, 3);
        m.down();
        assert_eq!(m.selected, 0);
        for _ in 0..4 {
            m.down();
        }
        assert_eq!(m.selected, 0); // a full loop returns home

        // Fresh → 3 rows; the same walk wraps at 2.
        let mut fresh = MainMenu::new(None);
        assert_eq!(fresh.option_count(), 3);
        fresh.up();
        assert_eq!(fresh.selected, 2);
    }

    #[test]
    fn save_layout_exposes_select_act_and_erase_rows() {
        let sv = SaveState { current_act: Act::Shannon1937, acts_completed: vec![] };
        let with = MainMenu::new(Some(&sv));
        let opts = with.options();
        assert_eq!(opts.len(), 4);
        assert_eq!(opts[0].1, MenuAction::SelectAct); // RE-ENTER opens the act picker
        assert_eq!(opts[1].1, MenuAction::NewGame); // ERASE MEMORY CORE is always present
        assert_eq!(opts[3].1, MenuAction::Exit);

        // Fresh layout: 3 rows, the first is New Game, no SelectAct/Erase split.
        let without = MainMenu::new(None);
        let fopts = without.options();
        assert_eq!(fopts.len(), 3);
        assert_eq!(fopts[0].1, MenuAction::NewGame);
        assert!(fopts.iter().all(|(_, a)| *a != MenuAction::SelectAct));
    }

    #[test]
    fn act_picker_unlocks_exactly_up_to_the_furthest_act() {
        // Reached Act IV (Boole) → acts I–IV unlocked, V/VI absent.
        let sv = SaveState { current_act: Act::Boole1854, acts_completed: vec![Act::Jacquard1804] };
        let mut m = MainMenu::new(Some(&sv));
        assert_eq!(m.unlocked_count(), 4);
        let acts = m.unlocked_acts();
        assert_eq!(acts, vec![Act::Jacquard1804, Act::Babbage1837, Act::Lovelace1843, Act::Boole1854]);
        assert!(!acts.contains(&Act::Shannon1937));
        assert!(!acts.contains(&Act::Turing1936_1950));

        // Opening the picker parks the cursor on the latest act and wraps in-bounds.
        m.open_act_select();
        assert_eq!(m.mode, MenuMode::ActSelect);
        assert_eq!(m.act_cursor, 3);
        assert_eq!(m.selected_act(), Act::Boole1854);
        m.act_down();
        assert_eq!(m.act_cursor, 0); // wrapped past the end
        assert_eq!(m.selected_act(), Act::Jacquard1804);
        m.act_up();
        assert_eq!(m.act_cursor, 3);
    }

    #[test]
    fn act_picker_renders_without_panic() {
        let sv = SaveState { current_act: Act::Turing1936_1950, acts_completed: vec![Act::Jacquard1804] };
        let mut m = MainMenu::new(Some(&sv));
        m.open_act_select();
        for (w, h) in [(120u16, 40u16), (80, 24), (40, 14), (20, 8)] {
            let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
            term.draw(|f| {
                let s = f.size();
                render_main_menu(f, s, &m, 0);
            })
            .unwrap();
        }
    }
}
