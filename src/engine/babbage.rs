use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    Frame,
};

use super::state::{Act, GlobalStateContext, ScreenState};
use super::mind_log::{DialogueEngine, Speaker, VoiceCue};
use super::layout;

// ─────────────────────────────────────────────────────────────────────────────
// TrueColor Palette Constants — workspace brass/mechanical theme
// ─────────────────────────────────────────────────────────────────────────────
const WS_BG:           Color = Color::Rgb(12, 11, 10);
const HEADER_FG:       Color = Color::Rgb(220, 170, 80);
const DIM_FG:          Color = Color::Rgb(80, 70, 60);
const VAL_FG:          Color = Color::Rgb(240, 200, 120);
const GEAR_FG:         Color = Color::Rgb(210, 140, 60);
const MATCH_OK:        Color = Color::Rgb(80, 200, 80);
const ERROR_FG:        Color = Color::Rgb(255, 60, 40);
const WARN_FG:         Color = Color::Rgb(255, 180, 40);
const PHASE1_ACCENT:   Color = Color::Rgb(180, 150, 100);
const PHASE2_ACCENT:   Color = Color::Rgb(210, 110, 60);

// ─────────────────────────────────────────────────────────────────────────────
// Simulation Constants
// ─────────────────────────────────────────────────────────────────────────────
// Each column settles on its own cadence so the carry travels like a wave instead
// of all gears slamming home at once. Unit frames per column "tick weight":
//   Value = 1 frame, Delta 1 = 2 frames, Delta 2 = 3 frames.
const COLUMN_WEIGHTS: [u16; 3] = [1, 2, 3];
const STAGGER_UNIT: u16 = 12; // frames per weight-unit
// Total crank cycle = the slowest column (Delta 2, weight 3) plus settle slack.
const CRANK_DURATION_FRAMES: u16 = STAGGER_UNIT * 3 + 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BabbagePhase {
    MethodOfDifferences, // Phase 1: Mathematical reduction to constant differences
    HardwareBug,         // Phase 2: Simulating carry propagation and installing delay buffers
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogKind {
    Info,
    Error,
    Warning,
    Success,
}

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
}

pub struct BabbagePuzzle {
    pub columns: [i32; 3],          // [Value, Delta 1, Delta 2]
    pub target_start: [i32; 3],     // Target initial baseline: [1, 3, 2]
    pub cursor_col: usize,          // Cursor selection (0 to 2)
    pub cursor_row: usize,          // 0 for Columns, 1 for Delay Buffers
    pub carry_buffers: [bool; 3],   // [D0, D1, D2] delay buffers installed
    pub phase: BabbagePhase,
    pub crank_active: bool,
    pub crank_frame: u16,
    pub step_count: usize,          // How many terms computed
    pub locked: bool,               // Mechanical gear lock failure
    pub locked_col: Option<usize>,  // Column index that caused the lock
    pub solved: bool,
    pub status_log: Vec<(String, LogKind)>,
}

impl BabbagePuzzle {
    pub fn new() -> Self {
        Self {
            columns: [0, 0, 0],
            target_start: [1, 3, 2],
            cursor_col: 0,
            cursor_row: 0,
            carry_buffers: [false, false, false],
            phase: BabbagePhase::MethodOfDifferences,
            crank_active: false,
            crank_frame: 0,
            step_count: 0,
            locked: false,
            locked_col: None,
            solved: false,
            status_log: vec![
                ("Compute constant Delta 2 differences for f(n) = n^2...".into(), LogKind::Info),
                ("Columns represent: [ VALUE ] [ DELTA 1 ] [ DELTA 2 ]".into(), LogKind::Info),
            ],
        }
    }

    fn push_log(&mut self, msg: String, kind: LogKind) {
        self.status_log.push((msg, kind));
        if self.status_log.len() > 8 {
            self.status_log.remove(0);
        }
    }

