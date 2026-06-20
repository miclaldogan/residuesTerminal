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
// TrueColor palette — amber phosphor, shared with the rest of the engine
// ─────────────────────────────────────────────────────────────────────────────
const WS_BG:     Color = Color::Rgb(10, 8, 0);
const BORDER_FG: Color = Color::Rgb(120, 90, 20);
const HEADER_FG: Color = Color::Rgb(255, 213, 102);
const TEXT_FG:   Color = Color::Rgb(255, 176, 0);
const DIM_FG:    Color = Color::Rgb(74, 50, 5);
const ACCENT:    Color = Color::Rgb(153, 104, 10);
const LOCK_FG:   Color = Color::Rgb(200, 100, 20);
const ERR_FG:    Color = Color::Rgb(255, 102, 51);
const OK_FG:     Color = Color::Rgb(255, 213, 102);
const WARN_FG:   Color = Color::Rgb(255, 176, 0);
const INFO_FG:   Color = Color::Rgb(170, 150, 110);
const REG_FILL:  Color = Color::Rgb(255, 176, 0);
const REG_EMPTY: Color = Color::Rgb(42, 30, 2);
const REG_BLINK: Color = Color::Rgb(255, 150, 40);

// ─────────────────────────────────────────────────────────────────────────────
// Micro-architecture constants
// ─────────────────────────────────────────────────────────────────────────────
/// The 3-register computational ceiling. A reference to a 4th register aborts.
const REGISTER_COUNT: usize = 3;
/// Phase-1 linear core size: the 5th instruction triggers the memory overrun.
const LINEAR_CORE_CEILING: usize = 4;
/// Runaway guard — execution beyond this many steps scorches the candle.
const STEP_BURN_THRESHOLD: usize = 500;
/// The targeted Bernoulli matrix-series checksum the program must accumulate in R3
/// (7+6+5+4+3+2+1 — a triangular reduction that can only be reached by a loop that
/// devours its own counter, never by a single LOAD).
const BERNOULLI_CHECKSUM: i64 = 28;

// ═════════════════════════════════════════════════════════════════════════════
// DATA STRUCTURES
// ═════════════════════════════════════════════════════════════════════════════

/// The 2-state constraint funnel for the Analytical Engine programming puzzle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LovelacePhase {
    /// Phase 1 — branch ops locked; straight-line code only, capped at 4 instructions.
    LinearFlow,
    /// Phase 2 — branch ops (`IF_Z`, `JMP`) unlocked; loops are required.
    LoopBranch,
}

/// Severity classification for the workspace status log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogKind {
    Info,
    Error,
    Warning,
    Success,
}

/// A decoded instruction. Register operands are pre-validated to 0..REGISTER_COUNT
/// and jump targets to 0..program length, so the executor needs no bounds branches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Instr {
    Load(usize, i64),   // LOAD  r, v   → r = v
    Store(usize, usize),// STORE s, d   → d = s
    Add(usize, usize),  // ADD   r, s   → r += s
    Sub(usize, usize),  // SUB   r, s   → r -= s
    IfZ(usize, usize),  // IF_Z  r, l   → if r == 0 { pc = l }
    Jmp(usize),         // JMP   l      → pc = l
    Halt,               // HALT         → terminate cleanly
}

/// A compilation failure tied to the offending source line for in-editor highlight.
#[derive(Debug)]
struct CompileError {
    line: usize,
    code: &'static str,
    msg: String,
}

/// Why a register token was rejected during parsing.
enum RegErr {
    /// Referenced a register beyond the 3-slot ceiling (human R-number carried back).
    Ceiling(i64),
    Malformed,
}

/// Outcome of a single synchronous execution pass.
enum RunOutcome {
    Halted,
    Runaway,
}

/// The complete interactive state for the Act III Lovelace assembly puzzle.
pub struct LovelacePuzzle {
    pub source: Vec<String>,
    pub cursor_line: usize,
    pub editing: bool,
    pub edit_buffer: String,
    pub phase: LovelacePhase,
    pub registers: [i64; REGISTER_COUNT],
    pub last_steps: usize,
    pub status_log: Vec<(String, LogKind)>,
    pub error_line: Option<usize>,
    pub solved: bool,
    /// Set after a runaway loop — drives the high-frequency register-glyph thrash.
    pub thrashing: bool,
    /// Set when Phase 1 overruns its 4-instruction core, pending player acknowledgment.
    pub crashed_overrun: bool,
}

