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
// TrueColor palette — amber phosphor degrading into interrogation-room white
// ─────────────────────────────────────────────────────────────────────────────
const WS_BG:     Color = Color::Rgb(10, 8, 0);
const BORDER_FG: Color = Color::Rgb(120, 90, 20);
const HEADER_FG: Color = Color::Rgb(255, 213, 102);
const TEXT_FG:   Color = Color::Rgb(255, 176, 0);
const DIM_FG:    Color = Color::Rgb(74, 50, 5);
const SECTION_FG:Color = Color::Rgb(160, 140, 100);
const BIT_ON:    Color = Color::Rgb(255, 176, 0);
const BIT_OFF:   Color = Color::Rgb(42, 30, 2);
const TARGET_FG: Color = Color::Rgb(153, 104, 10);
const CURSOR_FG: Color = Color::Rgb(255, 255, 210);
const OK_FG:     Color = Color::Rgb(255, 213, 102);
const FAIL_FG:   Color = Color::Rgb(255, 102, 51);
const CORRUPT_FG:Color = Color::Rgb(180, 80, 20);
/// Diluted interrogation beam — a desaturated charcoal shadow sweep (#242428) rather
/// than a blinding opaque white, so gate glyphs stay legible as the light passes.
const SEARCHLIGHT: Color = Color::Rgb(0x24, 0x24, 0x28);

// ─────────────────────────────────────────────────────────────────────────────
// Tuning constants
// ─────────────────────────────────────────────────────────────────────────────
const GRID_W: usize = 6; // boolean lanes
const GRID_H: usize = 4; // reduction rows
/// Toxicity above which the physical tremor begins corrupting keystrokes.
const TREMOR_PPM: f32 = 30.0;
/// Toxicity above which the logical truth tables themselves start to lie.
const GUILT_PPM: f32 = 50.0;
/// Frames between interrogation-monologue fragments fed to the mind log.
const MONO_GAP: u16 = 210;
/// Corruption glyphs that bleed across the board as the mind dissolves.
const INK_GLYPHS: [char; 5] = ['@', '#', '%', '&', '\u{00A7}'];

/// Fragmented court-record phrases stamped into the decaying board texture.
const COURT_FRAGMENTS: &[&str] = &[
    "section 11",
    "gross indecency",
    "you will sign",
    "did you...",
    "probation",
    "50mg daily",
    "Harry",
    "I have done",
    "nothing wrong",
];

// ═════════════════════════════════════════════════════════════════════════════
// DATA STRUCTURES
// ═════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    And,
    Or,
    Xor,
    Not,
}

impl Gate {
    fn glyph(self) -> char {
        match self {
            Gate::And => '&',
            Gate::Or => '|',
            Gate::Xor => '^',
            Gate::Not => '!',
        }
    }

    /// Apply the gate. `Not` ignores the second operand (the lane below the carry).
    fn apply(self, a: u8, b: u8) -> u8 {
        match self {
            Gate::And => a & b,
            Gate::Or => a | b,
            Gate::Xor => a ^ b,
            Gate::Not => 1 - (a & 1),
        }
    }

    fn from_key(c: char) -> Option<Gate> {
        match c {
            '&' => Some(Gate::And),
            '|' => Some(Gate::Or),
            '^' => Some(Gate::Xor),
            '!' => Some(Gate::Not),
            _ => None,
        }
    }

    /// Advance one step around the circular gate queue: `&` → `|` → `^` → `!` → `&`.
    /// Drives the streamlined Space-bar cycling workflow (no Shift-symbol typing).
    fn cycle(self) -> Gate {
        match self {
            Gate::And => Gate::Or,
            Gate::Or => Gate::Xor,
            Gate::Xor => Gate::Not,
            Gate::Not => Gate::And,
        }
    }
}

const GATE_CYCLE: [Gate; 4] = [Gate::And, Gate::Or, Gate::Xor, Gate::Not];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogKind {
    Info,
    Error,
    Warning,
    Success,
}

