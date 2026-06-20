use std::collections::HashMap;

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
// TrueColor palette
// ─────────────────────────────────────────────────────────────────────────────
const WS_BG:      Color = Color::Rgb(10, 8, 0);
const BORDER_FG:  Color = Color::Rgb(120, 90, 20);
const HEADER_FG:  Color = Color::Rgb(255, 213, 102);
const TEXT_FG:    Color = Color::Rgb(255, 176, 0);
const DIM_FG:     Color = Color::Rgb(74, 50, 5);
const SECTION_FG: Color = Color::Rgb(160, 140, 100);
const OK_FG:      Color = Color::Rgb(255, 213, 102);
const FAIL_FG:    Color = Color::Rgb(255, 102, 51);
const STABLE_FG:  Color = Color::Rgb(255, 176, 0);
const CORRUPT_FG: Color = Color::Rgb(255, 80, 40);
const HEAD_FG:    Color = Color::Rgb(20, 14, 0);   // dark ink on bright head block
const HEAD_BG:    Color = Color::Rgb(255, 213, 102);

// ─────────────────────────────────────────────────────────────────────────────
// Machine constants
// ─────────────────────────────────────────────────────────────────────────────
/// Blank tape symbol byte ('_').
const BLANK: u8 = 0x5F;
/// Hard ceiling on tape growth and on a single run's step count — bounds memory and
/// guarantees the executor always terminates (no infinite hang on a trapped head).
const MAX_TAPE: usize = 64;
const MAX_STEPS: usize = 256;
/// Frames between Imitation-Game split-interrogation phases — a short breathing beat
/// (~3 s at 62.5 fps) so the next question arrives promptly after the current one resolves,
/// rather than leaving the puzzle frozen. (Was 1250 ≈ 20 s, the cause of the dead pause.)
/// Note: the per-question answer window — the heart-spike mechanic — is the separate, longer
/// `INTERROGATION_WINDOW` below and is unchanged.
const INTERROGATION_PERIOD: u16 = 188;
/// Frames the player has to answer an interrogation — a comfortable but tense ~15 s at
/// 62.5 fps, so there is time to read the milestone log and absorb the text degradation.
const INTERROGATION_WINDOW: u16 = 938;

// ═════════════════════════════════════════════════════════════════════════════
// THE STATE MACHINE
// ═════════════════════════════════════════════════════════════════════════════

/// Finite-control states. `A` scans the stabilised prefix, `B` is the corruption trap
/// (never halts), `C` confirms the terminator, `Halt` is the accepting stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
    A,
    B,
    C,
    Halt,
}

