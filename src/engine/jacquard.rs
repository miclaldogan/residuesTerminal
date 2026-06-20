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
// TrueColor Palette Constants — workspace amber/parchment theme
// ─────────────────────────────────────────────────────────────────────────────
const WS_BG:           Color = Color::Rgb(10, 8, 0);       // deep amber-black
const HEADER_FG:       Color = Color::Rgb(255, 213, 102);   // bright amber header
const TEXT_FG:         Color = Color::Rgb(255, 176, 0);     // standard amber
const DIM_FG:          Color = Color::Rgb(74, 50, 5);       // amber ghost/dim
const CELL_ON:         Color = Color::Rgb(255, 176, 0);     // amber cell active
const CELL_OFF:        Color = Color::Rgb(42, 30, 2);       // very dark amber cell
const TARGET_ON:       Color = Color::Rgb(153, 104, 10);    // target amber dim
const TARGET_OFF:      Color = Color::Rgb(34, 24, 2);       // target amber ghost
const MATCH_OK:        Color = Color::Rgb(255, 213, 102);   // amber bright (success)
const MATCH_FAIL:      Color = Color::Rgb(255, 102, 51);    // amber-orange (fail)
const ERROR_FG:        Color = Color::Rgb(255, 102, 51);    // amber-orange error
const WARN_FG:         Color = Color::Rgb(255, 176, 0);     // amber warning
const WEAVE_SCANNER:   Color = Color::Rgb(255, 213, 102);   // amber bright scanner
const PHASE1_ACCENT:   Color = Color::Rgb(255, 176, 0);     // amber
const PHASE2_ACCENT:   Color = Color::Rgb(153, 104, 10);    // dim amber
const PHASE3_ACCENT:   Color = Color::Rgb(255, 213, 102);   // bright amber

// ─────────────────────────────────────────────────────────────────────────────
// Simulation Constants
// ─────────────────────────────────────────────────────────────────────────────
/// Total rows the weave simulation must verify (2 full pattern repeats).
const SIMULATION_ROWS: usize = 8;
/// Frame ticks between each weave step (~133ms at 60 FPS).
const WEAVE_PACE: u16 = 8;
/// Maximum continuous roll rows before mechanical tearing (Phase 1).
const ROLL_TEAR_LIMIT: usize = 6;
/// Maximum discrete chain blocks before mass displacement failure (Phase 2).
const CHAIN_MASS_LIMIT: usize = 6;

// ═════════════════════════════════════════════════════════════════════════════
// DATA STRUCTURES
// ═════════════════════════════════════════════════════════════════════════════

/// Historical mechanism progression states — the 3-state constraint funnel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JacquardPhase {
    BouchonRoll,  // Phase 1: Continuous paper roll (Bouchon, 1725)
    FalconChain,  // Phase 2: Rigid block-chain vector (Falcon, 1728)
    PunchedCard,  // Phase 3: Optimized modular cards (Jacquard, 1804)
}

/// Severity classification for workspace status log entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogKind {
    Info,
    Error,
    Warning,
    Success,
}

/// The complete interactive state for the Act I Jacquard loom puzzle.
pub struct JacquardPuzzle {
    pub cards: Vec<[u8; 8]>,
    pub target: [[u8; 8]; 4],
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub tension: f32,
    pub weave_row: usize,
    pub weaving: bool,
    pub snapped: bool,
    pub jammed: bool,
    pub phase: JacquardPhase,
    pub status_log: Vec<(String, LogKind)>,
    pub solved: bool,
    pub compression_flagged: bool,
    pub weave_tick_acc: u16,
    pub weave_match_map: Vec<bool>,
    /// Consecutive failed verification runs — drives the cynical blunder taunt.
    pub consecutive_fails: usize,
}

impl JacquardPuzzle {
    pub fn new() -> Self {
        Self {
            cards: vec![[0u8; 8]],
            target: [
                [1, 0, 1, 0, 1, 0, 1, 0], // 0xAA
                [0, 1, 0, 1, 0, 1, 0, 1], // 0x55
                [1, 0, 1, 0, 1, 0, 1, 0], // 0xAA
                [0, 1, 0, 1, 0, 1, 0, 1], // 0x55
            ],
            cursor_row: 0,
            cursor_col: 0,
            tension: 0.0,
            weave_row: 0,
            weaving: false,
            snapped: false,
            jammed: false,
            phase: JacquardPhase::BouchonRoll,
            status_log: vec![
                ("Feed the continuous paper roll into the loom...".into(), LogKind::Info),
            ],
            solved: false,
            compression_flagged: false,
            weave_tick_acc: 0,
            weave_match_map: Vec::new(),
            consecutive_fails: 0,
        }
    }