/// A tiny self-contained LCG — deterministic per-frame artifacts, no external deps.
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed.wrapping_add(0x9E3779B97F4A7C15) }
    }
    fn next(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }
    /// A float in [0, 1).
    fn unit(&mut self) -> f32 {
        (self.next() % 100_000) as f32 / 100_000.0
    }
}

/// The interactive state of the Act IV Boole logic-interrogation puzzle.
pub struct BoolePuzzle {
    pub logic_grid: Vec<Vec<Gate>>,
    pub input_bits: Vec<u8>,
    pub target_output: Vec<u8>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub state_ppm: f32,
    pub interrogation_monologue: Vec<String>,
    pub mono_idx: usize,
    pub mono_timer: u16,
    pub status_log: Vec<(String, LogKind)>,
    pub solved: bool,
    pub drift_seed: u64,
}

impl BoolePuzzle {
    pub fn new() -> Self {
        let input_bits = vec![1u8, 0, 1, 1, 0, 1];

        // Derive the target from a known-good solution grid, so the puzzle is always
        // solvable: the player starts from a different configuration and must steer
        // the lanes back to this output.
        let solution = vec![
            vec![Gate::Xor; GRID_W],
            vec![Gate::And; GRID_W],
            vec![Gate::Or; GRID_W],
            vec![Gate::Xor; GRID_W],
        ];
        let target_output = eval_pure(&solution, &input_bits);

        // Start grid: a uniform AND lattice (a different output from the target).
        let logic_grid = vec![vec![Gate::And; GRID_W]; GRID_H];

        Self {
            logic_grid,
            input_bits,
            target_output,
            cursor_row: 0,
            cursor_col: 0,
            state_ppm: 0.0,
            interrogation_monologue: vec![
                "INTERROGATOR: Did you introduce Harry to this type of conduct?".to_string(),
                "TURING: I have done nothing wrong. The mathematics is clean.".to_string(),
                "COURT: Section 11. An act of gross indecency with a male person.".to_string(),
                "INTERROGATOR: You will sign the statement. Sign it.".to_string(),
                "TURING: The truth table holds. It must hold. One and one is one.".to_string(),
                "CLINIC: Stilboestrol, fifty milligrams. Continue the dosage.".to_string(),
                "TURING: My hands. I cannot keep my hands from shaking.".to_string(),
                "COURT: Probation, conditional upon organo-therapy treatment.".to_string(),
                "TURING: An apple. The smell of almonds. It clears the mind.".to_string(),
            ],
            mono_idx: 0,
            mono_timer: 60,
            status_log: vec![
                ("Re-derive the checksum. WASD/arrows move \u{00B7} SPACE cycles the gate \u{00B7} R verifies.".to_string(), LogKind::Info),
            ],
            solved: false,
            drift_seed: 0x5715_B007,
        }
    }

    fn push_log(&mut self, msg: String, kind: LogKind) {
        self.status_log.push((msg, kind));
        if self.status_log.len() > 8 {
            self.status_log.remove(0);
        }
    }
}

/// Evaluate the lane network with pure Boolean truth (used for target derivation and
/// the win check). Each row folds lane `j` with its right neighbour (wrapping).
fn eval_pure(grid: &[Vec<Gate>], input: &[u8]) -> Vec<u8> {
    let mut v = input.to_vec();
    for row in grid {
        let w = v.len().max(1);
        let mut nv = vec![0u8; v.len()];
        for j in 0..v.len() {
            let a = v[j];
            let b = v[(j + 1) % w];
            let g = row.get(j).copied().unwrap_or(Gate::Or);
            nv[j] = g.apply(a, b) & 1;
        }
        v = nv;
    }
    v
}