/// The 3-symbol tape alphabet the finite control reads (derived from the raw byte and
/// whether the cell is a still-corrupted act residue).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Sym {
    Blank,
    Stable,
    Corrupt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dir {
    L,
    R,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogKind {
    Info,
    Error,
    Warning,
    Success,
}

/// One residue of a completed act, seeded onto the tape as a corrupted byte the player
/// must overwrite back to its stable target.
struct Residue {
    act: &'static str,
    target: u8,
}

/// A single Imitation-Game split interrogation: the on-screen prompt (question + three
/// inline 1/2/3 options) and the 0-based index of the true answer the player must isolate.
struct Interrogation {
    prompt: &'static str, // "[IMITATION] … \u{2014} 1:… | 2:… | 3:…"
    correct: usize,       // 0-based index of the true answer
    timer: u16,           // frames remaining to answer
}

/// The climactic Universal-Machine puzzle state.
pub struct TuringCore {
    pub infinite_tape: Vec<u8>,
    pub head_position: usize,
    pub current_state: State,
    state_transition_table: HashMap<(State, Sym), (State, Sym, Dir)>,
    residues: Vec<Residue>,
    residue_at: HashMap<usize, usize>, // tape index → residues[] index
    run_steps: usize,
    interrogation: Option<Interrogation>,
    interro_timer: u16,
    status_log: Vec<(String, LogKind)>,
    pub solved: bool,
    /// True while the heart is spiked from a failed interrogation — drives the +panic BPM
    /// modifier and the extra log corruption. Cleared when the pulse is steadied.
    pub heart_spiked: bool,
}

impl TuringCore {
    pub fn new() -> Self {
        // ── Seed the tape with the residues of the five completed acts, contiguous so
        //    the scanner can verify them as one prefix, then a blank terminator. ──
        let residues = vec![
            Residue { act: "JACQUARD", target: 0x18 },
            Residue { act: "BABBAGE",  target: 0x37 },
            Residue { act: "LOVELACE", target: 0x43 },
            Residue { act: "BOOLE",    target: 0x54 },
            Residue { act: "SHANNON",  target: 0x48 },
        ];
        let mut infinite_tape: Vec<u8> = Vec::new();
        let mut residue_at: HashMap<usize, usize> = HashMap::new();
        for (i, _r) in residues.iter().enumerate() {
            residue_at.insert(i, i);
            infinite_tape.push(0x00); // corrupted seed (≠ target)
        }
        // Blank run after the residues.
        for _ in 0..3 {
            infinite_tape.push(BLANK);
        }

        Self {
            infinite_tape,
            head_position: 0,
            current_state: State::A,
            state_transition_table: build_table(),
            residues,
            residue_at,
            run_steps: 0,
            interrogation: None,
            interro_timer: INTERROGATION_PERIOD,
            status_log: vec![
                ("Stabilise the act residues (hex keys), then ENTER to run to HALT.".to_string(), LogKind::Info),
            ],
            solved: false,
            heart_spiked: false,
        }
    }

    /// The player's progression counter — the number of act residues already stabilised
    /// (0..5). Drives the active interrogation milestone, the heartbeat formula, and the
    /// progressive log decay.
    pub fn stable_residues_count(&self) -> usize {
        self.stabilised_count()
    }

    fn push_log(&mut self, msg: String, kind: LogKind) {
        self.status_log.push((msg, kind));
        if self.status_log.len() > 8 {
            self.status_log.remove(0);
        }
    }

    fn cell(&self, pos: usize) -> u8 {
        self.infinite_tape.get(pos).copied().unwrap_or(BLANK)
    }

    /// Classify a cell into the finite-control alphabet given its residue context.
    fn classify(&self, pos: usize) -> Sym {
        let byte = self.cell(pos);
        if byte == BLANK {
            return Sym::Blank;
        }
        if let Some(&ri) = self.residue_at.get(&pos) {
            if byte != self.residues[ri].target {
                return Sym::Corrupt;
            }
        }
        Sym::Stable
    }

    fn residues_all_stable(&self) -> bool {
        self.residue_at
            .iter()
            .all(|(&pos, &ri)| self.cell(pos) == self.residues[ri].target)
    }

    fn stabilised_count(&self) -> usize {
        self.residue_at
            .iter()
            .filter(|(&pos, &ri)| self.cell(pos) == self.residues[ri].target)
            .count()
    }

    /// Stabilise the next still-corrupt residue (lowest tape position), writing its
    /// target byte onto the tape. Returns `true` if one was stabilised — the mutation
    /// that lets a correctly-isolated memory advance `RESIDUES: N/5 stable`.
    fn stabilize_next(&mut self) -> bool {
        let mut entries: Vec<(usize, usize)> =
            self.residue_at.iter().map(|(&p, &r)| (p, r)).collect();
        entries.sort_by_key(|&(p, _)| p);
        for (pos, ri) in entries {
            if self.cell(pos) != self.residues[ri].target {
                if pos < self.infinite_tape.len() {
                    self.infinite_tape[pos] = self.residues[ri].target;
                }
                return true;
            }
        }
        false
    }

    /// Force every residue to its stable target (the F10 demo fast-forward).
    fn stabilize_all(&mut self) {
        let entries: Vec<(usize, u8)> = self
            .residue_at
            .iter()
            .map(|(&p, &r)| (p, self.residues[r].target))
            .collect();
        for (pos, target) in entries {
            if pos < self.infinite_tape.len() {
                self.infinite_tape[pos] = target;
            }
        }
    }

    /// Execute one finite-control transition. Bounds are saturated and the tape grows
    /// at most to `MAX_TAPE`, so this can never panic or grow without limit.
    fn step(&mut self) {
        if self.current_state == State::Halt {
            return;
        }
        let sym = self.classify(self.head_position);
        let &(next, write, dir) = match self.state_transition_table.get(&(self.current_state, sym)) {
            Some(t) => t,
            None => return, // no rule → wedge (treated as a non-halt by the run loop)
        };
        // Write (only when the class actually changes — keeps the player's bytes intact).
        if write != sym {
            if self.head_position < self.infinite_tape.len() {
                self.infinite_tape[self.head_position] = rep_byte(write);
            }
        }
        self.current_state = next;
        match dir {
            Dir::L => {
                if self.head_position == 0 {
                    if self.infinite_tape.len() < MAX_TAPE {
                        self.infinite_tape.insert(0, BLANK);
                        // residue indices shift right by one
                        self.residue_at = self
                            .residue_at
                            .iter()
                            .map(|(&p, &r)| (p + 1, r))
                            .collect();
                        // head stays at 0 (now the new blank)
                    }
                } else {
                    self.head_position -= 1;
                }
            }
            Dir::R => {
                if self.head_position + 1 >= self.infinite_tape.len() {
                    if self.infinite_tape.len() < MAX_TAPE {
                        self.infinite_tape.push(BLANK);
                        self.head_position += 1;
                    }
                    // else clamp in place
                } else {
                    self.head_position += 1;
                }
            }
        }
    }

    /// Run the machine from the current head, restarting the finite control at `A`.
    /// Terminates on `Halt` or after `MAX_STEPS` (a trapped, non-halting head).
    fn run(&mut self) {
        self.current_state = State::A;
        let mut steps = 0;
        while self.current_state != State::Halt && steps < MAX_STEPS {
            self.step();
            steps += 1;
        }
        self.run_steps = steps;
    }
}

/// The complete 5-tuple transition matrix: `(state, read) → (state, write, move)`.
fn build_table() -> HashMap<(State, Sym), (State, Sym, Dir)> {
    use Dir::*;
    use State::*;
    use Sym::*;
    let mut t = HashMap::new();
    // A — scan the stabilised prefix rightward.
    t.insert((A, Stable), (A, Stable, R));
    t.insert((A, Corrupt), (B, Corrupt, R)); // a corrupt residue traps the head
    t.insert((A, Blank), (C, Blank, L)); // reached the terminator → confirm
    // B — the corruption trap: keeps running right, never halts.
    t.insert((B, Stable), (B, Stable, R));
    t.insert((B, Corrupt), (B, Corrupt, R));
    t.insert((B, Blank), (B, Blank, R));
    // C — terminator confirmed → accept and stop.
    t.insert((C, Stable), (Halt, Stable, L));
    t.insert((C, Corrupt), (B, Corrupt, R));
    t.insert((C, Blank), (Halt, Blank, L));
    t
}

/// Representative byte for a written symbol class (only used when a transition changes
/// a cell's class — this machine writes identity, so it is effectively a safe default).
fn rep_byte(sym: Sym) -> u8 {
    match sym {
        Sym::Blank => BLANK,
        Sym::Stable => 0x01,
        Sym::Corrupt => 0x00,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Imitation-Game content
// ─────────────────────────────────────────────────────────────────────────────

/// One chronological interrogation: the spoken mind-stream log line and the on-screen
/// prompt with its three inline options. `correct` is 0-based (1/2/3 keys → 0/1/2).
struct InterroDef {
    log: &'static str,
    prompt: &'static str,
    correct: usize,
}

/// The five progressive milestones of Alan Turing's life, indexed by the player's
/// stabilised-residue count (0..4) — never random, so the Imitation Game reads as a
/// chronological story rather than thematic noise. Options map [1] Investigator,
/// [2] Memory, [3] Machine.
const INTERROGATIONS: [InterroDef; 5] = [
    InterroDef {
        log: "I imagined a machine of infinite patience: a tape, a single head...",
        prompt: "[IMITATION] What bounds the architecture of a mechanical mind? \u{2014} 1:The Crown Registry | 2:Human Sorrow | 3:The Infinite Tape",
        correct: 2,
    },
    InterroDef {
        log: "Christopher... he died so young. Can the spirit survive the breakdown of the physical clockwork?",
        prompt: "[IMITATION] Where does consciousness wander when the system halts? \u{2014} 1:Official Obituary | 2:Christopher's Ghost | 3:The Open Relay",
        correct: 1,
    },
    InterroDef {
        log: "We broke Enigma. We saved millions. Yet, the Official Secrets Act binds my tongue.",
        prompt: "[IMITATION] You saved an empire that demands your erasure. Who holds the master key? \u{2014} 1:The Secrets Act | 2:Hut 8 Memoirs | 3:The Bombe's Drums",
        correct: 0,
    },
    InterroDef {
        log: "Stand before the bench. Prosecuted for who I am. A gross indecency.",
        prompt: "[IMITATION] Define your deviation to the eyes of the law. \u{2014} 1:Gross Indecency Charge | 2:Solitary Remorse | 3:Logical Paradox",
        correct: 0,
    },
    InterroDef {
        log: "They make me take the pills. The flesh is failing. Drawing closer to the sweet almond hush...",
        prompt: "[IMITATION] The organic channel is corrupted by poison. What is the final state? \u{2014} 1:Police Dossier | 2:The Bitten Apple | 3:System Shutdown",
        correct: 1,
    },
];

/// Split an authored interrogation prompt — formatted `"<question> \u{2014} 1:… | 2:… | 3:…"`
/// — into the question text and its individual options. Layout-only: every fragment is the
/// verbatim authored wording, just trimmed. Falls back to `(whole prompt, [])` if the prompt
/// carries no `\u{2014}` separator.
fn split_prompt(prompt: &str) -> (&str, Vec<&str>) {
    match prompt.split_once('\u{2014}') {
        Some((question, opts)) => {
            let options = opts
                .split('|')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();
            (question.trim(), options)
        }
        None => (prompt.trim(), Vec::new()),
    }
}

/// Word-wrap a string to `max_w`-wide lines (breaking on spaces; an over-long token is
/// hard-split). Used to lay the interrogation prompt across the narrow centre panel.
fn wrap_text(text: &str, max_w: usize) -> Vec<String> {
    let max_w = max_w.max(1);
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in text.split(' ') {
        if word.chars().count() > max_w {
            if !cur.is_empty() {
                lines.push(std::mem::take(&mut cur));
            }
            let chars: Vec<char> = word.chars().collect();
            let mut i = 0;
            while chars.len() - i > max_w {
                lines.push(chars[i..i + max_w].iter().collect());
                i += max_w;
            }
            cur = chars[i..].iter().collect();
        } else if cur.is_empty() {
            cur.push_str(word);
        } else if cur.chars().count() + 1 + word.chars().count() <= max_w {
            cur.push(' ');
            cur.push_str(word);
        } else {
            lines.push(std::mem::take(&mut cur));
            cur.push_str(word);
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
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

/// Draw the bottom status gauge, clamping the line to the strict inner width
/// (`area.width - 4`) so a long prompt (e.g. "[IMITATION] split interrogation...") can
/// never bleed past the centre panel's right vertical border. The flanking border
/// tokens stay rigid and uniform regardless of the status string's length.
fn draw_status_footer(buf: &mut Buffer, area: Rect, msg: &str, col: Color) {
    let max_line_w = area.width.saturating_sub(4) as usize;
    let clamped: String = msg.chars().take(max_line_w).collect();
    layout::draw_gauge_footer(buf, area, &clamped, col);
}

fn state_name(s: State) -> &'static str {
    match s {
        State::A => "A",
        State::B => "B",
        State::C => "C",
        State::Halt => "HALT",
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// RENDER — THE REEL TAPE ENGINE (ACT VI)
// ═════════════════════════════════════════════════════════════════════════════

/// Render the centre workspace: the horizontally-scrolling Turing tape bracketed in
/// heavy borders with a pulsating head, plus the System Deck dossier beneath it. The
/// left Mind Stream is the standard mind-log panel; the right archive is the desk.
pub fn render_workspace(
    f: &mut Frame,
    area: Rect,
    state: &mut GlobalStateContext,
    core: &TuringCore,
    narration_streaming: bool,
) {
    let buf = f.buffer_mut();
    layout::draw_workspace_grid(buf, area, WS_BG);

    if state.screen_state == ScreenState::AmbientDesk {
        layout::draw_gauge_footer(buf, area, "", DIM_FG);
        return;
    }

    let (x0, y0, w, h) = (area.x, area.y, area.width, area.height);
    if w < 24 || h < 14 {
        return;
    }
    let frame = state.frame_counter;

    draw_box(buf, area, Style::default().fg(BORDER_FG).bg(WS_BG));
    let header = clip(
        " UNIVERSAL MACHINE \u{2500}\u{2500} A. TURING (1936 / 1950) ",
        w.saturating_sub(4) as usize,
    );
    buf_set_str(buf, x0 + 2, y0, &header, Style::default().fg(HEADER_FG).bg(WS_BG));

    // ── Victory takes the whole panel. ──
    if core.solved {
        draw_victory(buf, area, frame);
        if let Some((msg, _)) = core.status_log.last() {
            draw_status_footer(buf, area, msg, OK_FG);
        }
        return;
    }

    let inner_x = x0 + 2;
    let inner_w = w.saturating_sub(4);

    // ── The reel-to-reel tape strip. ──
    let tape_y = y0 + 2;
    draw_tape(buf, inner_x, tape_y, inner_w, core, frame);

    // ── Finite-control readout under the tape. ──
    let ctrl_y = tape_y + 4;
    let ctrl = format!(
        "STATE {}    HEAD @{}    STEPS {}",
        state_name(core.current_state),
        core.head_position,
        core.run_steps
    );
    let ctrl_col = if core.current_state == State::Halt { OK_FG } else { TEXT_FG };
    buf_set_str(buf, inner_x, ctrl_y, &clip(&ctrl, inner_w as usize), Style::default().fg(ctrl_col).bg(WS_BG));

    // ── Active interrogation prompt (the Imitation Game intrusion). The choices stay
    //    sealed while the milestone narrative is still streaming in the left panel —
    //    they materialise only once the line has finished (or been skipped). ──
    let mut deck_y = ctrl_y + 2;
    if let Some(inter) = core.interrogation.as_ref().filter(|_| !narration_streaming) {
        // Ceil-divide by the ~63-frame second so a 938-frame window reads a clean 15→1
        // and never flashes 0 while time remains.
        let secs = (inter.timer + 62) / 63;
        buf_set_str(
            buf,
            inner_x,
            deck_y,
            &clip(&format!("\u{2592} IMITATION \u{2014} isolate the true memory ({}s)", secs), inner_w as usize),
            Style::default().fg(FAIL_FG).bg(WS_BG),
        );
        deck_y += 1;
        let bottom = y0 + h.saturating_sub(2);
        // The chronological prompt reads as a block; the 1/2/3 options then sit on their
        // own lines beneath it, separated by a blank row, so they never crowd the question.
        let (question, options) = split_prompt(inter.prompt);
        for line in wrap_text(question, inner_w as usize) {
            if deck_y >= bottom {
                break;
            }
            buf_set_str(buf, inner_x, deck_y, &line, Style::default().fg(SECTION_FG).bg(WS_BG));
            deck_y += 1;
        }
        // Blank line separating the question block from the choices.
        deck_y += 1;
        for opt in options {
            for line in wrap_text(opt, inner_w as usize) {
                if deck_y >= bottom {
                    break;
                }
                buf_set_str(buf, inner_x, deck_y, &line, Style::default().fg(TEXT_FG).bg(WS_BG));
                deck_y += 1;
            }
        }
        deck_y += 1;
    }

    // ── System Deck dossier. ──
    if deck_y + 5 <= y0 + h.saturating_sub(1) {
        draw_system_deck(buf, inner_x, deck_y, inner_w.min(30), core);
    }

    if let Some((msg, kind)) = core.status_log.last() {
        let col = match kind {
            LogKind::Info => Color::Rgb(170, 150, 110),
            LogKind::Error => FAIL_FG,
            LogKind::Warning => TEXT_FG,
            LogKind::Success => OK_FG,
        };
        draw_status_footer(buf, area, msg, col);
    }
}

fn draw_tape(buf: &mut Buffer, x: u16, y: u16, width: u16, core: &TuringCore, frame: u64) {
    let cell_w: u16 = 4; // " XX " or "▸XX◂"
    let span: u16 = cell_w + 1; // + separator
    let visible = ((width.saturating_sub(1)) / span).clamp(1, 13);
    let len = core.infinite_tape.len();

    // Window centred on the head, clamped into range.
    let half = (visible / 2) as usize;
    let start = core.head_position.saturating_sub(half);
    let start = start.min(len.saturating_sub(visible as usize).max(0));

    // Pulsating head highlight (high-frequency).
    let pulse_on = (frame / 4) % 2 == 0;

    let mut top = String::from("\u{256A}"); // ╪
    let mut mid = String::from("\u{2551}"); // ║
    let mut bot = String::from("\u{256A}");
    let mut head_cols: Vec<(u16, u8)> = Vec::new(); // (screen x, byte) for head styling

    let mut sx = x + 1;
    for i in 0..visible as usize {
        let pos = start + i;
        let last = i + 1 == visible as usize;
        // borders between cells
        top.push_str("\u{2550}\u{2550}\u{2550}\u{2550}"); // ════
        bot.push_str("\u{2550}\u{2550}\u{2550}\u{2550}");
        top.push(if last { '\u{256A}' } else { '\u{2564}' }); // ╪ or ╤
        bot.push(if last { '\u{256A}' } else { '\u{2567}' }); // ╪ or ╧

        let byte = core.cell(pos);
        let is_head = pos == core.head_position;
        let cell_str = if is_head {
            format!("\u{25B8}{:02X}\u{25C2}", byte) // ▸XX◂
        } else if byte == BLANK {
            " __ ".to_string()
        } else {
            format!(" {:02X} ", byte)
        };
        mid.push_str(&cell_str);
        mid.push(if last { '\u{2551}' } else { '\u{2502}' });
        if is_head {
            head_cols.push((sx, byte));
        }
        sx += span;
    }

    let border_style = Style::default().fg(BORDER_FG).bg(WS_BG);
    buf_set_str(buf, x, y, &clip(&top, width as usize), border_style);
    // Mid row: draw plainly, then recolour each cell glyph by class / head.
    buf_set_str(buf, x, y + 1, &clip(&mid, width as usize), Style::default().fg(TEXT_FG).bg(WS_BG));
    buf_set_str(buf, x, y + 2, &clip(&bot, width as usize), border_style);

    // Recolour cell contents by residue class + the pulsating head block.
    let mut cx = x + 1;
    for i in 0..visible as usize {
        let pos = start + i;
        let is_head = pos == core.head_position;
        let sym = core.classify(pos);
        let base = match sym {
            Sym::Corrupt => CORRUPT_FG,
            Sym::Blank => DIM_FG,
            Sym::Stable => STABLE_FG,
        };
        for k in 0..cell_w {
            let gx = cx + k;
            if !in_bounds(buf, gx, y + 1) {
                continue;
            }
            let cell = buf.get_mut(gx, y + 1);
            if is_head {
                if pulse_on {
                    cell.fg = HEAD_FG;
                    cell.bg = HEAD_BG;
                } else {
                    cell.fg = HEAD_BG;
                    cell.bg = WS_BG;
                }
            } else {
                cell.fg = base;
                cell.bg = WS_BG;
            }
        }
        cx += span;
    }
    let _ = head_cols;
}

/// The centre System Deck dossier. Deliberately puzzle-focused: STATE / GLYPHS / RESIDUES
/// plus the per-act stabilisation tracker. The candle life-line and the heartbeat metronome
/// are NOT repeated here — they already live on the right-column desk and the bottom
/// telemetry bar respectively, so the player isn't shown the same gauge twice.
fn draw_system_deck(buf: &mut Buffer, x: u16, y: u16, width: u16, core: &TuringCore) {
    let h: u16 = 6;
    let rect = Rect::new(x, y, width, h);
    // Fill the dossier interior so the blueprint grid doesn't bleed through the text.
    for fy in y..y + h {
        for fx in x..x + width {
            buf_set(buf, fx, fy, ' ', Style::default().fg(WS_BG).bg(WS_BG));
        }
    }
    draw_box(buf, rect, Style::default().fg(BORDER_FG).bg(WS_BG));
    buf_set_str(buf, x + 1, y, &clip(" SYSTEM DECK ", width.saturating_sub(2) as usize), Style::default().fg(HEADER_FG).bg(WS_BG));

    let inner = width.saturating_sub(2) as usize;
    let stable = core.stabilised_count();
    let total = core.residues.len();

    let rows: [(String, Color); 3] = [
        (format!("STATE   : {}", state_name(core.current_state)), if core.current_state == State::Halt { OK_FG } else { TEXT_FG }),
        (format!("GLYPHS  : {}", core.infinite_tape.len()), TEXT_FG),
        (format!("RESIDUES: {}/{} stable", stable, total), if stable == total { OK_FG } else { CORRUPT_FG }),
    ];
    let mut ry = y + 1;
    for (text, color) in rows {
        buf_set_str(buf, x + 1, ry, &clip(&text, inner), Style::default().fg(color).bg(WS_BG));
        ry += 1;
    }
    // Act-stabilisation progress tracker: each prior act's initial, lit when stable.
    if ry < y + h - 1 {
        buf_set_str(buf, x + 1, ry, "ACTS:", Style::default().fg(SECTION_FG).bg(WS_BG));
        let mut gx = x + 7;
        for r in &core.residues {
            if gx >= x + width.saturating_sub(1) {
                break;
            }
            let ok = core.cell_for_residue(r);
            let ch = r.act.chars().next().unwrap_or('?');
            let color = if ok { OK_FG } else { CORRUPT_FG };
            buf_set(buf, gx, ry, ch, Style::default().fg(color).bg(WS_BG));
            gx += 2;
        }
    }
}

impl TuringCore {
    fn cell_for_residue(&self, r: &Residue) -> bool {
        // Find this residue's tape index and compare against target.
        self.residue_at
            .iter()
            .find(|&(_, &ri)| std::ptr::eq(&self.residues[ri], r))
            .map(|(&pos, _)| self.cell(pos) == r.target)
            .unwrap_or(false)
    }
}

fn draw_victory(buf: &mut Buffer, area: Rect, frame: u64) {
    let cx = area.x + area.width / 2;
    let cy = area.y + area.height / 2;
    let on = (frame / 24) % 2 == 0;
    let lines: [(&str, Color); 4] = [
        ("THE MACHINE HALTS", OK_FG),
        ("Every residue stabilised. The sequence resolves.", TEXT_FG),
        ("\"...and the imitation was, after all, the man.\"", Color::Rgb(200, 150, 40)),
        (if on { "[ the tape runs to silence ]" } else { "" }, DIM_FG),
    ];
    let mut ly = cy.saturating_sub(2);
    for (text, color) in lines {
        let half = text.chars().count() as u16 / 2;
        buf_set_str(buf, cx.saturating_sub(half), ly, text, Style::default().fg(color).bg(WS_BG));
        ly += 1;
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// EVENT HANDLER
// ═════════════════════════════════════════════════════════════════════════════

/// Route a key against the Turing core. 1/2/3 answer an active interrogation; otherwise
/// Left/Right slide the head (heavy click), hex keys overwrite the cell, Space clears
/// it, and Enter runs the machine toward HALT.
pub fn handle_input(
    key: KeyEvent,
    core: &mut TuringCore,
    state: &mut GlobalStateContext,
    dialogue: &mut DialogueEngine,
    audio: &mut AudioEngine,
) {
    if core.solved {
        return;
    }

    // ── Streaming skip-interrupt (and the narrative gate). While the milestone log is
    //    still typing out in the left panel, ANY keypress flushes the whole remaining
    //    line at once and is consumed — so the centre choices stay sealed and 1/2/3 are
    //    never processed until the narrative has finished (or is skipped). The options
    //    unlock the exact frame `is_streaming()` drops to false. ──
    if dialogue.is_streaming() {
        dialogue.skip();
        return;
    }

    // The Imitation Game intercepts numeric input while a phase is live.
    if core.interrogation.is_some() {
        if let KeyCode::Char(c @ '1'..='3') = key.code {
            let pick = (c as u8 - b'1') as usize;
            answer_interrogation(core, state, dialogue, audio, pick);
            return;
        }
    }

    match key.code {
        // Hidden developer shortcut (demo-recording safeguard): F10 resolves the whole
        // deck matrix, maxes RESIDUES to 5/5, runs to HALT, and fires the final ending.
        KeyCode::F(10) => {
            core.stabilize_all();
            core.run();
            trigger_victory(core, state, dialogue);
        }
        KeyCode::Left => {
            if core.head_position > 0 {
                core.head_position -= 1;
                audio.head_click();
            }
        }
        KeyCode::Right => {
            if core.head_position + 1 >= core.infinite_tape.len() {
                if core.infinite_tape.len() < MAX_TAPE {
                    core.infinite_tape.push(BLANK);
                    core.head_position += 1;
                    audio.head_click();
                }
            } else {
                core.head_position += 1;
                audio.head_click();
            }
        }
        KeyCode::Char(c) if c.is_ascii_hexdigit() => {
            if let Some(nib) = c.to_digit(16) {
                let pos = core.head_position;
                if pos < core.infinite_tape.len() {
                    let cur = core.infinite_tape[pos];
                    core.infinite_tape[pos] = ((cur << 4) | (nib as u8)) & 0xFF;
                    audio.daktilo_fast();
                }
            }
        }
        KeyCode::Char(' ') => {
            let pos = core.head_position;
            if pos < core.infinite_tape.len() {
                core.infinite_tape[pos] = 0x00; // clear to a corrupted blank-of-intent
                audio.backspace_snap();
            }
        }
        KeyCode::Enter => {
            core.run();
            if core.current_state == State::Halt && core.residues_all_stable() {
                trigger_victory(core, state, dialogue);
            } else if core.current_state == State::Halt {
                core.push_log("Halted early on a blank — residues remain corrupt. Stabilise them all.".to_string(), LogKind::Warning);
            } else {
                core.push_log("Non-halting: a corrupt residue traps the head. Resolve the corruption.".to_string(), LogKind::Error);
            }
        }
        _ => {}
    }
}

fn answer_interrogation(
    core: &mut TuringCore,
    state: &mut GlobalStateContext,
    dialogue: &mut DialogueEngine,
    audio: &mut AudioEngine,
    pick: usize,
) {
    let correct = core.interrogation.as_ref().map(|i| i.correct).unwrap_or(0);
    core.interrogation = None;
    if pick == correct {
        // Steady the pulse and stabilise the next act residue — isolating a true memory
        // mutates the tape and advances RESIDUES: N/5 stable.
        state.arrhythmia_multiplier = 0.0;
        core.heart_spiked = false;
        audio.menu_confirm();
        if core.stabilize_next() {
            let n = core.stabilised_count();
            let total = core.residues.len();
            core.push_log(
                format!("[IMITATION] true memory isolated \u{2014} residue stabilised ({}/{}).", n, total),
                LogKind::Success,
            );
        } else {
            core.push_log("[IMITATION] true memory isolated \u{2014} the pulse steadies.".to_string(), LogKind::Success);
        }
        // Every residue stable → run to HALT and route into the closing cinematic.
        if core.residues_all_stable() {
            core.run();
            trigger_victory(core, state, dialogue);
        }
    } else {
        interrogation_penalty(core, state, audio);
    }
}

/// Resolve the climax: mark the act complete, settle the heart, and play Turing's final
/// line. `core.solved` flips here; the main loop watches for it and, once the line has
/// streamed, routes into the closing Act-VI outro cinematic (the bitten apple) and on to
/// the menu — so the endgame never freezes or loops. Idempotent.
fn trigger_victory(core: &mut TuringCore, state: &mut GlobalStateContext, dialogue: &mut DialogueEngine) {
    if core.solved {
        return;
    }
    core.solved = true;
    core.heart_spiked = false;
    core.push_log("HALT reached on a stabilised tape. The computation resolves.".to_string(), LogKind::Success);
    if !state.acts_completed.contains(&Act::Turing1936_1950) {
        state.acts_completed.push(Act::Turing1936_1950);
    }
    state.arrhythmia_multiplier = 0.0;
    dialogue.play(
        Speaker::Turing,
        "[TURING]: \"The machine halts. Every ghost accounted for, every residue resolved. Let the tape run on, quietly, into the dark.\"",
        Some(VoiceCue::Victory(Act::Turing1936_1950)),
    );
}

/// The cost of failing to isolate the true memory: the heart spikes past 140 BPM, the
/// candle gutters, and the channel shorts out.
fn interrogation_penalty(core: &mut TuringCore, state: &mut GlobalStateContext, audio: &mut AudioEngine) {
    state.base_heartbeat_bpm = state.base_heartbeat_bpm.max(140);
    state.arrhythmia_multiplier = 2.0;
    core.heart_spiked = true;
    state.melt_candle(15);
    audio.glitch();
    core.push_log("[IMITATION] failed to isolate the memory \u{2014} the heart spikes to 140+.".to_string(), LogKind::Error);
}

// ═════════════════════════════════════════════════════════════════════════════
// TICK — the Imitation Game cadence
// ═════════════════════════════════════════════════════════════════════════════

/// Per-frame update: drive the split-interrogation cadence (a short between-question beat,
/// then the timed answer window) and time out an unanswered phase into a penalty.
pub fn tick_turing(
    core: &mut TuringCore,
    state: &mut GlobalStateContext,
    dialogue: &mut DialogueEngine,
    audio: &mut AudioEngine,
) {
    if core.solved {
        return;
    }

    if let Some(inter) = core.interrogation.as_mut() {
        if inter.timer > 0 {
            inter.timer -= 1;
        } else {
            // Unanswered → penalty.
            core.interrogation = None;
            interrogation_penalty(core, state, audio);
        }
        return;
    }

    if core.interro_timer > 0 {
        core.interro_timer -= 1;
        return;
    }
    core.interro_timer = INTERROGATION_PERIOD;

    // Progressive matrix: the active question is tied to the player's progression — the
    // number of residues already stabilised (0..4) — so each correct answer advances the
    // story to the next chronological milestone. No randomness.
    let idx = core.stabilised_count().min(INTERROGATIONS.len() - 1);
    let def = &INTERROGATIONS[idx];
    core.interrogation = Some(Interrogation {
        prompt: def.prompt,
        correct: def.correct,
        timer: INTERROGATION_WINDOW,
    });
    core.push_log("[IMITATION] split interrogation \u{2014} isolate the true memory (1/2/3).".to_string(), LogKind::Warning);
    if !dialogue.is_typing() {
        // Accelerated Act VI cadence so the choices unlock without dead air.
        dialogue.play_fast(Speaker::System, def.log, Some(VoiceCue::PoliceBootstep));
    }
    // Memory Echo: the prepared milestone whisper (q1..q5), hard-panned to the latched
    // ear so consecutive milestones alternate left/right — aligned to the chronological
    // state via the residue index. Both `_left` and `_right` takes exist per milestone.
    let pan = audio.next_whisper_pan();
    let side = if pan < 0.0 { "left" } else { "right" };
    audio.play_whisper(&format!("whispers/turing_q{}_{}.mp3", idx + 1, side), pan);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stabilise_all(core: &mut TuringCore) {
        let targets: Vec<(usize, u8)> = core
            .residue_at
            .iter()
            .map(|(&pos, &ri)| (pos, core.residues[ri].target))
            .collect();
        for (pos, target) in targets {
            core.infinite_tape[pos] = target;
        }
    }

    #[test]
    fn split_prompt_separates_question_from_stacked_options() {
        // Every authored prompt splits into a question and exactly three options, so the
        // renderer can stack 1/2/3 on their own lines beneath the question block.
        for def in INTERROGATIONS.iter() {
            let (question, options) = split_prompt(def.prompt);
            assert!(!question.is_empty() && !question.contains('\u{2014}'), "clean question");
            assert!(!question.contains('|'), "options must not leak into the question");
            assert_eq!(options.len(), 3, "three stacked options");
            assert!(options[0].starts_with("1:"));
            assert!(options[1].starts_with("2:"));
            assert!(options[2].starts_with("3:"));
        }
    }

    #[test]
    fn interrogation_matrix_is_five_progressive_and_well_formed() {
        // One question per residue, in chronological order; correct index always valid;
        // each prompt carries its three inline options.
        assert_eq!(INTERROGATIONS.len(), 5);
        for def in INTERROGATIONS.iter() {
            assert!(def.correct < 3, "answer index must be 0/1/2");
            assert!(def.prompt.contains("1:") && def.prompt.contains("2:") && def.prompt.contains("3:"));
            assert!(!def.log.is_empty());
        }
        // The active question index tracks the player's stabilised-residue progression.
        let core = TuringCore::new();
        let idx = core.stabilised_count().min(INTERROGATIONS.len() - 1);
        assert_eq!(idx, 0); // fresh tape → milestone 0 (the Infinite Tape)
        assert_eq!(INTERROGATIONS[0].correct, 2);
    }

    #[test]
    fn wrap_text_never_exceeds_width() {
        for line in wrap_text(INTERROGATIONS[2].prompt, 28) {
            assert!(line.chars().count() <= 28);
        }
    }

    #[test]
    fn heart_spike_and_progression_counter_start_calm() {
        let core = TuringCore::new();
        assert!(!core.heart_spiked);
        assert_eq!(core.stable_residues_count(), 0);
        // The vitals formula the main loop applies: 75 + count*15 (+55 when spiked).
        let calm = 75 + core.stable_residues_count() as u32 * 15;
        assert_eq!(calm, 75);
    }

    #[test]
    fn fresh_tape_is_corrupt_and_does_not_halt() {
        let mut core = TuringCore::new();
        assert!(!core.residues_all_stable());
        core.head_position = 0;
        core.run();
        assert_ne!(core.current_state, State::Halt, "corrupt residue must trap the head");
    }

    #[test]
    fn stabilised_tape_halts_from_first_cell() {
        let mut core = TuringCore::new();
        stabilise_all(&mut core);
        assert!(core.residues_all_stable());
        core.head_position = 0;
        core.run();
        assert_eq!(core.current_state, State::Halt);
        assert!(core.run_steps <= MAX_STEPS);
    }

    #[test]
    fn hex_shift_entry_builds_a_byte() {
        // Emulate typing '4' then 'A' into a cell → 0x4A.
        let mut cur: u8 = 0x00;
        for nib in [0x4u8, 0xA] {
            cur = ((cur << 4) | nib) & 0xFF;
        }
        assert_eq!(cur, 0x4A);
    }

    #[test]
    fn transition_table_is_complete_for_active_states() {
        let t = build_table();
        for st in [State::A, State::B, State::C] {
            for sym in [Sym::Blank, Sym::Stable, Sym::Corrupt] {
                assert!(t.contains_key(&(st, sym)), "missing rule for {:?},{:?}", st, sym);
            }
        }
    }

    #[test]
    fn isolating_memories_stabilises_residues_to_full() {
        // Each correctly-isolated memory stabilises one residue; five reach 5/5, at
        // which point the tape halts cleanly (the endgame trigger condition).
        let mut core = TuringCore::new();
        assert_eq!(core.stabilised_count(), 0);
        let total = core.residues.len();
        for i in 1..=total {
            assert!(core.stabilize_next());
            assert_eq!(core.stabilised_count(), i);
        }
        assert!(!core.stabilize_next(), "nothing left to stabilise at 5/5");
        assert!(core.residues_all_stable());
        core.head_position = 0;
        core.run();
        assert_eq!(core.current_state, State::Halt);
    }

    #[test]
    fn f10_stabilize_all_resolves_the_deck() {
        let mut core = TuringCore::new();
        core.stabilize_all();
        assert!(core.residues_all_stable());
        assert_eq!(core.stabilised_count(), core.residues.len());
    }

    #[test]
    fn status_footer_never_overruns_the_right_border() {
        use ratatui::{backend::TestBackend, Terminal};
        let mut core = TuringCore::new();
        core.status_log.clear();
        core.push_log(
            "[IMITATION] split interrogation \u{2014} 1:Court 2:Memory 3:Machine, far beyond the panel edge".to_string(),
            LogKind::Warning,
        );
        let mut state = GlobalStateContext::new();
        state.screen_state = ScreenState::ActivePuzzle;
        let (w, h) = (30u16, 18u16);
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| {
            render_workspace(f, Rect::new(0, 0, w, h), &mut state, &core, false);
        })
        .unwrap();
        let buf = term.backend().buffer().clone();
        // The right border column must stay the box border on the footer text row —
        // the long status line is clamped to width-4 and can never reach it.
        let sym = buf.get(w - 1, h - 2).symbol().to_string();
        assert_eq!(sym, "\u{2502}", "footer status bled onto/past the right border");
    }

    #[test]
    fn head_never_panics_at_left_edge() {
        let mut core = TuringCore::new();
        core.head_position = 0;
        // Force a left move via a manual step in a left-moving state.
        core.current_state = State::C;
        core.infinite_tape[0] = BLANK; // (C, Blank) → Halt, L
        core.step();
        assert!(core.head_position <= core.infinite_tape.len());
    }
}
