use std::collections::{HashMap, HashSet};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    Frame,
};

use super::state::{Act, GlobalStateContext, ScreenState};
use super::mind_log::{DialogueEngine, Speaker, VoiceCue};
use super::audio::AudioEngine;
use super::layout;

// ─────────────────────────────────────────────────────────────────────────────
// TrueColor palette — amber phosphor over a noisy carrier
// ─────────────────────────────────────────────────────────────────────────────
const WS_BG:      Color = Color::Rgb(10, 8, 0);
const BORDER_FG:  Color = Color::Rgb(120, 90, 20);
const HEADER_FG:  Color = Color::Rgb(255, 213, 102);
const TEXT_FG:    Color = Color::Rgb(255, 176, 0);
const DIM_FG:     Color = Color::Rgb(74, 50, 5);
const SECTION_FG: Color = Color::Rgb(160, 140, 100);
const ACCENT:     Color = Color::Rgb(153, 104, 10);
const CURSOR_FG:  Color = Color::Rgb(255, 255, 210);
const OK_FG:      Color = Color::Rgb(255, 213, 102);
const FAIL_FG:    Color = Color::Rgb(255, 102, 51);
const COLLIDE_FG: Color = Color::Rgb(255, 60, 40); // bright red — bit-path collision

// ─────────────────────────────────────────────────────────────────────────────
// Channel constants
// ─────────────────────────────────────────────────────────────────────────────
/// Channel bandwidth W (Hz, abstract) used in Shannon's capacity theorem.
const BANDWIDTH: f32 = 6.0;
/// Signal-to-noise ratio of the carrier. With W = 6 and SNR = 7, capacity
/// C = W·log2(1 + SNR) = 6·log2(8) = 18 bits — a tight but reachable budget.
const SNR: f32 = 7.0;
/// Hard ceiling on a single symbol's code length (keeps the trie + UI bounded).
const MAX_CODE_LEN: usize = 6;

// ═════════════════════════════════════════════════════════════════════════════
// DATA STRUCTURES
// ═════════════════════════════════════════════════════════════════════════════

/// The 2-phase funnel: compress within capacity, then survive channel noise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShannonPhase {
    /// Phase 1 — assign a prefix-free code per symbol, total bits within capacity.
    Compress,
    /// Phase 2 — static noise flips bits; a Hamming parity guard must correct them.
    ErrorCorrect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogKind {
    Info,
    Error,
    Warning,
    Success,
}

/// Which widget the cursor is on. Left/Right shift focus between the symbol list (where
/// 0/1 build codes) and the CHANNEL box (where the PARITY guard is toggled).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Symbols,
    Channel,
}

/// The interactive state of the Act V Shannon information-channel puzzle.
pub struct ShannonPuzzle {
    pub raw_message: &'static str,
    pub symbol_frequencies: HashMap<char, f32>,
    pub bit_allocations: HashMap<char, String>,
    pub channel_noise_level: f32, // SNR
    pub bandwidth: f32,           // W
    pub symbols: Vec<char>,       // unique symbols in first-appearance order
    pub cursor: usize,            // selected symbol row
    pub focus: Focus,             // which widget Left/Right navigation is on
    pub phase: ShannonPhase,
    pub parity_guard: bool,       // Hamming(7,4) error-correction allocated?
    pub status_log: Vec<(String, LogKind)>,
    pub solved: bool,
}