impl LovelacePuzzle {
    pub fn new() -> Self {
        Self {
            source: vec![
                "LOAD 0, 7".to_string(),
                "LOAD 1, 1".to_string(),
                "LOAD 2, 0".to_string(),
            ],
            cursor_line: 0,
            editing: false,
            edit_buffer: String::new(),
            phase: LovelacePhase::LinearFlow,
            registers: [0; REGISTER_COUNT],
            last_steps: 0,
            status_log: vec![
                ("Accumulate the Bernoulli checksum into R3. Press R to compile.".to_string(), LogKind::Info),
                ("PHASE 1: branch ops locked. Linear core capped at 4 instructions.".to_string(), LogKind::Warning),
            ],
            error_line: None,
            solved: false,
            thrashing: false,
            crashed_overrun: false,
        }
    }

    /// Push a bounded status message (max 8 entries retained).
    fn push_log(&mut self, msg: String, kind: LogKind) {
        self.status_log.push((msg, kind));
        if self.status_log.len() > 8 {
            self.status_log.remove(0);
        }
    }

    /// Parse and validate the editor source into executable bytecode. Enforces the
    /// register ceiling, the phase branch-lock, jump-target bounds, and the Phase-1
    /// linear-core overrun. Blank lines are skipped and do not consume an index.
    fn compile(&self) -> Result<(Vec<Instr>, Vec<usize>), CompileError> {
        let mut instr: Vec<Instr> = Vec::new();
        let mut src_lines: Vec<usize> = Vec::new();

        for (sidx, raw) in self.source.iter().enumerate() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            // Normalise commas to spaces, then tokenise.
            let normalised: String = line.chars().map(|c| if c == ',' { ' ' } else { c }).collect();
            let toks: Vec<&str> = normalised.split_whitespace().collect();
            if toks.is_empty() {
                continue;
            }
            let op = toks[0].to_ascii_uppercase();

            let decoded = match op.as_str() {
                "LOAD" => {
                    expect_args(&toks, 2, sidx, "LOAD r, v")?;
                    Instr::Load(reg_arg(toks[1], sidx)?, int_arg(toks[2], sidx)?)
                }
                "STORE" => {
                    expect_args(&toks, 2, sidx, "STORE s, d")?;
                    Instr::Store(reg_arg(toks[1], sidx)?, reg_arg(toks[2], sidx)?)
                }
                "ADD" => {
                    expect_args(&toks, 2, sidx, "ADD r, s")?;
                    Instr::Add(reg_arg(toks[1], sidx)?, reg_arg(toks[2], sidx)?)
                }
                "SUB" => {
                    expect_args(&toks, 2, sidx, "SUB r, s")?;
                    Instr::Sub(reg_arg(toks[1], sidx)?, reg_arg(toks[2], sidx)?)
                }
                "IF_Z" => {
                    if self.phase == LovelacePhase::LinearFlow {
                        return Err(branch_locked(sidx, "IF_Z"));
                    }
                    expect_args(&toks, 2, sidx, "IF_Z r, l")?;
                    Instr::IfZ(reg_arg(toks[1], sidx)?, line_arg(toks[2], sidx)?)
                }
                "JMP" => {
                    if self.phase == LovelacePhase::LinearFlow {
                        return Err(branch_locked(sidx, "JMP"));
                    }
                    expect_args(&toks, 1, sidx, "JMP l")?;
                    Instr::Jmp(line_arg(toks[1], sidx)?)
                }
                "HALT" => {
                    expect_args(&toks, 0, sidx, "HALT")?;
                    Instr::Halt
                }
                _ => {
                    return Err(CompileError {
                        line: sidx,
                        code: "ERR_OPCODE",
                        msg: format!("unknown instruction '{}'", toks[0]),
                    });
                }
            };

            instr.push(decoded);
            src_lines.push(sidx);
        }

        if instr.is_empty() {
            return Err(CompileError {
                line: 0,
                code: "ERR_EMPTY",
                msg: "no instructions to compile".to_string(),
            });
        }

        // Jump-target bounds: every IF_Z/JMP must land inside the program.
        for (k, ins) in instr.iter().enumerate() {
            let target = match ins {
                Instr::IfZ(_, l) => Some(*l),
                Instr::Jmp(l) => Some(*l),
                _ => None,
            };
            if let Some(l) = target {
                if l >= instr.len() {
                    return Err(CompileError {
                        line: src_lines[k],
                        code: "ERR_BAD_JUMP",
                        msg: format!("jump target {} is outside the program (0..{})", l, instr.len()),
                    });
                }
            }
        }