/// Evaluate the network for *display*, optionally letting guilt corrupt the truth:
/// above [`GUILT_PPM`] each gate result has a PPM-scaled chance to flip — the
/// hardware-simulated hallucination that makes the player distrust their axioms.
fn eval_haunted(grid: &[Vec<Gate>], input: &[u8], ppm: f32, seed: u64) -> Vec<u8> {
    let mut v = input.to_vec();
    let mut rng = Lcg::new(seed);
    // Display-only flip probability, capped at 0.30 so the live board lies enough to
    // unsettle the player without becoming unreadable. The *verify* path never calls
    // this — it uses `eval_pure`, so a clean simulation tick is guaranteed every R press.
    let flip_p = if ppm > GUILT_PPM {
        (((ppm - GUILT_PPM) / 50.0) * 0.30).clamp(0.0, 0.30)
    } else {
        0.0
    };
    for row in grid {
        let w = v.len().max(1);
        let mut nv = vec![0u8; v.len()];
        for j in 0..v.len() {
            let a = v[j];
            let b = v[(j + 1) % w];
            let g = row.get(j).copied().unwrap_or(Gate::Or);
            let mut out = g.apply(a, b) & 1;
            if flip_p > 0.0 && rng.unit() < flip_p {
                out ^= 1; // the axiom betrays him
            }
            nv[j] = out;
        }
        v = nv;
    }
    v
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
    if rect.width < 2 || rect.height < 2 { return; }
    let (x, y, w, h) = (rect.x, rect.y, rect.width, rect.height);
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

fn clip(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// Linear blend between two RGB colours (non-RGB passes through).
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

// ═════════════════════════════════════════════════════════════════════════════
// PROCEDURAL DECAY PASSES
// ═════════════════════════════════════════════════════════════════════════════

/// True if `(x, y)` falls inside `protect`. Used to keep the active logic board free
/// of corruption glyphs so gate visibility is never compromised.
fn in_rect(r: Rect, x: u16, y: u16) -> bool {
    x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
}

/// Procedural ink bleed: mutate the blueprint background texture (only the faint grid
/// dots/crosses and empty cells — never live signal glyphs) into corruption glyphs and
/// fragmented court phrases. The static is *masked* to the strict `zone` sub-rect and
/// additionally forbidden inside the protected board, on the `warn_y` row, and on any
/// non-background glyph — so the outer border, the row indices, and the bottom warning/
/// footer bars stay 100% pristine even at maximum glitch intensity.
fn apply_ink_bleed(buf: &mut Buffer, zone: Rect, protect: Rect, warn_y: u16, frame: u64, ppm: f32, seed: u64) {
    if ppm <= TREMOR_PPM {
        return;
    }
    let density = (((ppm - TREMOR_PPM) / 70.0) * 0.20).clamp(0.0, 0.20);
    let mut rng = Lcg::new(seed ^ (frame / 3));

    for y in zone.y..zone.y.saturating_add(zone.height) {
        for x in zone.x..zone.x.saturating_add(zone.width) {
            if !in_bounds(buf, x, y) || in_rect(protect, x, y) || y == warn_y {
                continue;
            }
            // Only corrupt background texture: spaces and the blueprint grid glyphs.
            let sym = buf.get(x, y).symbol().chars().next().unwrap_or(' ');
            if sym != ' ' && sym != '\u{00B7}' && sym != '\u{253C}' {
                continue;
            }
            if rng.unit() < density {
                let g = INK_GLYPHS[(rng.next() as usize) % INK_GLYPHS.len()];
                let shade = lerp_color(DIM_FG, CORRUPT_FG, rng.unit());
                buf_set(buf, x, y, g, Style::default().fg(shade).bg(WS_BG));
            }
        }
    }

    // Occasionally stamp a fragmented court phrase — but only well inside the masked
    // zone, clear of the board, the warning row, and the right edge, so it can never
    // spill across a border or over a structural label.
    if ppm > GUILT_PPM && zone.width > 6 && zone.height > 0 && rng.unit() < 0.5 {
        let frag = COURT_FRAGMENTS[(rng.next() as usize) % COURT_FRAGMENTS.len()];
        let max_w = zone.width.saturating_sub(2) as usize;
        let fy = zone.y + (rng.next() as u16 % zone.height);
        let span = zone.width.saturating_sub(frag.len() as u16 + 2).max(1);
        let fx = zone.x + 1 + (rng.next() as u16 % span);
        if !in_rect(protect, fx, fy) && fy != warn_y {
            buf_set_str(buf, fx, fy, &clip(frag, max_w), Style::default().fg(CORRUPT_FG).bg(WS_BG));
        }
    }
}

/// The police searchlight: above [`GUILT_PPM`] a single flickering beam sweeps
/// horizontally across the board. It is a *diluted charcoal shadow* wash (#242428),
/// not an opaque white-out — the background dims under it while the foreground gate
/// glyphs are only lightly grazed, so the logic board stays readable during the sweep.
fn apply_searchlight(buf: &mut Buffer, area: Rect, frame: u64, ppm: f32) {
    if ppm <= GUILT_PPM || area.width == 0 {
        return;
    }
    let intensity = ((ppm - GUILT_PPM) / 50.0).clamp(0.0, 1.0);
    let period: u64 = 150;
    let phase = (frame % period) as f32 / period as f32;
    let beam_x = area.x as f32 + phase * area.width as f32;
    let beam_w = (area.width as f32 * 0.20).max(3.0);
    // Harsh, unstable flicker.
    let flicker = (0.6 + 0.4 * ((frame as f32 * 0.8).sin())).clamp(0.0, 1.0)
        * if frame % 17 == 0 { 0.4 } else { 1.0 };

    // Sweep the interior only — the one-cell border ring is left untouched so the
    // structural frame never dims or flickers under the beam.
    let y0 = area.y.saturating_add(1);
    let y1 = area.y.saturating_add(area.height).saturating_sub(1);
    let x0 = area.x.saturating_add(1);
    let x1 = area.x.saturating_add(area.width).saturating_sub(1);
    for y in y0..y1 {
        for x in x0..x1 {
            if !in_bounds(buf, x, y) {
                continue;
            }
            let d = (x as f32 - beam_x).abs();
            if d >= beam_w {
                continue;
            }
            // Diluted weight: the beam is a charcoal shadow wash, capped well below
            // opaque so the foreground gate glyphs survive the sweep and stay readable.
            let f = (1.0 - d / beam_w) * intensity * flicker * 0.5;
            if f <= 0.0 {
                continue;
            }
            let cell = buf.get_mut(x, y);
            // Foreground (the gates/bits) is only lightly grazed; the background takes
            // the brunt of the shadow so the lane signals remain visible underneath.
            cell.fg = lerp_color(cell.fg, SEARCHLIGHT, f * 0.35);
            cell.bg = lerp_color(cell.bg, SEARCHLIGHT, f);
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// RENDER — THE LOGIC INTERROGATION (ACT IV)
// ═════════════════════════════════════════════════════════════════════════════

/// Draw the Boole logic board: input lanes, the editable gate matrix, the haunted
/// live output, and the target — under procedural ink bleed and the searchlight.
pub fn render_boole(
    f: &mut Frame,
    area: Rect,
    state: &mut GlobalStateContext,
    puzzle: &BoolePuzzle,
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
    let ppm = state.stilboestrol_ppm;
    let frame = state.frame_counter;

    let inner_x = x0 + 2;
    let lane_x = inner_x + 5;

    // The protected logic-board rect: ink bleed is forbidden here so the lanes, gates,
    // output and target rows always read cleanly. Spans IN → TGT plus a little margin.
    let board_w = (GRID_W as u16 * 3 + 6).min(w.saturating_sub(2));
    let board_h = (GRID_H as u16 + 5).min(h.saturating_sub(3));
    let board = Rect::new(inner_x, y0 + 2, board_w, board_h);

    // The exact row the AXIOM DRIFT / tremor warning prints on (matches the cy walk
    // below). The noise mask skips it explicitly so the warning bar is never corrupted.
    let warn_y = y0 + 2 + 1 + GRID_H as u16 + 1 + 1 + 2;

    // The strict inner zone where ambient static may bleed: inside the border, above the
    // two-row gauge footer band, never on the border ring. Combined with the board
    // protect and the warning-row skip, this guarantees that the borders, the row
    // indices, and the bottom warning/footer bars all remain pristine.
    let noise_zone = Rect::new(x0 + 1, y0 + 1, w.saturating_sub(2), h.saturating_sub(3));

    // 1. Procedural decay of the background BEFORE the signals are drawn.
    apply_ink_bleed(buf, noise_zone, board, warn_y, frame, ppm, puzzle.drift_seed);

    // 2. Border + header.
    draw_box(buf, area, Style::default().fg(BORDER_FG).bg(WS_BG));
    let header = clip(
        " BOOLEAN INTERROGATION \u{2500}\u{2500} G. BOOLE (1854) ",
        w.saturating_sub(4) as usize,
    );
    buf_set_str(buf, x0 + 2, y0, &header, Style::default().fg(HEADER_FG).bg(WS_BG));

    let mut cy = y0 + 2;

    // 3. Input lanes.
    buf_set_str(buf, inner_x, cy, "IN ", Style::default().fg(SECTION_FG).bg(WS_BG));
    for (j, &b) in puzzle.input_bits.iter().enumerate() {
        let cx = lane_x + (j as u16) * 3;
        let col = if b != 0 { BIT_ON } else { BIT_OFF };
        buf_set_str(buf, cx, cy, &format!("{}", b), Style::default().fg(col).bg(WS_BG));
    }
    cy += 1;

    // 4. The editable gate matrix.
    for (r, row) in puzzle.logic_grid.iter().enumerate() {
        if cy >= y0 + h.saturating_sub(6) {
            break;
        }
        buf_set_str(buf, inner_x, cy, &format!("{:>2} ", r), Style::default().fg(DIM_FG).bg(WS_BG));
        for (c, gate) in row.iter().enumerate() {
            let cx = lane_x + (c as u16) * 3;
            let is_cursor = r == puzzle.cursor_row && c == puzzle.cursor_col;
            if is_cursor {
                buf_set_str(buf, cx.saturating_sub(1), cy, &format!("[{}]", gate.glyph()), Style::default().fg(CURSOR_FG).bg(WS_BG));
            } else {
                buf_set(buf, cx, cy, gate.glyph(), Style::default().fg(TEXT_FG).bg(WS_BG));
            }
        }
        cy += 1;
    }

    cy += 1;

    // 5. The haunted live output (this is the lie the player must reason against).
    let live = eval_haunted(&puzzle.logic_grid, &puzzle.input_bits, puzzle.state_ppm, puzzle.drift_seed ^ frame);
    let pure = eval_pure(&puzzle.logic_grid, &puzzle.input_bits);
    buf_set_str(buf, inner_x, cy, "OUT", Style::default().fg(SECTION_FG).bg(WS_BG));
    for (j, &b) in live.iter().enumerate() {
        let cx = lane_x + (j as u16) * 3;
        let matched = pure.get(j) == puzzle.target_output.get(j);
        let col = if matched { OK_FG } else { FAIL_FG };
        buf_set_str(buf, cx, cy, &format!("{}", b), Style::default().fg(col).bg(WS_BG));
    }
    cy += 1;

    // 6. Target checksum.
    buf_set_str(buf, inner_x, cy, "TGT", Style::default().fg(SECTION_FG).bg(WS_BG));
    for (j, &b) in puzzle.target_output.iter().enumerate() {
        let cx = lane_x + (j as u16) * 3;
        buf_set_str(buf, cx, cy, &format!("{}", b), Style::default().fg(TARGET_FG).bg(WS_BG));
    }
    cy += 2;

    // 7. Toxicity / dissonance readout.
    if puzzle.state_ppm > GUILT_PPM {
        let warn = clip("\u{2592} AXIOM DRIFT \u{2014} the truth tables are lying \u{2592}", (w - 4) as usize);
        buf_set_str(buf, inner_x, cy, &warn, Style::default().fg(FAIL_FG).bg(WS_BG));
    } else if puzzle.state_ppm > TREMOR_PPM {
        buf_set_str(buf, inner_x, cy, "\u{2592} tremor rising \u{2014} mind the keys", Style::default().fg(CORRUPT_FG).bg(WS_BG));
    }

    // 8. The searchlight sweep — AFTER the signals, washing them out.
    apply_searchlight(buf, area, frame, ppm);

    // 9. Status footer in the brass gauge.
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

// ═════════════════════════════════════════════════════════════════════════════
// EVENT HANDLER — KEYBOARD INPUT (with the physical tremor)
// ═════════════════════════════════════════════════════════════════════════════

/// Route a key against the Boole board. WASD **or** the arrow keys navigate; `Space`
/// cycles the gate under the cursor (`& → | → ^ → ! → &`) with a crisp mechanical click
/// — the streamlined workflow that replaces Shift-symbol typing; `R` verifies against
/// the target. Direct `& | ^ !` typing is still accepted (subject to the toxicity
/// tremor), but the deterministic Space cycle always keeps the board solvable.
pub fn handle_input(
    key: KeyEvent,
    puzzle: &mut BoolePuzzle,
    state: &mut GlobalStateContext,
    dialogue: &mut DialogueEngine,
    audio: &mut AudioEngine,
) {
    puzzle.state_ppm = state.stilboestrol_ppm;
    if puzzle.solved {
        return;
    }

    match key.code {
        KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
            if puzzle.cursor_row > 0 {
                puzzle.cursor_row -= 1;
            }
            audio.grid_nav();
        }
        KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => {
            if puzzle.cursor_row + 1 < puzzle.logic_grid.len() {
                puzzle.cursor_row += 1;
            }
            audio.grid_nav();
        }
        KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') => {
            if puzzle.cursor_col > 0 {
                puzzle.cursor_col -= 1;
            }
            audio.grid_nav();
        }
        KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') => {
            if puzzle.cursor_col + 1 < GRID_W {
                puzzle.cursor_col += 1;
            }
            audio.grid_nav();
        }
        // Streamlined gate selection: Space steps the cursor cell around the circular
        // gate queue. Deterministic (no tremor), so the puzzle is always solvable.
        KeyCode::Char(' ') => {
            let (r, c) = (puzzle.cursor_row, puzzle.cursor_col);
            if r < puzzle.logic_grid.len() && c < GRID_W {
                puzzle.logic_grid[r][c] = puzzle.logic_grid[r][c].cycle();
                audio.menu_confirm(); // heavy mechanical latch on each gate cycle
                // Memory Echo, binary-locked: the thought of TRUE punches the left ear,
                // FALSE the right — driven by the input bit under the cursor's lane.
                if puzzle.input_bits.get(c).copied().unwrap_or(0) == 1 {
                    audio.play_whisper("whispers/boole_true.mp3", -0.80);
                } else {
                    audio.play_whisper("whispers/boole_false.mp3", 0.80);
                }
            }
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            // A failed verification detonates the electrical short.
            if !verify(puzzle, state, dialogue) {
                audio.glitch();
            }
        }
        // Hidden developer shortcut (demo-recording safeguard): F10 loads the known-good
        // gate solution into every lane and runs the standard verification, cleanly
        // driving the successful transition into the Act Outro. Not surfaced in the
        // on-screen controls — the seed solution matches the derived TGT checksum.
        KeyCode::F(10) => {
            const SOLUTION: [Gate; GRID_H] = [Gate::Xor, Gate::And, Gate::Or, Gate::Xor];
            for (r, row) in puzzle.logic_grid.iter_mut().enumerate() {
                let g = SOLUTION[r % SOLUTION.len()];
                for cell in row.iter_mut() {
                    *cell = g;
                }
            }
            let _ = verify(puzzle, state, dialogue);
        }
        KeyCode::Char(c) => {
            if let Some(intended) = Gate::from_key(c) {
                // A mis-strike (the wrong gate landing) detonates the electrical short.
                if set_gate_with_tremor(puzzle, state, intended) {
                    audio.glitch();
                }
            }
        }
        _ => {}
    }
}

/// Apply a gate to the cursor cell, subject to the hormonal tremor at high toxicity.
/// Returns `true` iff a mis-strike landed the wrong gate (so the caller can detonate
/// the electrical-short SFX). A double-strike cursor slip is *not* a mis-strike.
fn set_gate_with_tremor(puzzle: &mut BoolePuzzle, state: &GlobalStateContext, intended: Gate) -> bool {
    let r = puzzle.cursor_row;
    let c = puzzle.cursor_col;
    if r >= puzzle.logic_grid.len() || c >= GRID_W {
        return false;
    }

    let mut rng = Lcg::new(
        state
            .frame_counter
            .wrapping_add(state.chemical_drift_seed)
            .wrapping_add((r * 31 + c) as u64),
    );

    let tremor_p = if puzzle.state_ppm > TREMOR_PPM {
        (((puzzle.state_ppm - TREMOR_PPM) / 70.0) * 0.5).clamp(0.0, 0.5)
    } else {
        0.0
    };

    // Mis-strike chance is calibrated DOWN 60% from the original 0.55 weight (→ 0.22)
    // so valid gate keys land accurately on the cursor column instead of constantly
    // mutating into foreign tokens. Note a mis-strike only ever picks another *gate*
    // (& | ^ !) — never a board-decoration glyph like § @ %.
    const MISSTRIKE_WEIGHT: f32 = 0.22;
    let mut gate = intended;
    let mut mis_struck = false;
    if tremor_p > 0.0 {
        let roll = rng.unit();
        if roll < tremor_p * MISSTRIKE_WEIGHT {
            // Mis-strike: a neighbouring key landed instead.
            let idx = (rng.next() as usize) % GATE_CYCLE.len();
            let wrong = GATE_CYCLE[idx];
            gate = if wrong == intended {
                GATE_CYCLE[(idx + 1) % GATE_CYCLE.len()]
            } else {
                wrong
            };
            mis_struck = true;
            puzzle.push_log(
                format!("[TREMOR] mis-strike: '{}' landed, not '{}'", gate.glyph(), intended.glyph()),
                LogKind::Warning,
            );
        } else if roll < tremor_p * (MISSTRIKE_WEIGHT + 0.18) {
            // Double-strike: the correct gate still sets on the active column, but the
            // hand then slips one lane over (a recoverable nudge, not a corruption).
            puzzle.logic_grid[r][c] = gate;
            puzzle.cursor_col = (c + 1).min(GRID_W - 1);
            puzzle.push_log("[TREMOR] double-strike: the cursor slipped".to_string(), LogKind::Warning);
            return false;
        }
    }

    puzzle.logic_grid[r][c] = gate;
    mis_struck
}

/// Verify the board with PURE Boolean truth (guilt only haunts the display, never the
/// underlying axiom — so the puzzle stays solvable beneath the hallucination). Returns
/// `true` on a verified checksum, `false` on mismatch (the caller fires the glitch SFX).
fn verify(puzzle: &mut BoolePuzzle, state: &mut GlobalStateContext, dialogue: &mut DialogueEngine) -> bool {
    let out = eval_pure(&puzzle.logic_grid, &puzzle.input_bits);
    if out == puzzle.target_output {
        puzzle.solved = true;
        puzzle.push_log("CHECKSUM VERIFIED. The logic held, beneath everything.".to_string(), LogKind::Success);
        if !state.acts_completed.contains(&Act::Boole1854) {
            state.acts_completed.push(Act::Boole1854);
        }
        dialogue.play(
            Speaker::Turing,
            "[TURING]: \"You see? Two values, a handful of operations \u{2014} and the truth survives, even when they make me doubt my own hands.\"",
            Some(VoiceCue::Victory(Act::Boole1854)),
        );
        state.current_act = Act::Shannon1937;
        true
    } else {
        let matched = out
            .iter()
            .zip(puzzle.target_output.iter())
            .filter(|(a, b)| a == b)
            .count();
        puzzle.push_log(
            format!("Checksum mismatch: {}/{} lanes. Re-derive the gates.", matched, puzzle.target_output.len()),
            LogKind::Error,
        );
        false
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// TICK — INTERROGATION MONOLOGUE + CHEMICAL RAMP
// ═════════════════════════════════════════════════════════════════════════════

/// Per-frame update: ramp the chemical toxicity (the trial's rising dosage) and feed
/// fragmented interrogation lines into the upper mind log char-by-char, each one
/// announced by a heavy police-bootstep SFX cue.
pub fn tick_boole(puzzle: &mut BoolePuzzle, state: &mut GlobalStateContext, dialogue: &mut DialogueEngine) {
    puzzle.state_ppm = state.stilboestrol_ppm;
    if puzzle.solved {
        return;
    }

    // The dosage climbs through the act — driving tremor, guilt, ink bleed, light.
    state.stilboestrol_ppm = (state.stilboestrol_ppm + 0.04).min(95.0);
    puzzle.state_ppm = state.stilboestrol_ppm;

    // Feed the next fragment only when the typewriter is idle, so lines never overlap.
    if dialogue.is_active() && dialogue.is_typing() {
        return;
    }
    if puzzle.mono_timer > 0 {
        puzzle.mono_timer -= 1;
        return;
    }
    if puzzle.interrogation_monologue.is_empty() {
        return;
    }
    let line = puzzle.interrogation_monologue[puzzle.mono_idx % puzzle.interrogation_monologue.len()].clone();
    puzzle.mono_idx += 1;
    puzzle.mono_timer = MONO_GAP;
    dialogue.play(Speaker::System, &line, Some(VoiceCue::PoliceBootstep));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_is_solvable_with_the_seed_solution() {
        let p = BoolePuzzle::new();
        let solution = vec![
            vec![Gate::Xor; GRID_W],
            vec![Gate::And; GRID_W],
            vec![Gate::Or; GRID_W],
            vec![Gate::Xor; GRID_W],
        ];
        assert_eq!(eval_pure(&solution, &p.input_bits), p.target_output);
    }

    #[test]
    fn start_grid_is_not_already_solved() {
        let p = BoolePuzzle::new();
        assert_ne!(eval_pure(&p.logic_grid, &p.input_bits), p.target_output);
    }

    #[test]
    fn pure_eval_is_deterministic_and_guilt_free() {
        let p = BoolePuzzle::new();
        let a = eval_pure(&p.logic_grid, &p.input_bits);
        let b = eval_pure(&p.logic_grid, &p.input_bits);
        assert_eq!(a, b);
        // Below the guilt threshold, the haunted eval matches the pure one.
        assert_eq!(eval_haunted(&p.logic_grid, &p.input_bits, 0.0, 1), a);
    }

    #[test]
    fn space_cycle_can_solve_the_board() {
        // The streamlined Space workflow steps each cell & → | → ^ → ! → &. From the
        // uniform-AND start grid, cycling every cell to the seed solution's gate must
        // reproduce the target checksum — i.e. the level stays solvable with Space alone.
        let p = BoolePuzzle::new();
        let want = [Gate::Xor, Gate::And, Gate::Or, Gate::Xor]; // per-row solution gate
        let mut grid = p.logic_grid.clone();
        for (r, row) in grid.iter_mut().enumerate() {
            for cell in row.iter_mut() {
                let mut guard = 0;
                while *cell != want[r] && guard < 8 {
                    *cell = cell.cycle();
                    guard += 1;
                }
            }
        }
        assert_eq!(eval_pure(&grid, &p.input_bits), p.target_output);
    }

    #[test]
    fn gate_cycle_is_a_closed_four_step_loop() {
        let mut g = Gate::And;
        for _ in 0..4 {
            g = g.cycle();
        }
        assert_eq!(g, Gate::And);
    }

    #[test]
    fn gates_apply_correctly() {
        assert_eq!(Gate::And.apply(1, 1), 1);
        assert_eq!(Gate::And.apply(1, 0), 0);
        assert_eq!(Gate::Or.apply(0, 1), 1);
        assert_eq!(Gate::Xor.apply(1, 1), 0);
        assert_eq!(Gate::Not.apply(1, 0), 0);
        assert_eq!(Gate::Not.apply(0, 1), 1);
    }
}