impl ShannonPuzzle {
    pub fn new() -> Self {
        let raw_message = "SILENCE";

        // Unique symbols (first-appearance order) + occurrence counts → probabilities.
        let mut symbols: Vec<char> = Vec::new();
        let mut counts: HashMap<char, usize> = HashMap::new();
        for ch in raw_message.chars() {
            if !counts.contains_key(&ch) {
                symbols.push(ch);
            }
            *counts.entry(ch).or_insert(0) += 1;
        }
        let total = raw_message.chars().count().max(1) as f32;
        let mut symbol_frequencies = HashMap::new();
        let mut bit_allocations = HashMap::new();
        for &s in &symbols {
            symbol_frequencies.insert(s, counts[&s] as f32 / total);
            bit_allocations.insert(s, String::new());
        }

        Self {
            raw_message,
            symbol_frequencies,
            bit_allocations,
            channel_noise_level: SNR,
            bandwidth: BANDWIDTH,
            symbols,
            cursor: 0,
            focus: Focus::Symbols,
            phase: ShannonPhase::Compress,
            parity_guard: false,
            status_log: vec![
                ("Assign a prefix-free binary code per symbol. Keys: 0 1  |  R verify.".to_string(), LogKind::Info),
            ],
            solved: false,
        }
    }

    fn push_log(&mut self, msg: String, kind: LogKind) {
        self.status_log.push((msg, kind));
        if self.status_log.len() > 8 {
            self.status_log.remove(0);
        }
    }

    fn code_of(&self, sym: char) -> &str {
        self.bit_allocations.get(&sym).map(|s| s.as_str()).unwrap_or("")
    }

    /// Shannon entropy H(X) = -Σ P(xᵢ)·log2 P(xᵢ), the theoretical bits-per-symbol
    /// floor. Probabilities are in (0, 1], so the log is always finite (no NaN).
    fn entropy(&self) -> f32 {
        let mut h = 0.0f32;
        for &p in self.symbol_frequencies.values() {
            if p > 0.0 {
                h -= p * p.log2();
            }
        }
        h.max(0.0)
    }

    /// Channel capacity C = W·log2(1 + SNR), floored to whole bits. SNR ≥ 0 keeps the
    /// argument ≥ 1, so the log is non-negative and finite.
    fn capacity_bits(&self) -> u32 {
        let arg = (1.0 + self.channel_noise_level).max(1.0);
        (self.bandwidth * arg.log2()).floor().max(0.0) as u32
    }

    /// Total length of the encoded message under the current allocation (Σ over the
    /// message of each symbol's code length).
    fn total_bits(&self) -> usize {
        self.raw_message
            .chars()
            .map(|c| self.code_of(c).chars().count())
            .sum()
    }

    fn all_assigned(&self) -> bool {
        self.symbols.iter().all(|&s| !self.code_of(s).is_empty())
    }