    /// Push a bounded status message onto the workspace log (max 8 entries).
    fn push_log(&mut self, msg: String, kind: LogKind) {
        self.status_log.push((msg, kind));
        if self.status_log.len() > 8 {
            self.status_log.remove(0);
        }
    }

    /// Reset all working state and transition to the specified phase.
    fn reset_for_phase(&mut self, phase: JacquardPhase) {
        self.phase = phase;
        self.cards = vec![[0u8; 8]];
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.tension = 0.0;
        self.weave_row = 0;
        self.weaving = false;
        self.snapped = false;
        self.jammed = false;
        self.weave_match_map.clear();
        self.compression_flagged = false;

        match phase {
            JacquardPhase::BouchonRoll => {
                self.push_log("Feed the continuous paper roll...".into(), LogKind::Info);
            }
            JacquardPhase::FalconChain => {
                self.push_log(
                    "Roll mechanism abandoned. Linking discrete Falcon chain blocks...".into(),
                    LogKind::Info,
                );
            }
            JacquardPhase::PunchedCard => {
                self.push_log(
                    "Chain mechanism abandoned. Modular punched cards loaded.".into(),
                    LogKind::Info,
                );
                self.push_log(
                    "Cards loop: instruction[row % cards.len()]. Find the period.".into(),
                    LogKind::Info,
                );
            }
        }
    }