        // The Memory-Ceiling overrun: Phase 1 cannot exceed its linear core. This is
        // the intentional crash that funnels the player into unlocking Phase 2.
        if self.phase == LovelacePhase::LinearFlow && instr.len() > LINEAR_CORE_CEILING {
            let line = *src_lines.get(LINEAR_CORE_CEILING).unwrap_or_else(|| src_lines.last().unwrap_or(&0));
            return Err(CompileError {
                line,
                code: "ERR_MEM_OVERRUN",
                msg: format!("linear flow exceeds the {}-instruction core ceiling", LINEAR_CORE_CEILING),
            });
        }

        Ok((instr, src_lines))
    }

    /// Compile + execute the program (triggered by the `R` key). Mutates the live
    /// register view, logs the outcome, and applies world effects (candle burn on a
    /// runaway loop, act completion on a verified checksum).
    fn run_program(&mut self, state: &mut GlobalStateContext, dialogue: &mut DialogueEngine) {
        self.error_line = None;
        self.thrashing = false;

        let (instr, _src_lines) = match self.compile() {
            Ok(v) => v,
            Err(e) => {
                self.error_line = Some(e.line);
                self.push_log(format!("[{}] {}", e.code, e.msg), LogKind::Error);
                if e.code == "ERR_MEM_OVERRUN" {
                    self.crashed_overrun = true;
                }
                return;
            }
        };

        let (outcome, regs, steps) = execute(&instr);
        self.registers = regs;
        self.last_steps = steps;

        match outcome {
            RunOutcome::Runaway => {
                // The Infinite-Loop Burn: accelerate candle decay and thrash the glyphs.
                state.melt_candle(10);
                self.thrashing = true;
                self.push_log(
                    format!("[ERR_INF_LOOP] runaway: >{} steps, no HALT — candle scorched.", STEP_BURN_THRESHOLD),
                    LogKind::Error,
                );
            }
            RunOutcome::Halted => {
                let looped = steps > instr.len();
                if self.phase == LovelacePhase::LoopBranch && regs[2] == BERNOULLI_CHECKSUM && looped {
                    self.solved = true;
                    self.push_log(
                        format!("VERIFIED: R3 = {} in {} steps. The series is woven.", BERNOULLI_CHECKSUM, steps),
                        LogKind::Success,
                    );
                    if !state.acts_completed.contains(&Act::Lovelace1843) {
                        state.acts_completed.push(Act::Lovelace1843);
                    }
                    dialogue.play(
                        Speaker::Turing,
                        "[TURING]: \"Ada was right. The engine does not merely calculate \u{2014} it reasons in loops, devouring its own counters to weave a series it was never handed.\"",
                        Some(VoiceCue::Victory(Act::Lovelace1843)),
                    );
                    state.current_act = Act::Boole1854;
                } else if regs[2] == BERNOULLI_CHECKSUM && !looped {
                    self.push_log(
                        "R3 matches, but no loop ran — the series must be iterated, not loaded.".to_string(),
                        LogKind::Warning,
                    );
                } else {
                    self.push_log(
                        format!("HALT after {} steps. R3 = {} (target {}).", steps, regs[2], BERNOULLI_CHECKSUM),
                        LogKind::Info,
                    );
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Parsing helpers (free functions so `?` propagates cleanly out of `compile`)
// ─────────────────────────────────────────────────────────────────────────────

fn expect_args(toks: &[&str], n: usize, line: usize, usage: &str) -> Result<(), CompileError> {
    if toks.len() != n + 1 {
        Err(CompileError {
            line,
            code: "ERR_SYNTAX",
            msg: format!("usage: {}", usage),
        })
    } else {
        Ok(())
    }
}

fn branch_locked(line: usize, op: &str) -> CompileError {
    CompileError {
        line,
        code: "ERR_BRANCH_LOCKED",
        msg: format!("{} is locked in Phase 1 LinearFlow", op),
    }
}

/// Resolve a register token to a 0-based index, enforcing the 3-register ceiling.
/// Accepts both `R1`/`R2`/`R3` and bare `0`/`1`/`2` (R1 == 0). `R4` or `3` abort.
fn parse_reg(tok: &str) -> Result<usize, RegErr> {
    let t = tok.trim();
    if t.is_empty() {
        return Err(RegErr::Malformed);
    }
    let (is_r, num_str) = match t.strip_prefix(['R', 'r']) {
        Some(rest) => (true, rest),
        None => (false, t),
    };
    let n: i64 = num_str.parse().map_err(|_| RegErr::Malformed)?;
    let index: i64 = if is_r {
        if n < 1 {
            return Err(RegErr::Malformed); // R0 is not a register
        }
        n - 1
    } else {
        n
    };
    if index < 0 {
        return Err(RegErr::Malformed);
    }
    if index as usize >= REGISTER_COUNT {
        return Err(RegErr::Ceiling(index + 1));
    }
    Ok(index as usize)
}

fn reg_arg(tok: &str, line: usize) -> Result<usize, CompileError> {
    match parse_reg(tok) {
        Ok(v) => Ok(v),
        Err(RegErr::Ceiling(n)) => Err(CompileError {
            line,
            code: "ERR_MEM_CEILING",
            msg: format!("R{} exceeds the 3-register ceiling \u{2014} compilation aborted", n),
        }),
        Err(RegErr::Malformed) => Err(CompileError {
            line,
            code: "ERR_BAD_REG",
            msg: format!("invalid register operand '{}'", tok),
        }),
    }
}

fn int_arg(tok: &str, line: usize) -> Result<i64, CompileError> {
    tok.trim().parse::<i64>().map_err(|_| CompileError {
        line,
        code: "ERR_BAD_INT",
        msg: format!("invalid integer literal '{}'", tok),
    })
}

fn line_arg(tok: &str, line: usize) -> Result<usize, CompileError> {
    tok.trim().parse::<usize>().map_err(|_| CompileError {
        line,
        code: "ERR_BAD_TARGET",
        msg: format!("invalid jump target '{}'", tok),
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// The interpreter core
// ─────────────────────────────────────────────────────────────────────────────

/// Execute validated bytecode. All register indices and jump targets were bounds-
/// checked at compile time, and arithmetic wraps, so this loop cannot panic. The
/// only escape hatches are HALT, running off the end, or the runaway step guard.
fn execute(instr: &[Instr]) -> (RunOutcome, [i64; REGISTER_COUNT], usize) {
    let mut regs = [0i64; REGISTER_COUNT];
    let mut pc: usize = 0;
    let mut steps: usize = 0;

    loop {
        if pc >= instr.len() {
            return (RunOutcome::Halted, regs, steps); // ran off the end → clean stop
        }
        steps += 1;
        if steps > STEP_BURN_THRESHOLD {
            return (RunOutcome::Runaway, regs, steps);
        }
        match instr[pc] {
            Instr::Load(r, v) => {
                regs[r] = v;
                pc += 1;
            }
            Instr::Store(s, d) => {
                regs[d] = regs[s];
                pc += 1;
            }
            Instr::Add(r, s) => {
                regs[r] = regs[r].wrapping_add(regs[s]);
                pc += 1;
            }
            Instr::Sub(r, s) => {
                regs[r] = regs[r].wrapping_sub(regs[s]);
                pc += 1;
            }
            Instr::IfZ(r, l) => {
                if regs[r] == 0 {
                    pc = l;
                } else {
                    pc += 1;
                }
            }
            Instr::Jmp(l) => {
                pc = l;
            }
            Instr::Halt => {
                return (RunOutcome::Halted, regs, steps);
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
        col = col.saturating_add(1);
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

/// Truncate to at most `max` characters so a long source line cannot bleed into the
/// register column.
fn clip(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// Transpose one adjacent (non-space) character pair, deterministically chosen by
/// `seed`. Models the hand tremor scrambling a committed editor line.
fn swap_adjacent(s: &str, seed: u64) -> String {
    let mut chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    if n < 2 {
        return s.to_string();
    }
    let start = (seed as usize) % (n - 1);
    for k in 0..(n - 1) {
        let i = (start + k) % (n - 1);
        if chars[i] != ' ' && chars[i + 1] != ' ' {
            chars.swap(i, i + 1);
            break;
        }
    }
    chars.into_iter().collect()
}

fn render_centered_overlay(buf: &mut Buffer, area: Rect, frame_bg: Color, border: Color, lines: &[(&str, Color)]) {
    let overlay_h: u16 = 9;
    let overlay_w = area.width.saturating_sub(4);
    if overlay_w < 6 { return; }
    let ox = area.x + 2;
    let oy = area.y + (area.height / 2).saturating_sub(4);
    let rect = Rect::new(ox, oy, overlay_w, overlay_h);
    buf_fill_bg(buf, rect, frame_bg);
    draw_box(buf, rect, Style::default().fg(border).bg(frame_bg));

    let cx = ox + overlay_w / 2;
    let mut ly = oy + 2;
    for (text, color) in lines {
        let half = text.chars().count() as u16 / 2;
        buf_set_str(buf, cx.saturating_sub(half), ly, text, Style::default().fg(*color).bg(frame_bg));
        ly += 1;
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// RENDER — THE ANALYTICAL ENGINE (ACT III)
// ═════════════════════════════════════════════════════════════════════════════

/// Draw the assembly editor + the three animated register columns into the center
/// panel. Mirrors the workspace convention: blueprint grid base, ambient render
/// guard, tension-free amber box, and a brass gauge footer carrying the status log.
pub fn render_lovelace(
    f: &mut Frame,
    area: Rect,
    state: &mut GlobalStateContext,
    puzzle: &LovelacePuzzle,
) {
    let buf = f.buffer_mut();
    layout::draw_workspace_grid(buf, area, WS_BG);

    // Ambient render guard — the engine is dark and uncalculated until taken up.
    if state.screen_state == ScreenState::AmbientDesk {
        layout::draw_gauge_footer(buf, area, "", DIM_FG);
        return;
    }

    let (x0, y0, w, h) = (area.x, area.y, area.width, area.height);
    if w < 24 || h < 12 { return; }

    draw_box(buf, area, Style::default().fg(BORDER_FG).bg(WS_BG));

    let (phase_name, banner, banner_col) = match puzzle.phase {
        LovelacePhase::LinearFlow => (
            "PHASE 1: LINEARFLOW",
            "BRANCH OPS LOCKED \u{2014} IF_Z / JMP disabled",
            LOCK_FG,
        ),
        LovelacePhase::LoopBranch => (
            "PHASE 2: LOOPBRANCH",
            "BRANCH OPS UNLOCKED \u{2014} loops enabled",
            ACCENT,
        ),
    };
    // Clip the title so it can never overrun and eat the top-right corner glyph.
    let header = clip(
        &format!(" ANALYTICAL ENGINE \u{2500}\u{2500} {} (1843) ", phase_name),
        w.saturating_sub(4) as usize,
    );
    buf_set_str(buf, x0 + 2, y0, &header, Style::default().fg(HEADER_FG).bg(WS_BG));

    // ── Terminal states draw a full overlay and return (no editor underneath). ──
    if puzzle.crashed_overrun {
        render_centered_overlay(
            buf, area, Color::Rgb(24, 10, 0), ERR_FG,
            &[
                ("[ERR_MEM_OVERRUN]", ERR_FG),
                ("Linear core exceeds 4 instructions.", Color::Rgb(220, 120, 40)),
                ("The 3-register engine cannot unroll this.", Color::Rgb(220, 120, 40)),
                ("", WS_BG),
                ("[ any key: unlock PHASE 2 LoopBranch ]", DIM_FG),
            ],
        );
        return;
    }
    if puzzle.solved {
        render_centered_overlay(
            buf, area, Color::Rgb(15, 12, 0), Color::Rgb(92, 68, 0),
            &[
                ("THE SERIES IS WOVEN", OK_FG),
                ("R3 holds the Bernoulli checksum.", TEXT_FG),
                ("The loop computed what it was never told.", TEXT_FG),
                ("", WS_BG),
                ("Advancing to Act IV: Boole...", DIM_FG),
            ],
        );
        return;
    }

    buf_set_str(buf, x0 + 2, y0 + 1, banner, Style::default().fg(banner_col).bg(WS_BG));

    // ── Geometry: editor on the left, three register columns on the right. ──
    let reg_stride: u16 = 4;
    let reg_block_w: u16 = reg_stride * 3;
    let reg_x0 = x0 + w.saturating_sub(reg_block_w + 2);
    let editor_x = x0 + 2;
    let editor_w = reg_x0.saturating_sub(editor_x + 1).max(1) as usize;

    let ed_top = y0 + 3;
    let ed_bottom = y0 + h.saturating_sub(2); // leave the bottom two rows for the gauge

    // Captions
    buf_set_str(buf, editor_x, y0 + 2, &clip("PROG \u{2502} Enter:edit A:add D:del R:run", editor_w), Style::default().fg(DIM_FG).bg(WS_BG));
    buf_set_str(buf, reg_x0, y0 + 2, "REGS", Style::default().fg(DIM_FG).bg(WS_BG));

    // ── Editor: instruction-indexed source with a live edit cursor. ──
    // Precompute the compiled index of each source line (blank lines carry none).
    let mut idxs: Vec<Option<usize>> = Vec::with_capacity(puzzle.source.len());
    let mut counter = 0usize;
    for raw in &puzzle.source {
        if raw.trim().is_empty() {
            idxs.push(None);
        } else {
            idxs.push(Some(counter));
            counter += 1;
        }
    }

    let visible_rows = ed_bottom.saturating_sub(ed_top) as usize;
    let start = if visible_rows > 0 && puzzle.cursor_line >= visible_rows {
        puzzle.cursor_line - visible_rows + 1
    } else {
        0
    };

    let mut ry = ed_top;
    for idx in start..puzzle.source.len() {
        if ry >= ed_bottom { break; }
        let prefix = match idxs[idx] {
            Some(n) => format!("{:>2}\u{2502}", n),
            None => "  \u{2502}".to_string(),
        };
        let selected = idx == puzzle.cursor_line;
        let content = if puzzle.editing && selected {
            format!("{}\u{2588}", puzzle.edit_buffer)
        } else {
            puzzle.source[idx].clone()
        };
        let marker = if selected { "\u{25B8}" } else { " " };
        let color = if puzzle.error_line == Some(idx) {
            ERR_FG
        } else if selected {
            HEADER_FG
        } else {
            TEXT_FG
        };
        let line = clip(&format!("{}{} {}", marker, prefix, content), editor_w);
        buf_set_str(buf, editor_x, ry, &line, Style::default().fg(color).bg(WS_BG));
        ry += 1;
    }

    // ── The three registers as vertical animated data columns. ──
    draw_registers(buf, reg_x0, ed_top, ed_bottom, reg_stride, puzzle, state.frame_counter);

    // ── Status footer in the brass gauge. ──
    if let Some((msg, kind)) = puzzle.status_log.last() {
        let col = match kind {
            LogKind::Info => INFO_FG,
            LogKind::Error => ERR_FG,
            LogKind::Warning => WARN_FG,
            LogKind::Success => OK_FG,
        };
        layout::draw_gauge_footer(buf, area, msg, col);
    }
}

/// Render R1/R2/R3 as bottom-anchored bar columns whose height tracks the register
/// magnitude, with a travelling shimmer highlight; under a runaway loop they blink
/// at high frequency (the thrash).
fn draw_registers(
    buf: &mut Buffer,
    reg_x0: u16,
    top: u16,
    bottom: u16,
    stride: u16,
    puzzle: &LovelacePuzzle,
    frame: u64,
) {
    let blink = puzzle.thrashing && (frame / 2) % 2 == 0;

    for i in 0..REGISTER_COUNT {
        let rx = reg_x0 + i as u16 * stride;
        buf_set_str(buf, rx, top, &format!("R{}", i + 1), Style::default().fg(ACCENT).bg(WS_BG));

        let bar_top = top + 1;
        let val_row = bottom.saturating_sub(1);
        if val_row <= bar_top {
            continue;
        }
        let bar_h = val_row - bar_top;

        let val = puzzle.registers[i];
        let ratio = (val.unsigned_abs() as f32 / 16.0).min(1.0);
        let fill = (ratio * bar_h as f32).round() as u16;

        for row in 0..bar_h {
            let y = val_row.saturating_sub(1 + row);
            if y < bar_top { break; }
            let filled = row < fill;
            let (glyph, color) = if filled {
                let shimmer = ((frame / 4) % bar_h as u64) as u16 == row;
                let c = if blink {
                    REG_BLINK
                } else if shimmer {
                    HEADER_FG
                } else {
                    REG_FILL
                };
                ('\u{2588}', c)
            } else {
                let c = if blink { Color::Rgb(120, 40, 10) } else { REG_EMPTY };
                ('\u{2591}', c)
            };
            buf_set_str(buf, rx, y, &format!("{0}{0}{0}", glyph), Style::default().fg(color).bg(WS_BG));
        }

        let vstr = clip(&format!("{}", val), stride as usize);
        let vcol = if puzzle.thrashing { ERR_FG } else { TEXT_FG };
        buf_set_str(buf, rx, val_row, &vstr, Style::default().fg(vcol).bg(WS_BG));
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// EVENT HANDLER — KEYBOARD INPUT
// ═════════════════════════════════════════════════════════════════════════════

/// Route a key event against the Lovelace puzzle. In edit mode, printable keys feed
/// the active line; otherwise arrows navigate, A/D add/delete lines, Enter toggles
/// edit, and R triggers the compilation pass.
pub fn handle_input(
    key: KeyEvent,
    puzzle: &mut LovelacePuzzle,
    state: &mut GlobalStateContext,
    dialogue: &mut DialogueEngine,
    audio: &mut AudioEngine,
) {
    // Completed act — frozen.
    if puzzle.solved {
        return;
    }

    // Acknowledge the Phase-1 overrun crash → unlock Phase 2.
    if puzzle.crashed_overrun {
        puzzle.crashed_overrun = false;
        puzzle.phase = LovelacePhase::LoopBranch;
        puzzle.error_line = None;
        puzzle.push_log(
            "PHASE 2: LoopBranch unlocked \u{2014} IF_Z / JMP available. Write the loop.".to_string(),
            LogKind::Success,
        );
        return;
    }

    // Any key clears the transient runaway thrash.
    puzzle.thrashing = false;

    // ── Edit mode: typed characters compose the current line. ──
    if puzzle.editing {
        match key.code {
            KeyCode::Enter => {
                if puzzle.cursor_line < puzzle.source.len() {
                    // Act III hand tremor: a rising chance the committed line lands with
                    // an adjacent character pair transposed (LOAD → LDOA). It is always
                    // correctable with Backspace, never a permanent corruption.
                    let chance = state.lovelace_input_swap_chance();
                    let mut committed = puzzle.edit_buffer.clone();
                    if chance > 0.0 {
                        let seed = state
                            .frame_counter
                            .wrapping_add(state.chemical_drift_seed)
                            .wrapping_add(puzzle.cursor_line as u64);
                        let roll = (seed % 1000) as f32 / 1000.0;
                        if roll < chance {
                            committed = swap_adjacent(&committed, seed);
                            puzzle.push_log(
                                "[TREMOR] the line committed scrambled \u{2014} Backspace to fix.".to_string(),
                                LogKind::Warning,
                            );
                        }
                    }
                    puzzle.source[puzzle.cursor_line] = committed;
                }
                puzzle.editing = false;
            }
            KeyCode::Backspace => {
                // The heavy mechanical snap of a correction (whether or not it removed
                // a glyph — the player is striking the key to fix a fumble).
                puzzle.edit_buffer.pop();
                audio.backspace_snap();
            }
            KeyCode::Char(c) => {
                if !c.is_control() && puzzle.edit_buffer.chars().count() < 40 {
                    puzzle.edit_buffer.push(c);
                    audio.daktilo_strike(); // a deliberate key drop on the platen
                }
            }
            _ => {}
        }
        return;
    }

    // ── Navigation / command mode. (W/S mirror Up/Down; A/D remain add/delete-line,
    //    so they keep their editor meaning in this command mode.) ──
    match key.code {
        KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
            if puzzle.cursor_line > 0 {
                puzzle.cursor_line -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => {
            if puzzle.cursor_line + 1 < puzzle.source.len() {
                puzzle.cursor_line += 1;
            }
        }
        KeyCode::Enter => {
            puzzle.editing = true;
            puzzle.edit_buffer = puzzle.source.get(puzzle.cursor_line).cloned().unwrap_or_default();
        }
        KeyCode::Char('a') | KeyCode::Char('A') => {
            let at = (puzzle.cursor_line + 1).min(puzzle.source.len());
            puzzle.source.insert(at, String::new());
            puzzle.cursor_line = at;
            puzzle.editing = true;
            puzzle.edit_buffer.clear();
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if puzzle.source.len() > 1 {
                let i = puzzle.cursor_line.min(puzzle.source.len() - 1);
                puzzle.source.remove(i);
                if puzzle.cursor_line >= puzzle.source.len() {
                    puzzle.cursor_line = puzzle.source.len() - 1;
                }
            } else {
                puzzle.push_log("Cannot delete the last line.".to_string(), LogKind::Warning);
            }
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            puzzle.run_program(state, dialogue);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn puzzle_with(phase: LovelacePhase, src: &[&str]) -> LovelacePuzzle {
        let mut p = LovelacePuzzle::new();
        p.phase = phase;
        p.source = src.iter().map(|s| s.to_string()).collect();
        p
    }

    #[test]
    fn parse_reg_accepts_both_forms_and_enforces_ceiling() {
        assert_eq!(parse_reg("0").ok(), Some(0));
        assert_eq!(parse_reg("R1").ok(), Some(0));
        assert_eq!(parse_reg("2").ok(), Some(2));
        assert_eq!(parse_reg("R3").ok(), Some(2));
        assert!(matches!(parse_reg("R4"), Err(RegErr::Ceiling(4))));
        assert!(matches!(parse_reg("3"), Err(RegErr::Ceiling(4))));
        assert!(matches!(parse_reg("R0"), Err(RegErr::Malformed)));
        assert!(matches!(parse_reg("x"), Err(RegErr::Malformed)));
    }

    #[test]
    fn winning_loop_computes_bernoulli_checksum() {
        let p = puzzle_with(
            LovelacePhase::LoopBranch,
            &["LOAD 0, 7", "LOAD 1, 1", "LOAD 2, 0", "ADD 2, 0", "SUB 0, 1", "IF_Z 0, 7", "JMP 3", "HALT"],
        );
        let (instr, _) = p.compile().expect("should compile in Phase 2");
        let (outcome, regs, steps) = execute(&instr);
        assert!(matches!(outcome, RunOutcome::Halted));
        assert_eq!(regs[2], BERNOULLI_CHECKSUM);
        assert!(steps > instr.len(), "a loop must execute more steps than instructions");
    }

    #[test]
    fn branch_ops_locked_in_phase_one() {
        let p = puzzle_with(LovelacePhase::LinearFlow, &["LOAD 0, 1", "JMP 0"]);
        let err = p.compile().unwrap_err();
        assert_eq!(err.code, "ERR_BRANCH_LOCKED");
    }

    #[test]
    fn fourth_register_aborts_compilation() {
        let p = puzzle_with(LovelacePhase::LoopBranch, &["LOAD 3, 1"]);
        assert_eq!(p.compile().unwrap_err().code, "ERR_MEM_CEILING");
    }

    #[test]
    fn phase_one_overruns_after_four_instructions() {
        let five: Vec<&str> = vec!["LOAD 0, 1"; 5];
        let p = puzzle_with(LovelacePhase::LinearFlow, &five);
        assert_eq!(p.compile().unwrap_err().code, "ERR_MEM_OVERRUN");
        // ...but exactly 4 is fine.
        let four: Vec<&str> = vec!["LOAD 0, 1"; 4];
        assert!(puzzle_with(LovelacePhase::LinearFlow, &four).compile().is_ok());
    }

    #[test]
    fn out_of_range_jump_is_rejected() {
        let p = puzzle_with(LovelacePhase::LoopBranch, &["LOAD 0, 1", "JMP 9"]);
        assert_eq!(p.compile().unwrap_err().code, "ERR_BAD_JUMP");
    }

    #[test]
    fn infinite_loop_trips_the_step_guard() {
        let p = puzzle_with(LovelacePhase::LoopBranch, &["LOAD 0, 1", "JMP 0"]);
        let (instr, _) = p.compile().unwrap();
        let (outcome, _regs, steps) = execute(&instr);
        assert!(matches!(outcome, RunOutcome::Runaway));
        assert!(steps > STEP_BURN_THRESHOLD);
    }

    #[test]
    fn store_copies_register_contents() {
        let p = puzzle_with(LovelacePhase::LoopBranch, &["LOAD 0, 9", "STORE 0, 2", "HALT"]);
        let (instr, _) = p.compile().unwrap();
        let (_o, regs, _s) = execute(&instr);
        assert_eq!(regs[2], 9);
    }
}

#[cfg(test)]
mod tremor_tests {
    use super::*;

    #[test]
    fn swap_adjacent_preserves_length_and_chars() {
        let out = swap_adjacent("LOAD", 0);
        assert_eq!(out.chars().count(), 4);
        let mut a: Vec<char> = "LOAD".chars().collect();
        let mut b: Vec<char> = out.chars().collect();
        a.sort_unstable();
        b.sort_unstable();
        assert_eq!(a, b); // same multiset, just transposed
        assert_ne!(out, "LOAD"); // an actual swap happened
    }

    #[test]
    fn swap_adjacent_is_noop_on_tiny_strings() {
        assert_eq!(swap_adjacent("", 7), "");
        assert_eq!(swap_adjacent("X", 7), "X");
    }
}