    /// Build the prefix trie from the live allocation and return it together with the
    /// set of symbols involved in a bit-path collision (duplicate code, or one code a
    /// prefix of another). Rebuilt on demand — cheap for a handful of short codes.
    fn build_trie(&self) -> (Trie, HashSet<char>) {
        let mut root = Trie::default();
        let mut collide: HashSet<char> = HashSet::new();
        for &s in &self.symbols {
            let code = self.code_of(s);
            if !code.is_empty() {
                trie_insert(&mut root, code, s, &mut collide);
            }
        }
        (root, collide)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Prefix trie (the live Huffman/code tree)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct Trie {
    zero: Option<Box<Trie>>,
    one: Option<Box<Trie>>,
    sym: Option<char>,
    collide: bool,
}

/// Insert `sym`'s code into the trie, flagging collisions: descending through a node
/// that already holds a symbol means a shorter code is a prefix of this one; landing
/// on a node that already has a symbol or children is a duplicate/prefix the other way.
fn trie_insert(root: &mut Trie, code: &str, sym: char, collide: &mut HashSet<char>) {
    let mut node = root;
    for bit in code.chars() {
        if let Some(existing) = node.sym {
            node.collide = true;
            collide.insert(existing);
            collide.insert(sym);
        }
        node = match bit {
            '0' => node.zero.get_or_insert_with(|| Box::new(Trie::default())).as_mut(),
            _ => node.one.get_or_insert_with(|| Box::new(Trie::default())).as_mut(),
        };
    }
    if node.sym.is_some() || node.zero.is_some() || node.one.is_some() {
        if let Some(existing) = node.sym {
            collide.insert(existing);
        }
        collide.insert(sym);
        node.collide = true;
    }
    node.sym = Some(sym);
}

/// Flatten the trie into displayable lines: `(text, is_collision)`. Children are
/// emitted in 0-then-1 order with box-drawing connectors.
fn render_trie(node: &Trie, prefix: &str, out: &mut Vec<(String, bool)>) {
    let mut kids: Vec<(char, &Trie)> = Vec::new();
    if let Some(z) = &node.zero {
        kids.push(('0', z));
    }
    if let Some(o) = &node.one {
        kids.push(('1', o));
    }
    let n = kids.len();
    for (i, (bit, child)) in kids.into_iter().enumerate() {
        let last = i + 1 == n;
        let branch = if last { "\u{2514}\u{2500}" } else { "\u{251C}\u{2500}" };
        let label = match child.sym {
            Some(s) => format!("{} [{}]", bit, s),
            None => format!("{} \u{2510}", bit),
        };
        out.push((format!("{}{}{}", prefix, branch, label), child.collide));
        let cont = if last { "   " } else { "\u{2502}  " };
        let child_prefix = format!("{}{}", prefix, cont);
        render_trie(child, &child_prefix, out);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SAFE BUFFER WRITE HELPERS
// ─────────────────────────────────────────────────────────────────────────────

fn in_bounds(buf: &Buffer, x: u16, y: u16) -> bool {
    let Rect { x: ax, y: ay, width: aw, height: ah } = *buf.area();
    x >= ax && x < ax + aw && y >= ay && y < ay + ah
}

fn buf_set(buf: &mut Buffer, x: u16, y: u16, ch: char, style: Style) {
    if in_bounds(buf, x, y) {
        let cell = buf.get_mut(x, y);
        cell.set_char(ch);
        cell.set_style(style);
    }
}

fn buf_set_str(buf: &mut Buffer, x: u16, y: u16, s: &str, style: Style) {
    let mut col = x;
    for ch in s.chars() {
        buf_set(buf, col, y, ch, style);
        col = col.saturating_add(1);
    }
}

fn draw_box(buf: &mut Buffer, rect: Rect, style: Style) {
    if rect.width < 2 || rect.height < 2 {
        return;
    }
    let (x, y, w, h) = (rect.x, rect.y, rect.width, rect.height);
    buf_set(buf, x, y, '┌', style);
    for i in 1..w - 1 {
        buf_set(buf, x + i, y, '─', style);
    }
    buf_set(buf, x + w - 1, y, '┐', style);
    for i in 1..h - 1 {
        buf_set(buf, x, y + i, '│', style);
        buf_set(buf, x + w - 1, y + i, '│', style);
    }
    buf_set(buf, x, y + h - 1, '└', style);
    for i in 1..w - 1 {
        buf_set(buf, x + i, y + h - 1, '─', style);
    }
    buf_set(buf, x + w - 1, y + h - 1, '┘', style);
}

fn clip(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

// ═════════════════════════════════════════════════════════════════════════════
// RENDER — THE INFORMATION CHANNEL (ACT V)
// ═════════════════════════════════════════════════════════════════════════════

/// Draw the Shannon workspace: a symbol/probability/code table (left), a live binary
/// code-tree (centre), and a channel-health card (right), under a status gauge.
pub fn render_shannon(
    f: &mut Frame,
    area: Rect,
    state: &mut GlobalStateContext,
    puzzle: &ShannonPuzzle,
) {
    let buf = f.buffer_mut();
    layout::draw_workspace_grid(buf, area, WS_BG);

    if state.screen_state == ScreenState::AmbientDesk {
        layout::draw_gauge_footer(buf, area, "", DIM_FG);
        return;
    }

    let (x0, y0, w, h) = (area.x, area.y, area.width, area.height);
    if w < 24 || h < 12 {
        return;
    }

    draw_box(buf, area, Style::default().fg(BORDER_FG).bg(WS_BG));
    let phase_name = match puzzle.phase {
        ShannonPhase::Compress => "PHASE 1: COMPRESS",
        ShannonPhase::ErrorCorrect => "PHASE 2: ERROR-CORRECT",
    };
    let header = clip(
        &format!(" INFORMATION CHANNEL \u{2500}\u{2500} {} (1937) ", phase_name),
        w.saturating_sub(4) as usize,
    );
    buf_set_str(buf, x0 + 2, y0, &header, Style::default().fg(HEADER_FG).bg(WS_BG));

    let (trie, collide) = puzzle.build_trie();

    // ── Sub-column geometry inside the workspace. ──
    let inner_x = x0 + 1;
    let inner_w = w.saturating_sub(2);
    let left_w: u16 = inner_w.saturating_mul(40) / 100;
    let left_w = left_w.clamp(12, 20);
    let right_w: u16 = 14;
    let body_top = y0 + 2;
    let body_bottom = y0 + h.saturating_sub(2);

    draw_symbol_table(buf, inner_x + 1, body_top, body_bottom, left_w, puzzle, &collide);

    let center_x = inner_x + left_w + 1;
    let right_x = inner_x + inner_w.saturating_sub(right_w);
    let center_w = right_x.saturating_sub(center_x + 1);
    if center_w >= 8 {
        draw_code_tree(buf, center_x, body_top, body_bottom, center_w, &trie);
    }
    if right_w >= 10 {
        draw_channel_card(buf, right_x, body_top, right_w, puzzle, &collide);
    }

    if let Some((msg, kind)) = puzzle.status_log.last() {
        let col = match kind {
            LogKind::Info => Color::Rgb(170, 150, 110),
            LogKind::Error => FAIL_FG,
            LogKind::Warning => TEXT_FG,
            LogKind::Success => OK_FG,
        };
        layout::draw_gauge_footer(buf, area, msg, col);
    }
}

fn draw_symbol_table(
    buf: &mut Buffer,
    x: u16,
    top: u16,
    bottom: u16,
    width: u16,
    puzzle: &ShannonPuzzle,
    collide: &HashSet<char>,
) {
    buf_set_str(buf, x, top, "SYM  P    CODE", Style::default().fg(SECTION_FG).bg(WS_BG));
    let mut ry = top + 1;
    for (i, &s) in puzzle.symbols.iter().enumerate() {
        if ry >= bottom {
            break;
        }
        let selected = i == puzzle.cursor;
        let p = puzzle.symbol_frequencies.get(&s).copied().unwrap_or(0.0);
        let code = puzzle.code_of(s);
        let shown_code = if selected {
            format!("{}\u{2588}", code) // inline edit cursor on the active symbol
        } else if code.is_empty() {
            "\u{2039}\u{2014}\u{203A}".to_string() // ‹—›
        } else {
            code.to_string()
        };
        let marker = if selected { "\u{25B8}" } else { " " };
        let line = clip(&format!("{}{}  {:.2} {}", marker, s, p, shown_code), width as usize);
        let color = if collide.contains(&s) {
            COLLIDE_FG
        } else if selected {
            CURSOR_FG
        } else {
            TEXT_FG
        };
        buf_set_str(buf, x, ry, &line, Style::default().fg(color).bg(WS_BG));
        ry += 1;
    }
}

fn draw_code_tree(buf: &mut Buffer, x: u16, top: u16, bottom: u16, width: u16, trie: &Trie) {
    buf_set_str(buf, x, top, "CODE TREE", Style::default().fg(SECTION_FG).bg(WS_BG));
    let has_any = trie.zero.is_some() || trie.one.is_some();
    if !has_any {
        buf_set_str(buf, x, top + 2, &clip("(no codes yet)", width as usize), Style::default().fg(DIM_FG).bg(WS_BG));
        return;
    }
    let mut lines: Vec<(String, bool)> = Vec::new();
    lines.push(("\u{25CF} root".to_string(), false));
    render_trie(trie, "", &mut lines);

    let mut ry = top + 1;
    for (text, is_collision) in lines {
        if ry >= bottom {
            break;
        }
        let color = if is_collision { COLLIDE_FG } else { ACCENT };
        buf_set_str(buf, x, ry, &clip(&text, width as usize), Style::default().fg(color).bg(WS_BG));
        ry += 1;
    }
}

fn draw_channel_card(
    buf: &mut Buffer,
    x: u16,
    top: u16,
    width: u16,
    puzzle: &ShannonPuzzle,
    collide: &HashSet<char>,
) {
    let h: u16 = 8;
    let rect = Rect::new(x, top, width, h);
    draw_box(buf, rect, Style::default().fg(BORDER_FG).bg(WS_BG));
    let focused = puzzle.focus == Focus::Channel;
    let title = clip(" CHANNEL ", width.saturating_sub(2) as usize);
    let title_col = if focused { CURSOR_FG } else { HEADER_FG };
    buf_set_str(buf, x + 1, top, &title, Style::default().fg(title_col).bg(WS_BG));

    let total = puzzle.total_bits();
    let cap = puzzle.capacity_bits();
    let saturated = total as u32 > cap;
    let entropy = puzzle.entropy();
    let inner = width.saturating_sub(2) as usize;

    let status = if puzzle.solved {
        ("VERIFIED", OK_FG)
    } else if !collide.is_empty() {
        ("COLLISION", COLLIDE_FG)
    } else if saturated {
        ("SATURATED", FAIL_FG)
    } else if puzzle.phase == ShannonPhase::ErrorCorrect {
        if puzzle.parity_guard {
            ("GUARDED", OK_FG)
        } else {
            ("NOISY", FAIL_FG)
        }
    } else {
        ("OK", OK_FG)
    };

    let budget_col = if saturated { FAIL_FG } else { TEXT_FG };
    let rows: [(String, Color); 5] = [
        (format!("BUDGET {}/{}", total, cap), budget_col),
        (format!("SNR    {:.1}", puzzle.channel_noise_level), TEXT_FG),
        (format!("H(X)   {:.2}", entropy), TEXT_FG),
        (format!("PARITY {}", if puzzle.parity_guard { "ON" } else { "off" }), if puzzle.parity_guard { OK_FG } else { DIM_FG }),
        (format!("STATUS {}", status.0), status.1),
    ];
    // The PARITY row (index 3) is the CHANNEL box's one interactive control — when the
    // box is focused, mark it with a ▸ cursor and brighten it so the toggle is obvious.
    const PARITY_ROW: usize = 3;
    let mut ry = top + 1;
    for (i, (text, color)) in rows.into_iter().enumerate() {
        let (line, col) = if focused && i == PARITY_ROW {
            (format!("\u{25B8} {}", text), CURSOR_FG)
        } else {
            (text, color)
        };
        buf_set_str(buf, x + 1, ry, &clip(&line, inner), Style::default().fg(col).bg(WS_BG));
        ry += 1;
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// EVENT HANDLER — KEYBOARD INPUT
// ═════════════════════════════════════════════════════════════════════════════

/// Route a key against the Shannon board. Up/Down select a symbol; `0`/`1` append to
/// its code; Backspace trims; `P` toggles the parity guard (Phase 2); `R` verifies.
pub fn handle_input(
    key: KeyEvent,
    puzzle: &mut ShannonPuzzle,
    state: &mut GlobalStateContext,
    dialogue: &mut DialogueEngine,
    audio: &mut AudioEngine,
) {
    if puzzle.solved {
        return;
    }

    match key.code {
        // ── Horizontal focus shift between the symbol list and the CHANNEL box, so the
        //    PARITY toggle is reachable with standard TUI navigation. ──
        KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') => {
            puzzle.focus = Focus::Symbols;
            audio.grid_nav();
        }
        KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') => {
            puzzle.focus = Focus::Channel;
            audio.grid_nav();
        }
        // Vertical: move the symbol cursor when the list is focused; toggle the parity
        // guard when the CHANNEL box is focused.
        KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => match puzzle.focus {
            Focus::Symbols => {
                if puzzle.cursor > 0 {
                    puzzle.cursor -= 1;
                }
                audio.grid_nav();
            }
            Focus::Channel => toggle_parity(puzzle, audio),
        },
        KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => match puzzle.focus {
            Focus::Symbols => {
                if puzzle.cursor + 1 < puzzle.symbols.len() {
                    puzzle.cursor += 1;
                }
                audio.grid_nav();
            }
            Focus::Channel => toggle_parity(puzzle, audio),
        },
        // Enter/Space activates the focused widget's control (the CHANNEL parity toggle).
        KeyCode::Enter | KeyCode::Char(' ') => {
            if puzzle.focus == Focus::Channel {
                toggle_parity(puzzle, audio);
            }
        }
        KeyCode::Char('0') | KeyCode::Char('1') => {
            puzzle.focus = Focus::Symbols; // typing a bit implies the symbol list
            if let Some(&sym) = puzzle.symbols.get(puzzle.cursor) {
                let code = puzzle.bit_allocations.entry(sym).or_default();
                if code.chars().count() < MAX_CODE_LEN {
                    code.push(if matches!(key.code, KeyCode::Char('1')) { '1' } else { '0' });
                    audio.daktilo_fast(); // high-frequency click per bit
                }
            }
        }
        KeyCode::Backspace => {
            if let Some(&sym) = puzzle.symbols.get(puzzle.cursor) {
                if let Some(code) = puzzle.bit_allocations.get_mut(&sym) {
                    if code.pop().is_some() {
                        audio.backspace_snap();
                    }
                }
            }
        }
        // Direct parity hotkey — works at any time during Act V, regardless of focus or
        // phase, so the player can always flip the guard without hunting for the widget.
        KeyCode::Char('p') | KeyCode::Char('P') => toggle_parity(puzzle, audio),
        // Hidden developer shortcut (demo-recording safeguard): F10 force-allocates the
        // parity guard and runs Phase 2 verification, cleanly driving the successful
        // transition into the Act Outro. Not surfaced in the on-screen controls.
        KeyCode::F(10) => {
            puzzle.phase = ShannonPhase::ErrorCorrect;
            puzzle.parity_guard = true;
            verify(puzzle, state, dialogue, audio);
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            verify(puzzle, state, dialogue, audio);
        }
        _ => {}
    }
}

/// Flip the Hamming(7,4) parity guard, log the change, and fire the heavy confirm latch.
fn toggle_parity(puzzle: &mut ShannonPuzzle, audio: &mut AudioEngine) {
    puzzle.parity_guard = !puzzle.parity_guard;
    let msg = if puzzle.parity_guard {
        "Hamming(7,4) parity guard allocated \u{2014} single-bit flips correctable.".to_string()
    } else {
        "Parity guard removed \u{2014} the channel is exposed to noise.".to_string()
    };
    puzzle.push_log(msg, LogKind::Info);
    audio.menu_confirm();
}

/// Verify the current phase. Phase 1 enforces the entropy/capacity ceiling and prefix
/// freedom; Phase 2 enforces the parity guard against channel noise. A saturation or
/// noise failure detonates the electrical-short glitch.
fn verify(
    puzzle: &mut ShannonPuzzle,
    state: &mut GlobalStateContext,
    dialogue: &mut DialogueEngine,
    audio: &mut AudioEngine,
) {
    match puzzle.phase {
        ShannonPhase::Compress => {
            if !puzzle.all_assigned() {
                puzzle.push_log("Every symbol needs a binary code before transmission.".to_string(), LogKind::Warning);
                return;
            }
            let (_, collide) = puzzle.build_trie();
            if !collide.is_empty() {
                puzzle.push_log(
                    "[ERR_PREFIX_COLLISION] duplicate bit-paths \u{2014} the decoder cannot split the stream.".to_string(),
                    LogKind::Error,
                );
                audio.glitch();
                return;
            }
            let total = puzzle.total_bits() as u32;
            let cap = puzzle.capacity_bits();
            if total > cap {
                puzzle.push_log(
                    format!("[ERR_CHANNEL_SATURATION] {} bits exceeds capacity C = {} bits.", total, cap),
                    LogKind::Error,
                );
                audio.glitch();
                return;
            }
            puzzle.phase = ShannonPhase::ErrorCorrect;
            puzzle.push_log(
                format!("Within capacity ({}/{} bits). PHASE 2: noise rising \u{2014} allocate parity (P).", total, cap),
                LogKind::Success,
            );
        }
        ShannonPhase::ErrorCorrect => {
            if !puzzle.parity_guard {
                puzzle.push_log(
                    "[NOISE] >1 bit flipped without a guard \u{2014} message decodes as gibberish.".to_string(),
                    LogKind::Error,
                );
                audio.glitch();
                return;
            }
            puzzle.solved = true;
            puzzle.push_log(
                "TRANSMISSION VERIFIED. Parity corrected the flips; the memory arrives intact.".to_string(),
                LogKind::Success,
            );
            if !state.acts_completed.contains(&Act::Shannon1937) {
                state.acts_completed.push(Act::Shannon1937);
            }
            dialogue.play(
                Speaker::Turing,
                "[TURING]: \"A message is just improbability, shaped. Compress it, guard it against the noise \u{2014} and even a fading mind can be sent across the wire, whole.\"",
                Some(VoiceCue::Victory(Act::Shannon1937)),
            );
            state.current_act = Act::Turing1936_1950;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_follows_shannon_theorem() {
        let p = ShannonPuzzle::new();
        // C = 6 * log2(1 + 7) = 6 * 3 = 18.
        assert_eq!(p.capacity_bits(), 18);
    }

    #[test]
    fn focus_starts_on_the_symbol_list() {
        // Left/Right shift this between the symbol list and the CHANNEL box so the
        // PARITY toggle is reachable; the guard starts off.
        let p = ShannonPuzzle::new();
        assert_eq!(p.focus, Focus::Symbols);
        assert!(!p.parity_guard);
    }

    #[test]
    fn entropy_is_positive_and_finite() {
        let p = ShannonPuzzle::new();
        let h = p.entropy();
        assert!(h.is_finite());
        assert!(h > 2.0 && h < 3.0); // ~2.52 bits/symbol for SILENCE
    }

    #[test]
    fn prefix_collision_is_detected() {
        let mut p = ShannonPuzzle::new();
        // '0' is a prefix of '01' → collision between the two symbols.
        p.bit_allocations.insert('S', "0".to_string());
        p.bit_allocations.insert('I', "01".to_string());
        let (_, collide) = p.build_trie();
        assert!(collide.contains(&'S') && collide.contains(&'I'));
    }

    #[test]
    fn a_prefix_free_within_capacity_code_compresses() {
        let mut p = ShannonPuzzle::new();
        let codes = [('S', "01"), ('I', "100"), ('L', "101"), ('E', "00"), ('N', "110"), ('C', "111")];
        for (s, c) in codes {
            p.bit_allocations.insert(s, c.to_string());
        }
        let (_, collide) = p.build_trie();
        assert!(collide.is_empty(), "code should be prefix-free");
        assert!(p.all_assigned());
        assert!(p.total_bits() as u32 <= p.capacity_bits(), "must fit within capacity");
    }
}