    /// Convert an 8-bit card row into its hexadecimal byte value (MSB-first).
    fn row_to_hex(row: &[u8; 8]) -> u8 {
        let mut val = 0u8;
        for (i, &bit) in row.iter().enumerate() {
            if bit != 0 {
                val |= 1 << (7 - i);
            }
        }
        val
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SAFE BUFFER WRITE HELPERS
// ─────────────────────────────────────────────────────────────────────────────

fn buf_set(buf: &mut Buffer, x: u16, y: u16, ch: char, style: Style) {
    let Rect { x: ax, y: ay, width: aw, height: ah } = *buf.area();
    if x >= ax && x < ax + aw && y >= ay && y < ay + ah {
        let cell = buf.get_mut(x, y);
        cell.set_char(ch);
        cell.set_style(style);
    }
}

fn buf_set_str(buf: &mut Buffer, x: u16, y: u16, s: &str, style: Style) {
    let Rect { x: ax, y: ay, width: aw, height: ah } = *buf.area();
    let mut col = x;
    for ch in s.chars() {
        if col >= ax + aw { break; }
        if col >= ax && y >= ay && y < ay + ah {
            let cell = buf.get_mut(col, y);
            cell.set_char(ch);
            cell.set_style(style);
        }
        col += 1;
    }
}

fn buf_fill_bg(buf: &mut Buffer, rect: Rect, bg: Color) {
    let Rect { x: ax, y: ay, width: aw, height: ah } = *buf.area();
    for row in rect.y..rect.y.saturating_add(rect.height) {
        for col in rect.x..rect.x.saturating_add(rect.width) {
            if col >= ax && col < ax + aw && row >= ay && row < ay + ah {
                buf.get_mut(col, row).set_style(Style::default().bg(bg));
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// BOX & OVERLAY DRAWING PRIMITIVES
// ─────────────────────────────────────────────────────────────────────────────

fn draw_box(buf: &mut Buffer, rect: Rect, style: Style) {
    if rect.width < 2 || rect.height < 2 { return; }
    let x = rect.x;
    let y = rect.y;
    let w = rect.width;
    let h = rect.height;

    buf_set(buf, x, y, '┌', style);
    for i in 1..w - 1 { buf_set(buf, x + i, y, '─', style); }
    buf_set(buf, x + w - 1, y, '┐', style);

    for i in 1..h - 1 {
        buf_set(buf, x, y + i, '│', style);
        buf_set(buf, x + w - 1, y + i, '│', style);
    }

    buf_set(buf, x, y + h - 1, '└', style);
    for i in 1..w - 1 { buf_set(buf, x + i, y + h - 1, '─', style); }
    buf_set(buf, x + w - 1, y + h - 1, '┘', style);
}

fn render_error_overlay(buf: &mut Buffer, area: Rect, code: &str, line1: &str, line2: &str) {
    let err_bg = Color::Rgb(20, 12, 0);
    let code_style = Style::default().fg(ERROR_FG).bg(err_bg);
    let text_style = Style::default().fg(Color::Rgb(200, 100, 20)).bg(err_bg);
    let hint_style = Style::default().fg(DIM_FG).bg(err_bg);

    let overlay_h: u16 = 9;
    let overlay_w = area.width.saturating_sub(4);
    let ox = area.x + 2;
    let oy = area.y + (area.height / 2).saturating_sub(4);

    let overlay_rect = Rect::new(ox, oy, overlay_w, overlay_h);
    buf_fill_bg(buf, overlay_rect, err_bg);
    draw_box(buf, overlay_rect, Style::default().fg(Color::Rgb(120, 70, 10)).bg(err_bg));

    let cx = ox + overlay_w / 2;
    let code_half = code.len() as u16 / 2;
    buf_set_str(buf, cx.saturating_sub(code_half), oy + 2, code, code_style);
    let l1_half = line1.len() as u16 / 2;
    buf_set_str(buf, cx.saturating_sub(l1_half), oy + 4, line1, text_style);
    let l2_half = line2.len() as u16 / 2;
    buf_set_str(buf, cx.saturating_sub(l2_half), oy + 5, line2, text_style);
    let hint = "[ press any key ]";
    buf_set_str(buf, cx.saturating_sub(hint.len() as u16 / 2), oy + 7, hint, hint_style);
}

fn render_victory_overlay(buf: &mut Buffer, area: Rect) {
    let vic_bg = Color::Rgb(15, 12, 0);
    let gold = Style::default().fg(Color::Rgb(255, 213, 102)).bg(vic_bg);
    let sub = Style::default().fg(Color::Rgb(255, 176, 0)).bg(vic_bg);
    let dim = Style::default().fg(DIM_FG).bg(vic_bg);

    let overlay_h: u16 = 9;
    let overlay_w = area.width.saturating_sub(4);
    let ox = area.x + 2;
    let oy = area.y + (area.height / 2).saturating_sub(4);

    let overlay_rect = Rect::new(ox, oy, overlay_w, overlay_h);
    buf_fill_bg(buf, overlay_rect, vic_bg);
    draw_box(buf, overlay_rect, Style::default().fg(Color::Rgb(92, 68, 0)).bg(vic_bg));

    let cx = ox + overlay_w / 2;
    let title = "THE LOOM IS COMPLETE";
    buf_set_str(buf, cx.saturating_sub(title.len() as u16 / 2), oy + 2, title, gold);
    let s1 = "Instruction separated from mechanism.";
    buf_set_str(buf, cx.saturating_sub(s1.len() as u16 / 2), oy + 4, s1, sub);
    let s2 = "The pattern lives on card.";
    buf_set_str(buf, cx.saturating_sub(s2.len() as u16 / 2), oy + 5, s2, sub);
    let s3 = "Advancing to Act II: Babbage...";
    buf_set_str(buf, cx.saturating_sub(s3.len() as u16 / 2), oy + 7, s3, dim);
}

// ═════════════════════════════════════════════════════════════════════════════
// RENDER — THE WORKSPACE (ACT I)
// ═════════════════════════════════════════════════════════════════════════════

/// Draw the interactive Jacquard workspace into the left panel.
/// Renders the bit-matrix, target weave, cursor, weave scanner, and status log.
pub fn render_workspace(
    f: &mut Frame,
    area: Rect,
    state: &mut GlobalStateContext,
    puzzle: &JacquardPuzzle,
) {
    let buf = f.buffer_mut();
    // Blueprint dot-matrix sheet instead of a flat black fill; puzzle elements draw
    // on top of it and only overwrite the cells they occupy.
    layout::draw_workspace_grid(buf, area, WS_BG);

    // ── RENDER GUARD: in the ambient phase the loom is invisible, uncalculated and
    //    hidden under the dormant grid. Draw ONLY the grid (above) and the gauge
    //    baseline, then return before any card / bit / target geometry is touched. ──
    if state.screen_state == ScreenState::AmbientDesk {
        layout::draw_gauge_footer(buf, area, "", DIM_FG);
        return;
    }

    let x0 = area.x;
    let y0 = area.y;
    let w = area.width;
    let h = area.height;

    if w < 20 || h < 10 { return; }

    // ── Tension-modulated border color ──
    let tension_r = (140.0 + puzzle.tension * 115.0).min(255.0) as u8;
    let tension_g = (120.0 * (1.0 - puzzle.tension * 0.6)).max(0.0) as u8;
    let tension_b = (90.0 * (1.0 - puzzle.tension * 0.6)).max(0.0) as u8;
    let border_style = Style::default().fg(Color::Rgb(tension_r, tension_g, tension_b)).bg(WS_BG);
    draw_box(buf, area, border_style);

    // ── Phase header inside top border ──
    let (phase_name, phase_year, phase_color) = match puzzle.phase {
        JacquardPhase::BouchonRoll => ("BOUCHON PAPER ROLL", "1725", PHASE1_ACCENT),
        JacquardPhase::FalconChain => ("FALCON CHAIN BLOCKS", "1728", PHASE2_ACCENT),
        JacquardPhase::PunchedCard => ("JACQUARD PUNCHED CARDS", "1804", PHASE3_ACCENT),
    };
    let header_text = format!(" THE LOOM \u{2500}\u{2500} {} ({}) ", phase_name, phase_year);
    let phase_style = Style::default().fg(phase_color).bg(WS_BG);
    buf_set_str(buf, x0 + 2, y0, &header_text, phase_style);

    // ── Error / Victory overlays ──
    if puzzle.snapped {
        render_error_overlay(
            buf, area,
            "[ERR_STR_TEAR]",
            "Continuous paper roll torn under",
            "mechanical tension.",
        );
        return;
    }
    if puzzle.jammed {
        render_error_overlay(
            buf, area,
            "[MECHANISM_JAMMED]",
            "Volumetric block-chain exceeds mass",
            "displacement boundaries.",
        );
        return;
    }
    if puzzle.solved {
        render_victory_overlay(buf, area);
        return;
    }

    // ── Cursor pulse derived from global frame counter ──
    let pulse_val = ((state.frame_counter as f64 * 0.15).sin() * 30.0) as i16;
    let cursor_r = (255i16 + pulse_val).clamp(210, 255) as u8;
    let cursor_g = (255i16 + pulse_val).clamp(210, 255) as u8;
    let cursor_b = (220i16 + pulse_val).clamp(180, 255) as u8;
    let cursor_style = Style::default().fg(Color::Rgb(cursor_r, cursor_g, cursor_b)).bg(WS_BG);

    let text_style = Style::default().fg(TEXT_FG).bg(WS_BG);
    let dim_style  = Style::default().fg(DIM_FG).bg(WS_BG);
    let section_style = Style::default().fg(Color::Rgb(160, 140, 100)).bg(WS_BG);

    let inner_x = x0 + 2;
    let mut cy = y0 + 1;

    // ── Section: INSTRUCTION CARDS ──
    buf_set_str(buf, inner_x, cy, "INSTRUCTION CARDS", section_style);
    cy += 1;

    let clamped_cr = puzzle.cursor_row.min(puzzle.cards.len().saturating_sub(1));
    let clamped_cc = puzzle.cursor_col.min(7);

    // Compute scanner position for weave animation
    let scanner_card_idx = if puzzle.weaving && !puzzle.cards.is_empty() {
        Some(match puzzle.phase {
            JacquardPhase::PunchedCard => puzzle.weave_row % puzzle.cards.len(),
            _ => puzzle.weave_row,
        })
    } else {
        None
    };

    let cards_top = cy;
    for (i, card) in puzzle.cards.iter().enumerate() {
        if cy >= y0 + h.saturating_sub(9) { break; }

        // ── Overweight shudder: when the loom is too heavy to cycle the rows
        //    judder violently sideways, scaled by the global vision_blur_factor. ──
        let shudder: i16 = if state.vision_blur_factor > 0.05 {
            let amp = state.vision_blur_factor * 4.5;
            (((state.frame_counter as f64 * 0.9 + i as f64 * 1.7).sin()) as f32 * amp).round() as i16
        } else {
            0
        };
        let shift = |base: u16| -> u16 { (base as i16 + shudder).max(x0 as i16 + 1) as u16 };

        // Weave execution tracker: ►
        if let Some(si) = scanner_card_idx {
            if i == si {
                let scan_style = Style::default().fg(WEAVE_SCANNER).bg(WS_BG);
                buf_set(buf, shift(inner_x), cy, '\u{25BA}', scan_style);
            }
        }

        // Card index label
        let label = format!("{}:", i);
        buf_set_str(buf, shift(inner_x + 2), cy, &label, dim_style);

        let cell_x0 = shift(inner_x + 2 + label.len() as u16 + 1);

        // Draw 8 cell triplets
        for (j, &bit) in card.iter().enumerate() {
            let cx = cell_x0 + (j as u16 * 4);
            let is_cursor = i == clamped_cr && j == clamped_cc && !puzzle.weaving;

            let (glyph, style) = if is_cursor {
                if bit != 0 { ("\u{2588}\u{2588}\u{2588}", cursor_style) }
                else        { ("\u{2592}\u{2592}\u{2592}", cursor_style) }
            } else if bit != 0 {
                ("\u{2593}\u{2593}\u{2593}", Style::default().fg(CELL_ON).bg(WS_BG))
            } else {
                ("\u{2591}\u{2591}\u{2591}", Style::default().fg(CELL_OFF).bg(WS_BG))
            };
            buf_set_str(buf, cx, cy, glyph, style);
        }

        // Per-row match status (live evaluation) and hex conversion
        let hex_val = JacquardPuzzle::row_to_hex(card);
        let target_row = &puzzle.target[i % 4];
        let matches = *card == *target_row;

        let status_x = cell_x0 + 32;
        if matches {
            buf_set(buf, status_x, cy, '\u{2713}', Style::default().fg(MATCH_OK).bg(WS_BG));
        } else {
            buf_set(buf, status_x, cy, '\u{2717}', Style::default().fg(MATCH_FAIL).bg(WS_BG));
        }
        let hex_str = format!("0x{:02X}", hex_val);
        buf_set_str(buf, status_x + 2, cy, &hex_str, dim_style);

        cy += 1;
    }

    // Z-depth: drop a shadow under the instruction-card matrix block.
    let matrix_h = cy.saturating_sub(cards_top);
    if matrix_h > 0 {
        let mw = 32u16.min(w.saturating_sub(8));
        layout::draw_drop_shadow(buf, Rect::new(inner_x + 5, cards_top, mw, matrix_h));
    }

    // ── Loop indicator (Phase 3 only) ──
    if puzzle.phase == JacquardPhase::PunchedCard && !puzzle.cards.is_empty() {
        let loop_text = format!("\u{21BB} cards[row % {}]", puzzle.cards.len());
        buf_set_str(buf, inner_x + 2, cy, &loop_text, text_style);
        cy += 1;
    }

    // ── Overweight indicator: no numbers, just the strain. ──
    if puzzle.compression_flagged {
        let warn = "\u{2592}\u{2592} TOO HEAVY \u{2014} the carriage cannot cycle this many cards \u{2592}\u{2592}";
        buf_set_str(buf, inner_x + 2, cy, warn, Style::default().fg(ERROR_FG).bg(WS_BG));
        cy += 1;
    }

    // ── Simulation progress ──
    if !puzzle.weave_match_map.is_empty() {
        let matched = puzzle.weave_match_map.iter().filter(|&&m| m).count();
        let total = puzzle.weave_match_map.len();
        let sim_text = format!("Sim: {}/{} rows matched", matched, total);
        let sim_color = if matched == total { MATCH_OK } else { MATCH_FAIL };
        buf_set_str(buf, inner_x + 2, cy, &sim_text, Style::default().fg(sim_color).bg(WS_BG));
        cy += 1;
    }

    cy += 1;

    // ── Section: TARGET DAMASK WEAVE ──
    buf_set_str(buf, inner_x, cy, "TARGET DAMASK WEAVE", section_style);
    cy += 1;

    let target_style = Style::default().fg(HEADER_FG).bg(WS_BG);
    let target_top = cy;
    for (i, row) in puzzle.target.iter().enumerate() {
        if cy >= y0 + h.saturating_sub(4) { break; }

        let label = format!("{}:", i);
        buf_set_str(buf, inner_x + 2, cy, &label, dim_style);

        let cell_x0 = inner_x + 2 + label.len() as u16 + 1;
        for (j, &bit) in row.iter().enumerate() {
            let cx = cell_x0 + (j as u16 * 4);
            let (glyph, style) = if bit != 0 {
                ("\u{2593}\u{2593}\u{2593}", Style::default().fg(TARGET_ON).bg(WS_BG))
            } else {
                ("\u{2591}\u{2591}\u{2591}", Style::default().fg(TARGET_OFF).bg(WS_BG))
            };
            buf_set_str(buf, cx, cy, glyph, style);
        }

        let hex_val = JacquardPuzzle::row_to_hex(row);
        let hex_str = format!("0x{:02X}", hex_val);
        buf_set_str(buf, cell_x0 + 34, cy, &hex_str, target_style);

        cy += 1;
    }

    // Z-depth: drop a shadow under the target-weave matrix block.
    let target_h = cy.saturating_sub(target_top);
    if target_h > 0 {
        let mw = 32u16.min(w.saturating_sub(8));
        layout::draw_drop_shadow(buf, Rect::new(inner_x + 5, target_top, mw, target_h));
    }

    // ── Status footer — the latest log line set in a brass measuring gauge. ──
    if let Some((msg, kind)) = puzzle.status_log.last() {
        let col = match kind {
            LogKind::Info    => Color::Rgb(170, 150, 110),
            LogKind::Error   => ERROR_FG,
            LogKind::Warning => WARN_FG,
            LogKind::Success => MATCH_OK,
        };
        layout::draw_gauge_footer(buf, area, msg, col);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// EVENT HANDLER — KEYBOARD INPUT
// ═════════════════════════════════════════════════════════════════════════════

/// Process a low-level keyboard event against the Jacquard puzzle state.
/// Arrow keys navigate, Space/Enter toggle bits, A/D add/delete cards, R runs weave.
/// True iff the player's drawn deck, looped, reproduces the repeating 4-row target
/// damask for a full simulation — i.e. the *shape* itself is mathematically correct,
/// independent of any downstream physical (roll/chain) limit. This is the FIRST gate the
/// run check consults, so a wrong design is reported as a pattern error, never masked by
/// a mechanical tear/jam.
fn pattern_matches(puzzle: &JacquardPuzzle) -> bool {
    if puzzle.cards.is_empty() {
        return false;
    }
    (0..SIMULATION_ROWS).all(|row| puzzle.cards[row % puzzle.cards.len()] == puzzle.target[row % 4])
}

pub fn handle_jacquard_input(
    key: KeyEvent,
    puzzle: &mut JacquardPuzzle,
    state: &mut GlobalStateContext,
    audio: &mut AudioEngine,
) {
    // ── Error state acknowledgment → phase transition ──
    if puzzle.snapped {
        puzzle.reset_for_phase(JacquardPhase::FalconChain);
        return;
    }
    if puzzle.jammed {
        puzzle.reset_for_phase(JacquardPhase::PunchedCard);
        return;
    }
    if puzzle.solved || puzzle.weaving {
        return; // No input during weave animation or after completion
    }

    // Alphanumeric tremor check
    let mut double_strike = false;
    if state.stilboestrol_ppm > 30.0 {
        let seed = state.frame_counter.wrapping_add(state.chemical_drift_seed).wrapping_add(1);
        let next_seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let chance = ((next_seed % 100) as f32 / 100.0).clamp(0.0, 1.0) * 0.25;
        if (next_seed % 100) as f32 / 100.0 < chance {
            if let KeyCode::Char(ch) = key.code {
                if ch.is_alphanumeric() {
                    double_strike = true;
                }
            }
        }
    }

    let runs = if double_strike { 2 } else { 1 };
    for run in 0..runs {
        if run == 1 {
            puzzle.push_log("[TREMOR]: Double-strike key registered".into(), LogKind::Warning);
        }
        match key.code {
            // ── Navigation ── (W/S mirror Up/Down; A/D remain add/delete-card here,
            //    so horizontal cursor movement stays on the arrow keys.)
            KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                if puzzle.cursor_row > 0 {
                    puzzle.cursor_row -= 1;
                }
                audio.grid_nav();
            }
            KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => {
                if puzzle.cursor_row + 1 < puzzle.cards.len() {
                    puzzle.cursor_row += 1;
                }
                audio.grid_nav();
            }
            KeyCode::Left => {
                if puzzle.cursor_col > 0 {
                    puzzle.cursor_col -= 1;
                }
                audio.grid_nav();
            }
            KeyCode::Right => {
                if puzzle.cursor_col < 7 {
                    puzzle.cursor_col += 1;
                }
                audio.grid_nav();
            }

            // ── Bit toggle (XOR) — punch/unpunch a hole (a structural decision). ──
            KeyCode::Enter | KeyCode::Char(' ') => {
                if puzzle.cursor_row < puzzle.cards.len() && puzzle.cursor_col < 8 {
                    puzzle.cards[puzzle.cursor_row][puzzle.cursor_col] ^= 1;
                    audio.menu_confirm();
                }
            }

            // ── Append blank card row ──
            KeyCode::Char('a') | KeyCode::Char('A') => {
                audio.loom_shuttle(); // heavy carriage slide as a layer is fed
                match puzzle.phase {
                    JacquardPhase::BouchonRoll => {
                        puzzle.cards.push([0u8; 8]);
                        puzzle.push_log(
                            format!("Roll extended to {} lines.", puzzle.cards.len()),
                            LogKind::Info,
                        );
                    }
                    JacquardPhase::FalconChain => {
                        if puzzle.cards.len() >= CHAIN_MASS_LIMIT {
                            puzzle.jammed = true;
                            puzzle.push_log(
                                "[MECHANISM_JAMMED]: Volumetric block-chain exceeds mass displacement boundaries.".into(),
                                LogKind::Error,
                            );
                        } else {
                            puzzle.cards.push([0u8; 8]);
                            puzzle.push_log(
                                format!("Chain block added. {} blocks linked.", puzzle.cards.len()),
                                LogKind::Info,
                            );
                        }
                    }
                    JacquardPhase::PunchedCard => {
                        if puzzle.cards.len() >= 4 {
                            puzzle.push_log(
                                "Max 4 cards in modular mode. Optimize with looping.".into(),
                                LogKind::Warning,
                            );
                        } else {
                            puzzle.cards.push([0u8; 8]);
                            puzzle.push_log(
                                format!("Card inserted. {} cards in deck.", puzzle.cards.len()),
                                LogKind::Info,
                            );
                        }
                    }
                }
            }

            // ── Delete selected card row ──
            KeyCode::Char('d') | KeyCode::Char('D') => {
                audio.loom_shuttle();
                if puzzle.cards.len() > 1 {
                    let idx = puzzle.cursor_row.min(puzzle.cards.len() - 1);
                    puzzle.cards.remove(idx);
                    if puzzle.cursor_row >= puzzle.cards.len() {
                        puzzle.cursor_row = puzzle.cards.len() - 1;
                    }
                    puzzle.push_log(
                        format!("Card {} removed. {} remaining.", idx, puzzle.cards.len()),
                        LogKind::Info,
                    );
                } else {
                    puzzle.push_log("Cannot remove last card.".into(), LogKind::Warning);
                }
            }

            // ── Trigger weave execution ──
            KeyCode::Char('r') | KeyCode::Char('R') => {
                if puzzle.cards.is_empty() {
                    puzzle.push_log("No cards to weave.".into(), LogKind::Error);
                    return;
                }
                // ── ORDER OF OPERATIONS ──
                // 1. Validate the drawn damask SHAPE against the target first. A wrong
                //    design is a pattern error, reported as such — never masked by a
                //    downstream mechanical paper-roll/carriage fault.
                if !pattern_matches(puzzle) {
                    puzzle.push_log(
                        "[ERR_PATTERN_MISMATCH]: Target weave profile not met.".into(),
                        LogKind::Error,
                    );
                    audio.glitch();
                    return;
                }
                // 2. The shape is correct — only NOW exercise the physical roll/chain
                //    capacity limits via the weave simulation.
                audio.loom_shuttle();
                puzzle.weaving = true;
                puzzle.weave_row = 0;
                puzzle.weave_tick_acc = 0;
                puzzle.tension = 0.0;
                puzzle.weave_match_map.clear();
                puzzle.compression_flagged = false;
                puzzle.push_log("Weaving initiated...".into(), LogKind::Info);
            }

            _ => {}
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// TICK — WEAVE SIMULATION STEP
// ═════════════════════════════════════════════════════════════════════════════

/// Advance the weave simulation by one step per WEAVE_PACE frames.
/// Must be called once per frame tick from the main engine loop.
pub fn tick_jacquard(
    puzzle: &mut JacquardPuzzle,
    state: &mut GlobalStateContext,
    dialogue: &mut DialogueEngine,
) {
    if !puzzle.weaving || puzzle.cards.is_empty() {
        return;
    }

    puzzle.weave_tick_acc += 1;
    if puzzle.weave_tick_acc < WEAVE_PACE {
        return;
    }
    puzzle.weave_tick_acc = 0;

    let row = puzzle.weave_row;

    match puzzle.phase {
        // ─────────────────────────────────────────────────────────────────
        // Phase 1: Continuous roll tears at ROLL_TEAR_LIMIT
        // ─────────────────────────────────────────────────────────────────
        JacquardPhase::BouchonRoll => {
            // Gradual tension buildup toward the tear point
            puzzle.tension = (row as f32) / (ROLL_TEAR_LIMIT as f32);

            if row >= ROLL_TEAR_LIMIT {
                puzzle.tension = 1.0;
                puzzle.snapped = true;
                puzzle.weaving = false;
                puzzle.push_log(
                    "[ERR_STR_TEAR]: Continuous paper roll torn under mechanical tension.".into(),
                    LogKind::Error,
                );
                return;
            }

            // Feed from roll data (blank if roll exhausted — no looping)
            let card = if row < puzzle.cards.len() {
                puzzle.cards[row]
            } else {
                [0u8; 8]
            };
            let target = puzzle.target[row % 4];
            puzzle.weave_match_map.push(card == target);
        }

        // ─────────────────────────────────────────────────────────────────
        // Phase 2: Chain exhaustion when discrete blocks run out
        // ─────────────────────────────────────────────────────────────────
        JacquardPhase::FalconChain => {
            // Gradual tension relative to chain capacity
            puzzle.tension = (row as f32) / (puzzle.cards.len().max(1) as f32);

            if row >= puzzle.cards.len() {
                puzzle.tension = 1.0;
                puzzle.jammed = true;
                puzzle.weaving = false;
                puzzle.push_log(
                    format!(
                        "[MECHANISM_JAMMED]: Chain depleted at row {}. Blocks cannot loop.",
                        row
                    ),
                    LogKind::Error,
                );
                return;
            }

            let card = puzzle.cards[row];
            let target = puzzle.target[row % 4];
            puzzle.weave_match_map.push(card == target);
        }

        // ─────────────────────────────────────────────────────────────────
        // Phase 3: Modular cards with period looping
        // ─────────────────────────────────────────────────────────────────
        JacquardPhase::PunchedCard => {
            let card_idx = row % puzzle.cards.len();
            let target_idx = row % 4;
            let card = puzzle.cards[card_idx];
            let target = puzzle.target[target_idx];
            puzzle.weave_match_map.push(card == target);
        }
    }

    puzzle.weave_row += 1;

    // ── Simulation complete — evaluate result ──
    if puzzle.weave_row >= SIMULATION_ROWS {
        puzzle.weaving = false;
        puzzle.tension = 0.0;

        // Only Phase 3 can reach completion (Phases 1 & 2 always fail before this)
        if puzzle.phase == JacquardPhase::PunchedCard {
            let all_match = puzzle.weave_match_map.iter().all(|&m| m);
            if all_match {
                puzzle.consecutive_fails = 0;
                if puzzle.cards.len() == 2 {
                    // ── OPTIMAL: period-2 compression discovered ──
                    puzzle.solved = true;
                    puzzle.push_log(
                        "OPTIMAL: 2-card period loop verified. The Loom is complete.".into(),
                        LogKind::Success,
                    );
                    if !state.acts_completed.contains(&Act::Jacquard1804) {
                        state.acts_completed.push(Act::Jacquard1804);
                    }
                    dialogue.play(
                        Speaker::Turing,
                        "[TURING]: \"There it is. The pattern lives on the card now, apart from the loom. The instruction is free of the machine.\"",
                        Some(VoiceCue::Victory(Act::Jacquard1804)),
                    );
                    state.current_act = Act::Babbage1837;
                } else {
                    // ── Overweight loom: the pattern is correct, but too many
                    //    cards make the carriage too heavy to cycle. Spike the
                    //    vision blur so the rows judder violently — the player is
                    //    forced (structurally, not by text) to discover the
                    //    repeating period and compress the deck down to it. ──
                    puzzle.compression_flagged = true;
                    state.vision_blur_factor = 1.0;
                    state.melt_candle(8);
                    puzzle.push_log(
                        "[OVERWEIGHT]: carriage stalls \u{2014} the deck repeats itself. Strip it to the loop.".into(),
                        LogKind::Error,
                    );
                }
            } else {
                let matched = puzzle.weave_match_map.iter().filter(|&&m| m).count();
                puzzle.push_log(
                    format!("Weave mismatch: {}/{} rows. Adjust card data.", matched, SIMULATION_ROWS),
                    LogKind::Error,
                );
                // ── Blunder commentary: a fourth-wall taunt after three straight
                //    failed verifications of an erratic, nonsensical pattern. ──
                puzzle.consecutive_fails += 1;
                if puzzle.consecutive_fails >= 3 {
                    puzzle.consecutive_fails = 0;
                    dialogue.play(
                        Speaker::MindLog,
                        "[MIND_LOG]: \"Hmm... hah. A complete mistake. Utter nonsense. Watch the registers.\"",
                        Some(VoiceCue::Blunder),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod pattern_order_tests {
    use super::*;

    #[test]
    fn correct_designs_pass_the_pattern_gate() {
        let mut p = JacquardPuzzle::new();
        // The optimal period-2 deck reproduces the 0xAA/0x55 target.
        p.cards = vec![
            [1, 0, 1, 0, 1, 0, 1, 0], // 0xAA
            [0, 1, 0, 1, 0, 1, 0, 1], // 0x55
        ];
        assert!(pattern_matches(&p), "the 2-card period solution must validate");

        // A full 4-card repeat is equally valid.
        p.cards = vec![
            [1, 0, 1, 0, 1, 0, 1, 0],
            [0, 1, 0, 1, 0, 1, 0, 1],
            [1, 0, 1, 0, 1, 0, 1, 0],
            [0, 1, 0, 1, 0, 1, 0, 1],
        ];
        assert!(pattern_matches(&p));
    }

    #[test]
    fn wrong_design_fails_the_pattern_gate_first() {
        let mut p = JacquardPuzzle::new();
        // A single blank card cannot reproduce the alternating target → mismatch, so the
        // run short-circuits to ERR_PATTERN_MISMATCH before any physical roll check.
        p.cards = vec![[0u8; 8]];
        assert!(!pattern_matches(&p));

        // One wrong bit anywhere also fails the shape check.
        p.cards = vec![
            [1, 0, 1, 0, 1, 0, 1, 0],
            [0, 1, 0, 1, 0, 1, 0, 0], // last bit wrong
        ];
        assert!(!pattern_matches(&p));
    }
}