    fn reset_for_phase(&mut self, phase: BabbagePhase) {
        self.phase = phase;
        self.crank_active = false;
        self.crank_frame = 0;
        self.locked = false;
        self.locked_col = None;
        
        match phase {
            BabbagePhase::MethodOfDifferences => {
                self.columns = [0, 0, 0];
                self.cursor_row = 0;
                self.step_count = 0;
                self.push_log("Reset to manual difference calculations.".into(), LogKind::Info);
            }
            BabbagePhase::HardwareBug => {
                self.columns = [1, 3, 2];
                self.cursor_row = 1; // Start focused on the delay buffers
                self.step_count = 0;
                self.push_log("Phase 2: Hardware carry configuration active.".into(), LogKind::Info);
                self.push_log("Insert delay buffers [D] to stagger carry propagation.".into(), LogKind::Info);
                self.push_log("Press [C] or [R] to turn the engine crank.".into(), LogKind::Info);
            }
        }
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
    let err_bg = Color::Rgb(35, 8, 8);
    let code_style = Style::default().fg(ERROR_FG).bg(err_bg);
    let text_style = Style::default().fg(Color::Rgb(180, 50, 40)).bg(err_bg);
    let hint_style = Style::default().fg(DIM_FG).bg(err_bg);

    let overlay_h: u16 = 9;
    let overlay_w = area.width.saturating_sub(4);
    let ox = area.x + 2;
    let oy = area.y + (area.height / 2).saturating_sub(4);

    let overlay_rect = Rect::new(ox, oy, overlay_w, overlay_h);
    buf_fill_bg(buf, overlay_rect, err_bg);
    draw_box(buf, overlay_rect, Style::default().fg(Color::Rgb(120, 30, 30)).bg(err_bg));

    let cx = ox + overlay_w / 2;
    let code_half = code.len() as u16 / 2;
    buf_set_str(buf, cx.saturating_sub(code_half), oy + 2, code, code_style);
    let l1_half = line1.len() as u16 / 2;
    buf_set_str(buf, cx.saturating_sub(l1_half), oy + 4, line1, text_style);
    let l2_half = line2.len() as u16 / 2;
    buf_set_str(buf, cx.saturating_sub(l2_half), oy + 5, line2, text_style);
    let hint = "[ press any key to reset engine ]";
    buf_set_str(buf, cx.saturating_sub(hint.len() as u16 / 2), oy + 7, hint, hint_style);
}

fn render_victory_overlay(buf: &mut Buffer, area: Rect) {
    let vic_bg = Color::Rgb(10, 22, 12);
    let gold = Style::default().fg(Color::Rgb(255, 210, 80)).bg(vic_bg);
    let sub = Style::default().fg(Color::Rgb(120, 185, 100)).bg(vic_bg);
    let dim = Style::default().fg(DIM_FG).bg(vic_bg);

    let overlay_h: u16 = 9;
    let overlay_w = area.width.saturating_sub(4);
    let ox = area.x + 2;
    let oy = area.y + (area.height / 2).saturating_sub(4);

    let overlay_rect = Rect::new(ox, oy, overlay_w, overlay_h);
    buf_fill_bg(buf, overlay_rect, vic_bg);
    draw_box(buf, overlay_rect, Style::default().fg(Color::Rgb(60, 130, 70)).bg(vic_bg));

    let cx = ox + overlay_w / 2;
    let title = "THE DIFFERENCE ENGINE COMPLETE";
    buf_set_str(buf, cx.saturating_sub(title.len() as u16 / 2), oy + 2, title, gold);
    let s1 = "Columns sing. Clean tables. At last.";
    buf_set_str(buf, cx.saturating_sub(s1.len() as u16 / 2), oy + 4, s1, sub);
    let s2 = "The memory carries the wave.";
    buf_set_str(buf, cx.saturating_sub(s2.len() as u16 / 2), oy + 5, s2, sub);
    let s3 = "Advancing to Act III: Lovelace...";
    buf_set_str(buf, cx.saturating_sub(s3.len() as u16 / 2), oy + 7, s3, dim);
}

/// Rotating-gear glyph. While cranking, each column spins at a rate set by its own
/// tick weight (Value fastest, Delta 2 slowest) and freezes once it has settled —
/// so the three gears visibly desync into a propagating carry wave.
fn get_gear_char(value: i32, crank_frame: u16, active: bool, col: usize) -> char {
    let chars = ['│', '/', '─', '\\'];
    let weight = COLUMN_WEIGHTS[col.min(2)];
    let settle = weight * STAGGER_UNIT;
    if !active {
        chars[(value as usize + col) % 4]
    } else if crank_frame >= settle {
        // This column has caught up to the wave and locked into place.
        chars[(value as usize + col) % 4]
    } else {
        let step = (crank_frame / weight) as usize;
        chars[(step + col) % 4]
    }
}

/// In-phase delays must form a staggered cascade from the slowest column (Delta 2)
/// down to the fastest (Value): you may only buffer Value once Delta 1 is buffered,
/// and Delta 1 only once Delta 2 is. Any inversion means a column pulls out of turn.
fn delays_in_phase(buffers: &[bool; 3]) -> bool {
    (buffers[2] as u8) >= (buffers[1] as u8) && (buffers[1] as u8) >= (buffers[0] as u8)
}

/// The column index that pulls too early (the axis the crunch noise rains down).
fn out_of_phase_col(buffers: &[bool; 3]) -> usize {
    if buffers[0] && !buffers[1] {
        0
    } else if buffers[1] && !buffers[2] {
        1
    } else {
        0
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// RENDER — THE WORKSPACE (ACT II)
// ═════════════════════════════════════════════════════════════════════════════

pub fn render_workspace(
    f: &mut Frame,
    area: Rect,
    state: &mut GlobalStateContext,
    puzzle: &BabbagePuzzle,
) {
    let mut area = area;
    
    // Visual Jitter on Failure
    if puzzle.locked {
        let intensity = (state.stilboestrol_ppm * 0.05).clamp(1.0, 5.0) as i16;
        let shift = ((state.frame_counter % 3) as i16 - 1) * intensity;
        area.x = (area.x as i16 + shift).max(0) as u16;
    }

    let buf = f.buffer_mut();
    // Blueprint dot-matrix sheet — keeps the panel from reading as a flat void.
    layout::draw_workspace_grid(buf, area, WS_BG);

    // ── RENDER GUARD: ambient phase shows only the dormant grid + gauge baseline.
    //    The difference engine's columns, gears and delays stay uncalculated and
    //    hidden until the dossier is taken up (ActivePuzzle). ──
    if state.screen_state == ScreenState::AmbientDesk {
        layout::draw_gauge_footer(buf, area, "", DIM_FG);
        return;
    }

    let x0 = area.x;
    let y0 = area.y;
    let w = area.width;
    let h = area.height;

    if w < 20 || h < 10 { return; }

    // Border
    let border_style = Style::default().fg(Color::Rgb(140, 110, 80)).bg(WS_BG);
    draw_box(buf, area, border_style);

    // Title Header
    let (phase_name, phase_year, phase_color) = match puzzle.phase {
        BabbagePhase::MethodOfDifferences => ("METHOD OF DIFFERENCES", "1822", PHASE1_ACCENT),
        BabbagePhase::HardwareBug => ("CARRY STAGGERING", "1837", PHASE2_ACCENT),
    };
    let header_text = format!(" DIFFERENCE ENGINE \u{2500}\u{2500} {} ({}) ", phase_name, phase_year);
    let phase_style = Style::default().fg(phase_color).bg(WS_BG);
    buf_set_str(buf, x0 + 2, y0, &header_text, phase_style);

    if puzzle.solved {
        render_victory_overlay(buf, area);
        return;
    }

    let dim_style  = Style::default().fg(DIM_FG).bg(WS_BG);
    let section_style = Style::default().fg(Color::Rgb(160, 140, 100)).bg(WS_BG);
    let val_style = Style::default().fg(VAL_FG).bg(WS_BG);
    let gear_style = Style::default().fg(GEAR_FG).bg(WS_BG);

    let inner_x = x0 + 2;
    let mut cy = y0 + 1;

    // Sub-header status
    let status_line = if puzzle.crank_active {
        // Which columns have caught the wave yet (settle = weight * unit).
        let f = puzzle.crank_frame;
        let wave: String = (0..3)
            .map(|c| if f >= COLUMN_WEIGHTS[c] * STAGGER_UNIT { '\u{25CF}' } else { '\u{25CB}' })
            .collect();
        format!("[ CARRY WAVE ]   V D1 D2 settling: {}", wave)
    } else {
        "[ IDLE ]   the gears wait, out of phase".into()
    };
    buf_set_str(buf, inner_x, cy, &status_line, Style::default().fg(HEADER_FG).bg(WS_BG));
    cy += 2;

    // Table Column Labels
    buf_set_str(buf, inner_x + 10, cy, "[ VALUE ]     [ DELTA 1 ]   [ DELTA 2 ]", section_style);
    cy += 1.max((h as i16 - 15) as u16 / 4);

    // Axis Row (Rotating Gears)
    buf_set_str(buf, inner_x + 2, cy, "GEARS:", dim_style);
    let g0_char = get_gear_char(puzzle.columns[0], puzzle.crank_frame, puzzle.crank_active, 0);
    let g1_char = get_gear_char(puzzle.columns[1], puzzle.crank_frame, puzzle.crank_active, 1);
    let g2_char = get_gear_char(puzzle.columns[2], puzzle.crank_frame, puzzle.crank_active, 2);
    
    buf_set_str(buf, inner_x + 13, cy, &format!("({})", g0_char), gear_style);
    buf_set_str(buf, inner_x + 27, cy, &format!("({})", g1_char), gear_style);
    buf_set_str(buf, inner_x + 41, cy, &format!("({})", g2_char), gear_style);
    cy += 2;

    // Gear Value Boxes
    buf_set_str(buf, inner_x + 2, cy + 1, "VALUE:", dim_style);

    for i in 0..3 {
        // In Phase 1 the cursor lives on the value boxes.
        let is_selected = puzzle.phase == BabbagePhase::MethodOfDifferences && puzzle.cursor_col == i;
        let border_color = if is_selected {
            let pulse = ((state.frame_counter as f64 * 0.15).sin() * 30.0) as i16;
            Color::Rgb((220 + pulse).clamp(180, 255) as u8, (180 + pulse).clamp(140, 255) as u8, (80 + pulse).clamp(40, 255) as u8)
        } else {
            Color::Rgb(100, 90, 80)
        };
        let box_style = Style::default().fg(border_color).bg(WS_BG);

        let bx = inner_x + 11 + (i as u16 * 14);
        buf_set_str(buf, bx, cy,     "┌───┐", box_style);
        buf_set_str(buf, bx, cy + 1, &format!("│{:03}│", puzzle.columns[i]), val_style);
        buf_set_str(buf, bx, cy + 2, "└───┘", box_style);
        // Z-depth: a physical drop-shadow under each register box.
        layout::draw_drop_shadow(buf, Rect::new(bx, cy, 5, 3));
    }
    cy += 4;

    // Carry Delay Buffers Row (Phase 2 only). Each buffer carries a delay weight
    // (Value 1 · Delta1 2 · Delta2 3); they must be staggered into a cascade.
    if puzzle.phase == BabbagePhase::HardwareBug {
        let in_phase = delays_in_phase(&puzzle.carry_buffers);
        buf_set_str(buf, inner_x + 2, cy, "DELAYS:", dim_style);
        for i in 0..3 {
            let active = puzzle.carry_buffers[i];
            // Delay slot rendered as a stagger of dots equal to the tick weight.
            let slots = ".".repeat(COLUMN_WEIGHTS[i] as usize);
            let label = if active {
                format!("[D{}]", slots)
            } else {
                format!("[ {} ]", " ".repeat(COLUMN_WEIGHTS[i] as usize))
            };
            let style = if !active {
                Style::default().fg(Color::Rgb(90, 80, 70)).bg(WS_BG)
            } else if in_phase {
                Style::default().fg(MATCH_OK).bg(WS_BG)
            } else {
                // This delay is pulling out of turn — it will shatter the drive.
                Style::default().fg(ERROR_FG).bg(WS_BG)
            };

            // In Phase 2 the cursor lives on the delay buffers.
            let is_selected = puzzle.cursor_col == i;
            let final_style = if is_selected {
                style.fg(Color::Rgb(255, 230, 150))
            } else {
                style
            };

            let bx = inner_x + 11 + (i as u16 * 14);
            buf_set_str(buf, bx, cy, &label, final_style);
        }
    }
    cy += 2;

    // Simulation stats — terms verified, no target vector spoiler.
    if puzzle.phase == BabbagePhase::HardwareBug {
        let step_text = format!("Carry wave held for {} terms", puzzle.step_count);
        let step_style = if puzzle.step_count >= 5 {
            Style::default().fg(MATCH_OK)
        } else {
            Style::default().fg(WARN_FG)
        };
        buf_set_str(buf, inner_x, cy, &step_text, step_style);
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

    // The Ink Bleed Cascade
    if puzzle.locked {
        let fail_col = puzzle.locked_col.unwrap_or(0);
        let cx0 = inner_x + 11 + (fail_col as u16 * 14);
        let mut rng = Lcg::new(state.frame_counter);
        
        let corrupt_chars = [
            '0','1','2','3','4','5','6','7','8','9','A','B','C','D','E','F',
            '@','*','%','#','!','?','Ø','Æ','ß','µ','▒','░','█','▄','▀'
        ];
        
        for y in y0 + 2..=y0 + h.saturating_sub(3) {
            for dx in 0..5 {
                let x = cx0 + dx;
                let ch = corrupt_chars[(rng.next() as usize) % corrupt_chars.len()];
                let cell_style = Style::default().fg(Color::Rgb(220, 50, 50)).bg(WS_BG);
                buf_set(buf, x, y, ch, cell_style);
            }
        }
        
        render_error_overlay(
            buf, area,
            "[GEAR_LOCK_VAL_OVERFLOW]",
            "Carry propagation failed \u{2014}",
            "all column gears locked simultaneously.",
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// EVENT HANDLER — KEYBOARD INPUT
// ═════════════════════════════════════════════════════════════════════════════

pub fn handle_babbage_input(
    key: KeyEvent,
    puzzle: &mut BabbagePuzzle,
    state: &mut GlobalStateContext,
    dialogue: &mut DialogueEngine,
) {
    // Acknowledging a shattered drive resets the engine — never a dead key.
    if puzzle.locked {
        puzzle.reset_for_phase(BabbagePhase::HardwareBug);
        state.arrhythmia_multiplier = 0.0;
        return;
    }
    // Input is only suspended while the gears are physically mid-cycle.
    if puzzle.solved || puzzle.crank_active {
        return;
    }

    // Alphanumeric tremor check
    let mut double_strike = false;
    if state.stilboestrol_ppm > 30.0 {
        let seed = state.frame_counter.wrapping_add(state.chemical_drift_seed).wrapping_add(2);
        let next_seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let chance = ((state.stilboestrol_ppm - 30.0) / 70.0).clamp(0.0, 1.0) * 0.25;
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
        // NOTE: there is no hidden row-mode any more. Left/Right always select a
        // column; Up/Down always *act* on that column in the current phase. This
        // removes the phase-desynchronisation deadlock that froze the keyboard.
        match key.code {
            KeyCode::Left => {
                puzzle.cursor_col = puzzle.cursor_col.saturating_sub(1);
            }
            KeyCode::Right => {
                if puzzle.cursor_col < 2 {
                    puzzle.cursor_col += 1;
                }
            }
            KeyCode::Up => match puzzle.phase {
                BabbagePhase::MethodOfDifferences => {
                    puzzle.columns[puzzle.cursor_col] = (puzzle.columns[puzzle.cursor_col] + 1).min(999);
                    check_phase1_match(puzzle);
                }
                BabbagePhase::HardwareBug => {
                    // Up installs the delay on the selected column.
                    puzzle.carry_buffers[puzzle.cursor_col] = true;
                    puzzle.push_log(format!("Delay buffer D{} installed.", puzzle.cursor_col), LogKind::Info);
                }
            },
            KeyCode::Down => match puzzle.phase {
                BabbagePhase::MethodOfDifferences => {
                    puzzle.columns[puzzle.cursor_col] = (puzzle.columns[puzzle.cursor_col] - 1).max(0);
                    check_phase1_match(puzzle);
                }
                BabbagePhase::HardwareBug => {
                    // Down removes the delay on the selected column.
                    puzzle.carry_buffers[puzzle.cursor_col] = false;
                    puzzle.push_log(format!("Delay buffer D{} removed.", puzzle.cursor_col), LogKind::Info);
                }
            },
            KeyCode::Enter | KeyCode::Char(' ') => {
                if puzzle.phase == BabbagePhase::HardwareBug {
                    let now = !puzzle.carry_buffers[puzzle.cursor_col];
                    puzzle.carry_buffers[puzzle.cursor_col] = now;
                    let s = if now { "installed" } else { "removed" };
                    puzzle.push_log(format!("Delay buffer D{} {}.", puzzle.cursor_col, s), LogKind::Info);
                }
            }
            KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Char('r') | KeyCode::Char('R') => {
                if puzzle.phase == BabbagePhase::HardwareBug {
                    if delays_in_phase(&puzzle.carry_buffers) {
                        puzzle.crank_active = true;
                        puzzle.crank_frame = 0;
                        puzzle.push_log("Cranking \u{2014} the carry begins to travel...".into(), LogKind::Info);
                    } else {
                        // ── OUT-OF-PHASE CRUNCH: a delay pulls before its turn and
                        //    the whole drive shatters at once. ──
                        trigger_crunch(puzzle, state, dialogue);
                    }
                }
            }
            _ => {}
        }
    }
}

/// The loud mechanical crunch fired when the crank is turned with out-of-phase
/// delays. Per spec: clear the verified terms vector, rain hexadecimal noise down
/// the offending column axis, and permanently poison the workspace by +20.0 ppm.
fn trigger_crunch(
    puzzle: &mut BabbagePuzzle,
    state: &mut GlobalStateContext,
    dialogue: &mut DialogueEngine,
) {
    let col = out_of_phase_col(&puzzle.carry_buffers);

    // Clear the verified terms vector — the sequence collapses back to baseline.
    puzzle.columns = puzzle.target_start;
    puzzle.step_count = 0;

    // Lock the drive and mark the column axis the noise will rain down.
    puzzle.locked = true;
    puzzle.locked_col = Some(col);
    puzzle.crank_active = false;
    puzzle.crank_frame = 0;

    // Permanent consequences.
    state.stilboestrol_ppm += 20.0;
    state.melt_candle(15);
    state.arrhythmia_multiplier = 2.0;

    puzzle.push_log(
        "[DRIVE_SHATTERED]: a delay pulled out of turn \u{2014} the gears all seized at once.".into(),
        LogKind::Error,
    );

    // Fourth-wall voice hook: the loud mechanical crunch.
    dialogue.play(
        Speaker::Turing,
        "[TURING]: \"No \u{2014} they all pulled at once again. Nine carries to zero and the whole drive shears. The impact must be staggered, the carry made to travel, not strike.\"",
        Some(VoiceCue::BabbageCrunch),
    );
}

fn check_phase1_match(puzzle: &mut BabbagePuzzle) {
    if puzzle.columns == puzzle.target_start {
        puzzle.push_log("Baseline vector verified: [1, 3, 2]".into(), LogKind::Success);
        puzzle.reset_for_phase(BabbagePhase::HardwareBug);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// TICK — SIMULATION STEP
// ═════════════════════════════════════════════════════════════════════════════

pub fn tick_babbage(
    puzzle: &mut BabbagePuzzle,
    state: &mut GlobalStateContext,
    dialogue: &mut DialogueEngine,
) {
    if !puzzle.crank_active || puzzle.locked || puzzle.solved {
        return;
    }

    puzzle.crank_frame += 1;
    if puzzle.crank_frame < CRANK_DURATION_FRAMES {
        return;
    }

    puzzle.crank_active = false;
    puzzle.crank_frame = 0;

    let old_val = puzzle.columns[0];
    let old_d1 = puzzle.columns[1];
    let old_d2 = puzzle.columns[2];

    let new_d1 = old_d1 + old_d2;
    let new_d2 = old_d2;
    let new_val = old_val + new_d1;

    // Check carry propagation
    let mut carry_val = false;
    let mut carry_d1 = false;
    let carry_d2 = false;

    // Value addition carry (units digit overflow)
    if (old_val % 10) + (new_d1 % 10) >= 10 {
        carry_val = true;
    }
    // Delta 1 addition carry (units digit overflow)
    if (old_d1 % 10) + (old_d2 % 10) >= 10 {
        carry_d1 = true;
    }

    let mut failed = false;
    let mut fail_col = None;
    if carry_val && !puzzle.carry_buffers[0] {
        failed = true;
        fail_col = Some(0);
        puzzle.push_log("[FATAL ERROR]: Carry overflow at Value gear column!".into(), LogKind::Error);
    }
    if carry_d1 && !puzzle.carry_buffers[1] {
        failed = true;
        if fail_col.is_none() { fail_col = Some(1); }
        puzzle.push_log("[FATAL ERROR]: Carry overflow at Delta 1 gear column!".into(), LogKind::Error);
    }
    if carry_d2 && !puzzle.carry_buffers[2] {
        failed = true;
        if fail_col.is_none() { fail_col = Some(2); }
        puzzle.push_log("[FATAL ERROR]: Carry overflow at Delta 2 gear column!".into(), LogKind::Error);
    }

    if failed {
        puzzle.locked = true;
        puzzle.locked_col = fail_col;
        puzzle.push_log("[FATAL ERROR]: Carry propagation failed — all gears lock at once. [GEAR_LOCK_VAL_OVERFLOW]".into(), LogKind::Error);
        
        // Permanent penalties to global context
        state.melt_candle(15);
        state.stilboestrol_ppm += 15.0; // permanent increase
        state.arrhythmia_multiplier = 2.0; // trigger skipped beat / flutter arrhythmia in mind_log
        return;
    }

    // Success step
    puzzle.columns[0] = new_val;
    puzzle.columns[1] = new_d1;
    puzzle.columns[2] = new_d2;
    puzzle.step_count += 1;

    puzzle.push_log(format!("Crank step {} successful: [{:03}, {:03}, {:03}]", puzzle.step_count, new_val, new_d1, new_d2), LogKind::Success);

    if puzzle.step_count >= 5 {
        puzzle.solved = true;
        puzzle.push_log("OPTIMAL: Sequenced delays verified. Babbage Difference Engine solved.".into(), LogKind::Success);
        if !state.acts_completed.contains(&Act::Babbage1837) {
            state.acts_completed.push(Act::Babbage1837);
        }
        dialogue.play(
            Speaker::Turing,
            "[TURING]: \"There. The columns sing. The carry moved like a wave and nothing broke. Clean tables, at last.\"",
            Some(VoiceCue::Victory(Act::Babbage1837)),
        );
        state.current_act = Act::Lovelace1843;
    }
}
