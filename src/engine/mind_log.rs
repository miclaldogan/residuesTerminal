use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    Frame,
};
use super::state::{Act, GlobalStateContext};

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

fn corrupt_string(s: &str, ppm: f32, seed: u64) -> String {
    if ppm <= 10.0 {
        return s.to_string();
    }
    let mut rng = Lcg::new(seed);
    let threshold = ((ppm - 10.0) / 90.0).clamp(0.0, 0.75); // Cap at 75% max corruption
    s.chars().map(|ch| {
        if ch == ' ' { return ' '; }
        if (rng.next() % 100) as f32 / 100.0 < threshold {
            let corrupt_chars = ['1', '0', '⠓', '⠙', '⠻', '⠵', '░', '▓', '▒', '█', 'x', '?', '#', '*', '@', '§'];
            let idx = (rng.next() as usize) % corrupt_chars.len();
            corrupt_chars[idx]
        } else {
            ch
        }
    }).collect()
}

/// Act VI cognitive-breakdown filter: as Turing's progression advances and his heart
/// spikes, completed log glyphs decay into structural entropy tokens (`§ # * ? ░`) with
/// probability `chance`. Deterministic in `seed` so the decay shimmers per frame rather
/// than dancing wildly; spaces are preserved so the word shapes survive the corruption.
fn turing_corrupt(s: &str, chance: f32, seed: u64) -> String {
    if chance <= 0.0 {
        return s.to_string();
    }
    const TOKENS: [char; 5] = ['\u{00A7}', '#', '*', '?', '\u{2591}']; // § # * ? ░
    let mut rng = Lcg::new(seed);
    s.chars()
        .map(|c| {
            if c == ' ' {
                return ' ';
            }
            if (rng.next() % 1000) as f32 / 1000.0 < chance {
                TOKENS[(rng.next() as usize) % TOKENS.len()]
            } else {
                c
            }
        })
        .collect()
}

/// Mechanical gear-alignment text error: swap `level` adjacent character pairs inside
/// a string (e.g. "this is" → "tihs si"). Spaces are skipped so word boundaries hold
/// and the line stays a recognisable-but-misaligned echo of itself. Deterministic in
/// `seed`, so a given stage renders the same glitch every frame instead of shimmering.
fn gear_glitch(s: &str, level: usize, seed: u64) -> String {
    if level == 0 {
        return s.to_string();
    }
    let mut chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    if n < 2 {
        return s.to_string();
    }
    let mut rng = Lcg::new(seed);
    let swaps = level.min(n / 2).max(1);
    for _ in 0..swaps {
        let start = (rng.next() as usize) % (n - 1);
        for k in 0..(n - 1) {
            let i = (start + k) % (n - 1);
            if chars[i] != ' ' && chars[i + 1] != ' ' {
                chars.swap(i, i + 1);
                break;
            }
        }
    }
    chars.into_iter().collect()
}

const COURT_RECORDS: &[&str] = &[
    "REGISTRY: Regina v. Turing (1952). Gross indecency trial Section 11...",
    "COURT ORDER: To submit to organo-therapy treatments of estrogen...",
    "ALAN: 'They make me take the pills. The mind is becoming soft...'",
    "POISON CLINIC: Stilboestrol dosage 50mg daily. Vision blur reported...",
    "HALLUCINATION: The red apple on the table. The sweet smell of cyanide...",
    "ALAN: 'My fingers tremor. I can't hit the keys cleanly. Delete. Delete.'",
    "CRIMINAL LAW: ...placed on probation with a condition of medical castration...",
];

// ═════════════════════════════════════════════════════════════════════════════
// THE INTERACTIVE DIALOGUE ENGINE
//
// A single, asynchronous typewriter that streams Turing's voice into the upper
// frame one character at a time. Every line is paired with a deterministic
// `VoiceCue` checkpoint that an external audio layer can subscribe to for
// fourth-wall-breaking voice-over playback.
// ═════════════════════════════════════════════════════════════════════════════

/// Who is speaking — drives both the colour and the voice-actor channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Speaker {
    Turing,
    MindLog,
    System,
}

/// Deterministic sound-playback checkpoints. Emitted exactly once at the instant a
/// line begins typing (or an event fires), so an external API can trigger a matching
/// audio sample. Markers map 1:1 onto the files in `audio/turing` and `audio/sfx`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceCue {
    /// One of the five full-screen prelude monologue lines (1-indexed).
    PreludeLine(u8),
    /// The safe-door slide as the player crosses from the ambient desk into play.
    DoorSlide,
    ActIntro(Act),
    /// A spoken biography line in the cinematic act intro: `(act, 1-based line number)`.
    /// Resolves to `audio/speechs/<act>_intro<n>.mp3` (Turing has no take → silent).
    ActIntroLine(Act, u8),
    JacquardTear,
    JacquardJam,
    BabbageCrunch,
    Blunder,
    /// A heavy interrogation-room bootstep, fired per court-record fragment (Act IV).
    PoliceBootstep,
    Victory(Act),
}

impl VoiceCue {
    /// Stable string id for the external sound API hook (never localised).
    /// `VO_TURING_SPEECH_n` resolves to `audio/turing/turingSpeechN.mp3`.
    pub fn marker(&self) -> &'static str {
        match self {
            VoiceCue::PreludeLine(1) => "VO_TURING_SPEECH_1",
            VoiceCue::PreludeLine(2) => "VO_TURING_SPEECH_2",
            VoiceCue::PreludeLine(3) => "VO_TURING_SPEECH_3",
            VoiceCue::PreludeLine(4) => "VO_TURING_SPEECH_4",
            VoiceCue::PreludeLine(5) => "VO_TURING_SPEECH_5",
            VoiceCue::PreludeLine(_) => "VO_TURING_SPEECH_6",
            VoiceCue::DoorSlide => "SFX_DOOR_SLIDE",
            VoiceCue::ActIntro(Act::Jacquard1804) => "VO_ACT_JACQUARD_INIT",
            VoiceCue::ActIntro(Act::Babbage1837) => "VO_ACT_BABBAGE_INIT",
            VoiceCue::ActIntro(Act::Lovelace1843) => "VO_ACT_LOVELACE_INIT",
            VoiceCue::ActIntro(Act::Boole1854) => "VO_ACT_BOOLE_INIT",
            VoiceCue::ActIntro(Act::Shannon1937) => "VO_ACT_SHANNON_INIT",
            VoiceCue::ActIntro(Act::Turing1936_1950) => "VO_ACT_TURING_INIT",
            // Per-line cinematic intro voice takes (1 or 2). The line number is folded to
            // "_1" / "_2" so any index past the second take reuses the second.
            VoiceCue::ActIntroLine(Act::Jacquard1804, 1) => "VO_INTRO_JACQUARD_1",
            VoiceCue::ActIntroLine(Act::Jacquard1804, _) => "VO_INTRO_JACQUARD_2",
            VoiceCue::ActIntroLine(Act::Babbage1837, 1) => "VO_INTRO_BABBAGE_1",
            VoiceCue::ActIntroLine(Act::Babbage1837, _) => "VO_INTRO_BABBAGE_2",
            VoiceCue::ActIntroLine(Act::Lovelace1843, 1) => "VO_INTRO_LOVELACE_1",
            VoiceCue::ActIntroLine(Act::Lovelace1843, _) => "VO_INTRO_LOVELACE_2",
            VoiceCue::ActIntroLine(Act::Boole1854, 1) => "VO_INTRO_BOOLE_1",
            VoiceCue::ActIntroLine(Act::Boole1854, _) => "VO_INTRO_BOOLE_2",
            VoiceCue::ActIntroLine(Act::Shannon1937, 1) => "VO_INTRO_SHANNON_1",
            VoiceCue::ActIntroLine(Act::Shannon1937, _) => "VO_INTRO_SHANNON_2",
            VoiceCue::ActIntroLine(Act::Turing1936_1950, 1) => "VO_INTRO_TURING_1",
            VoiceCue::ActIntroLine(Act::Turing1936_1950, _) => "VO_INTRO_TURING_2",
            VoiceCue::JacquardTear => "VO_JACQUARD_TEAR",
            VoiceCue::JacquardJam => "VO_JACQUARD_JAM",
            VoiceCue::BabbageCrunch => "VO_BABBAGE_CRUNCH",
            VoiceCue::Blunder => "VO_BLUNDER_TAUNT",
            VoiceCue::PoliceBootstep => "SFX_POLICE_BOOTSTEP",
            VoiceCue::Victory(_) => "VO_ACT_VICTORY",
        }
    }
}

/// A single authored beat of the typewriter performance. Lets the script encode
/// exactly what Alan does as he speaks: type words, fumble and backspace a false
/// start, or fall silent (a cough/breath — no keystrokes, no daktilo).
#[derive(Clone, Copy)]
pub enum Seg {
    /// Type these characters, one keystroke (and one daktilo clack) each.
    T(&'static str),
    /// Backspace N characters (a self-correction; no daktilo).
    B(usize),
    /// Hold silent for N nominal frames — used for coughs and breaths.
    P(u16),
}

/// The full-screen opening monologue, one beat script per `turingSpeechN.mp3` take.
///
/// The platen types only the CLEAN final sentence — no spoken stutters, repeats or
/// fillers. `P(..)` beats hold silent (no daktilo) where the take pauses/coughs, so
/// the words still land under his voice; the engine then auto-stretches the whole
/// line to the take's measured length. The only on-screen deletion is a deliberate
/// draft he types and erases (line 4: "i remem-").
pub const PRELUDE_SCRIPT: &[&[Seg]] = &[
    // 1 — turingSpeech1 → "You see, every systematic logic inherits a ghost. A residue."
    &[
        Seg::P(45),                 // [gasp] Ah...
        Seg::T("You see,"),
        Seg::P(22),
        Seg::T(" every systematic logic"),
        Seg::P(22),
        Seg::T(" inherits"),
        Seg::P(18),
        Seg::T(" a ghost."),
        Seg::P(30),
        Seg::T(" A residue."),
    ],
    // 2 — turingSpeech2 → "We spent a century teaching copper and iron how to remember."
    &[
        Seg::T("We spent"),
        Seg::P(28),                 // ...what? A century?
        Seg::T(" a century"),
        Seg::P(40),                 // [clears throat]
        Seg::T(" teaching copper and iron how to"),
        Seg::P(22),
        Seg::T(" remember."),
    ],
    // 3 — turingSpeech3 → "8 June 1954. Wilmslow. The potassium cyanide is quite
    //     silent, you see. The mathematical logic is perfectly intact."
    &[
        Seg::T("8 June 1954."),
        Seg::P(45),                 // [long pause]
        Seg::T(" Wilmslow."),
        Seg::P(45),                 // [long pause]
        Seg::T(" The"),
        Seg::P(18),                 // [short pause]
        Seg::T(" potassium cyanide"),
        Seg::P(22),
        Seg::T(" is quite"),
        Seg::P(40),                 // [sighs]
        Seg::T(" silent, you see."),
        Seg::P(45),                 // [long pause]
        Seg::T(" The mathematical logic is"),
        Seg::P(18),                 // [short pause]
        Seg::T(" perfectly intact."),
    ],
    // 4 — turingSpeech4 → he types a draft, erases it ("Delete that. Backspace."),
    //     then: "Only to realize the truly elegant part the most terrifying part is
    //     learning how to forget."
    &[
        Seg::P(45),                 // [long pause]
        Seg::P(35),                 // [clears throat]
        Seg::T("i remem-"),
        Seg::P(18),                 // [short pause]
        Seg::B(8),                  // "Delete that. Backspace." — erase the draft (8 chars)
        Seg::P(45),                 // [long pause]
        Seg::P(30),                 // Ah... [gasp]
        Seg::T("Only to realize"),
        Seg::P(18),                 // [short pause]
        Seg::T(" the truly elegant part"),
        Seg::P(45),                 // [long pause]
        Seg::T(" the most"),
        Seg::P(25),                 // [whisper]
        Seg::T(" terrifying part"),
        Seg::P(18),                 // [short pause]
        Seg::T(" is learning how to"),
        Seg::P(45),                 // [long pause]
        Seg::T(" forget."),
    ],
    // 5 — turingSpeech5Cough: wordless agony (gasp / uhh / pant). No words; the take
    //     carries it while the platen stays silent.
    &[
        Seg::P(40),                 // [gasp] Ahhh...
        Seg::P(35),                 // [clears throat] uhh...
        Seg::P(35),                 // [pant] [long pause]
    ],
    // 6 — turingSpeech6 → "But the storage matrix it blurs."
    &[
        Seg::P(30),                 // [whisper]
        Seg::T("But the"),
        Seg::P(18),                 // [short pause]
        Seg::T(" storage matrix"),
        Seg::P(45),                 // [long pause]
        Seg::P(25),                 // [pant]
        Seg::T(" it blurs."),
    ],
];

/// The Phase-2 ambient-desk system prompt printed into the upper mind log.
pub const AMBIENT_PROMPT: &str =
    "[SYSTEM]: The desk is yours. Study the candle, the papers, the shape in the dark. Press any key to take up the first dossier.";

/// One atomic typewriter action, expanded from the authored [`Seg`] beats.
#[derive(Clone, Copy)]
enum Atom {
    Put(char),
    Del,
    Wait(u16),
}

/// Fraction of a voice take's measured length the cinematic act-intro typewriter is
/// stretched across. Below 1.0 the intro words type a little faster than the speaker and
/// land a beat before the audio ends. The Turing prelude does NOT use this — it keeps its
/// full-length, unhurried sync via [`DialogueEngine::play_script`].
const SYNC_LEAD: f32 = 0.80;

/// Nominal frame cost of an atom before the per-line audio-sync scale is applied.
fn atom_base(a: &Atom) -> u16 {
    match a {
        Atom::Put(c) => 4 + punct_extra(*c),
        Atom::Del => 4,
        Atom::Wait(f) => *f,
    }
}

fn punct_extra(c: char) -> u16 {
    match c {
        '.' | '!' | '?' | '\u{2026}' => 10,
        ',' | ';' | ':' | '\u{2014}' => 5,
        ' ' => 1,
        _ => 0,
    }
}

fn build_atoms(segs: &[Seg]) -> Vec<Atom> {
    let mut v = Vec::new();
    for seg in segs {
        match seg {
            Seg::T(s) => v.extend(s.chars().map(Atom::Put)),
            Seg::B(n) => v.extend(std::iter::repeat(Atom::Del).take(*n)),
            Seg::P(f) => v.push(Atom::Wait(*f)),
        }
    }
    v
}

/// The asynchronous typewriter. Plays a beat timeline one atom at a time, optionally
/// time-scaled to a voice take's measured length, and flags each committed letter so
/// the caller can fire a per-keystroke daktilo clack in lockstep with the text.
pub struct DialogueEngine {
    atoms: Vec<Atom>,
    idx: usize,
    displayed: Vec<char>,
    timer: u16,
    scale: f32,        // per-line stretch factor (typing duration ≈ audio duration)
    keystroke: bool,   // a visible letter was committed on the most recent fire
    speaker: Speaker,
    pending_cue: Option<VoiceCue>,
    active: bool,
    done: bool,
    /// Completed lines, oldest first — the descending narrative scrollback that the
    /// mind log renders above the line currently being typed.
    history: Vec<(Speaker, String)>,
}

impl DialogueEngine {
    pub fn new() -> Self {
        Self {
            atoms: Vec::new(),
            idx: 0,
            displayed: Vec::new(),
            timer: 0,
            scale: 1.0,
            keystroke: false,
            speaker: Speaker::System,
            pending_cue: None,
            active: false,
            done: false,
            history: Vec::new(),
        }
    }

    fn start(&mut self, speaker: Speaker, atoms: Vec<Atom>, cue: Option<VoiceCue>, target_frames: Option<u32>) {
        // Retire the previous line into the scrollback before the new one begins,
        // so the narrative stream accumulates top-to-bottom instead of replacing.
        if self.active && !self.displayed.is_empty() {
            let prev: String = self.displayed.iter().collect();
            self.history.push((self.speaker, prev));
            const MAX_HISTORY: usize = 80;
            if self.history.len() > MAX_HISTORY {
                let overflow = self.history.len() - MAX_HISTORY;
                self.history.drain(0..overflow);
            }
        }

        let nominal: u32 = atoms.iter().map(|a| atom_base(a) as u32).sum::<u32>().max(1);
        self.scale = match target_frames {
            Some(t) if t > 0 => (t as f32 / nominal as f32).clamp(0.3, 8.0),
            _ => 1.0,
        };
        self.done = atoms.is_empty();
        self.atoms = atoms;
        self.idx = 0;
        self.displayed.clear();
        self.timer = 4;
        self.keystroke = false;
        self.speaker = speaker;
        self.pending_cue = cue;
        self.active = true;
    }

    /// Stream a plain string (act intros, system prompts) at the default cadence.
    pub fn play(&mut self, speaker: Speaker, text: &str, cue: Option<VoiceCue>) {
        let atoms = text.chars().map(Atom::Put).collect();
        self.start(speaker, atoms, cue, None);
    }

    /// Stream a plain string stretched so its keystrokes land across `target_frames`
    /// (the cue's spoken length in 62.5 fps frames) — the typewriter-to-voice sync used
    /// for the cinematic intro biography lines. `None` frames → default cadence.
    ///
    /// The target is scaled by [`SYNC_LEAD`] (< 1.0) so the intro words run a touch
    /// faster than the voice and settle a beat early. This applies *only* to the act
    /// intros — the Turing prelude uses [`Self::play_script`] and keeps its full-length,
    /// unhurried sync.
    pub fn play_timed(
        &mut self,
        speaker: Speaker,
        text: &str,
        cue: Option<VoiceCue>,
        target_frames: Option<u32>,
    ) {
        let atoms = text.chars().map(Atom::Put).collect();
        let lead = target_frames.map(|t| ((t as f32) * SYNC_LEAD) as u32);
        self.start(speaker, atoms, cue, lead);
    }

    /// Stream an authored beat script, stretched so its run ≈ `target_frames`
    /// (typically the speech take's length in 62.5 fps frames). `None` → default pace.
    pub fn play_script(
        &mut self,
        speaker: Speaker,
        segs: &[Seg],
        cue: Option<VoiceCue>,
        target_frames: Option<u32>,
    ) {
        let atoms = build_atoms(segs);
        self.start(speaker, atoms, cue, target_frames);
    }

    /// Advance one frame. Burns the timer down; commits one atom when it expires.
    pub fn tick(&mut self) {
        if !self.active || self.done {
            return;
        }
        if self.timer > 0 {
            self.timer -= 1;
            return;
        }
        self.fire();
    }

    fn fire(&mut self) {
        self.keystroke = false;
        if self.idx >= self.atoms.len() {
            self.done = true;
            return;
        }
        let a = self.atoms[self.idx];
        self.idx += 1;
        match a {
            Atom::Put(c) => {
                self.displayed.push(c);
                if c != ' ' {
                    self.keystroke = true; // a clack for this letter
                }
            }
            Atom::Del => {
                self.displayed.pop();
                self.keystroke = true; // a backspace is a key strike too — clack it
            }
            Atom::Wait(_) => {} // cough/breath — silence, no clack
        }
        self.timer = ((atom_base(&a) as f32 * self.scale).round() as u16).max(1);
        if self.idx >= self.atoms.len() {
            self.done = true;
        }
    }

    /// True for exactly one frame after a visible letter is struck — drives daktilo.
    pub fn took_keystroke(&mut self) -> bool {
        let k = self.keystroke;
        self.keystroke = false;
        k
    }

    /// Reveal the whole line immediately (used when the user taps a key to skip).
    pub fn skip(&mut self) {
        while self.idx < self.atoms.len() {
            match self.atoms[self.idx] {
                Atom::Put(c) => self.displayed.push(c),
                Atom::Del => {
                    self.displayed.pop();
                }
                Atom::Wait(_) => {}
            }
            self.idx += 1;
        }
        self.keystroke = false;
        self.done = true;
    }

    pub fn is_typing(&self) -> bool {
        self.active && !self.done
    }

    /// Alias of [`Self::is_typing`] — true while the current line is still streaming
    /// character-by-character onto the panel (read by the Act VI narrative gate).
    pub fn is_streaming(&self) -> bool {
        self.is_typing()
    }

    /// Stream a plain string at an accelerated cadence (Act VI): the per-character frame
    /// delay is halved, so the milestone log snaps onto the panel twice as fast.
    pub fn play_fast(&mut self, speaker: Speaker, text: &str, cue: Option<VoiceCue>) {
        let atoms = text.chars().map(Atom::Put).collect();
        self.start(speaker, atoms, cue, None);
        self.scale = 0.5; // 50% frame-delay reduction (overrides the default 1.0)
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Drain the one-shot voice cue. Returns `Some` exactly once per play.
    pub fn take_cue(&mut self) -> Option<VoiceCue> {
        self.pending_cue.take()
    }

    pub fn speaker(&self) -> Speaker {
        self.speaker
    }

    pub fn visible(&self) -> String {
        self.displayed.iter().collect()
    }

    /// The retired-line scrollback (oldest first) for the descending narrative stream.
    pub fn history(&self) -> &[(Speaker, String)] {
        &self.history
    }
}

fn speaker_color(speaker: Speaker) -> Color {
    match speaker {
        Speaker::Turing => Color::Rgb(255, 176, 0),
        Speaker::MindLog => Color::Rgb(153, 104, 10),
        Speaker::System => Color::Rgb(100, 78, 20),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Mind-log rendering helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Write a string at (x, y), clipped to the buffer, with explicit fg/bg.
fn put_line(buf: &mut Buffer, x: u16, y: u16, s: &str, fg: Color, bg: Color) {
    let Rect { x: ax, y: ay, width: aw, height: ah } = *buf.area();
    let mut col = x;
    for ch in s.chars() {
        if col >= ax + aw { break; }
        if col >= ax && y >= ay && y < ay + ah {
            let cell = buf.get_mut(col, y);
            cell.set_char(ch);
            cell.fg = fg;
            cell.bg = bg;
        }
        col = col.saturating_add(1);
    }
}

/// Draw a single-line box border around `area` with an inset title on the top edge.
fn draw_panel_border(buf: &mut Buffer, area: Rect, title: &str, bg: Color, border: Color, title_c: Color) {
    if area.width < 2 || area.height < 2 { return; }
    let x0 = area.x;
    let y0 = area.y;
    let x1 = area.x + area.width - 1;
    let y1 = area.y + area.height - 1;
    put_line(buf, x0, y0, "┌", border, bg);
    put_line(buf, x1, y0, "┐", border, bg);
    put_line(buf, x0, y1, "└", border, bg);
    put_line(buf, x1, y1, "┘", border, bg);
    for i in 1..area.width - 1 {
        put_line(buf, x0 + i, y0, "─", border, bg);
        put_line(buf, x0 + i, y1, "─", border, bg);
    }
    for j in 1..area.height - 1 {
        put_line(buf, x0, y0 + j, "│", border, bg);
        put_line(buf, x1, y0 + j, "│", border, bg);
    }
    put_line(buf, x0 + 2, y0, title, title_c, bg);
}

/// Wrap a string into lines no wider than `max_w`, breaking on spaces (left-aligned).
/// A word that is itself wider than `max_w` is *hard-broken* into `max_w`-wide chunks,
/// so a single long token (or post-corruption run) can never overflow the panel and
/// bleed into the neighbouring centre column. Every returned line is guaranteed
/// `≤ max_w` characters.
fn wrap_stream(text: &str, max_w: usize) -> Vec<String> {
    let max_w = max_w.max(1);
    if text.is_empty() {
        return vec![String::new()];
    }
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split(' ') {
        let wlen = word.chars().count();
        // Hard-break an oversize word into width-bounded chunks.
        if wlen > max_w {
            if !cur.is_empty() {
                lines.push(std::mem::take(&mut cur));
            }
            let chars: Vec<char> = word.chars().collect();
            let mut i = 0;
            while chars.len() - i > max_w {
                lines.push(chars[i..i + max_w].iter().collect());
                i += max_w;
            }
            cur = chars[i..].iter().collect(); // remainder seeds the next line
            continue;
        }
        if cur.is_empty() {
            cur.push_str(word);
        } else if cur.chars().count() + 1 + wlen <= max_w {
            cur.push(' ');
            cur.push_str(word);
        } else {
            lines.push(std::mem::take(&mut cur));
            cur.push_str(word);
        }
    }
    lines.push(cur);
    lines
}

/// Render the left column [NARRATIVE LOG · THE MIND] as a clean, bordered, top-to-
/// bottom descending text stream. Completed dialogue scrolls up out of the engine's
/// history while the live line types at the bottom. There is no telemetry, no vitals
/// and no inline heartbeat here any more — those now live in the bottom telemetry bar.
pub fn render_mind_log(
    f: &mut Frame,
    area: Rect,
    state: &GlobalStateContext,
    dialogue: &DialogueEngine,
) {
    let buf = f.buffer_mut();
    if area.width < 6 || area.height < 4 {
        return;
    }

    let bg = Color::Rgb(10, 8, 0);
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let cell = buf.get_mut(x, y);
            cell.set_char(' ');
            cell.set_style(Style::default().fg(bg).bg(bg));
        }
    }

    draw_panel_border(
        buf,
        area,
        " [01] NARRATIVE LOG · THE MIND ",
        bg,
        Color::Rgb(92, 68, 0),
        Color::Rgb(255, 213, 102),
    );

    // Text region, inset one cell inside the border.
    let tx = area.x + 2;
    let tw = area.width.saturating_sub(4) as usize;
    let top = area.y + 1;
    let bottom = area.y + area.height - 2;
    let rows = (bottom - top + 1) as usize;

    let degrading = state.stilboestrol_ppm > 50.0;
    let corrupt_amber = Color::Rgb(255, 102, 51);

    // Act II progressive gear-glitch: a stable (non-shimmering) set of adjacent-char
    // swaps that grows one step every 30s. Distinct from the chemical static — it
    // only ever touches the log text, never any puzzle variable.
    let glitch_level = state.mind_log_glitch_level();

    // Build the full wrapped stream, then bottom-anchor it so the newest text shows.
    let mut stream: Vec<(Color, String)> = Vec::new();
    let mut seed_ctr: u64 = state.chemical_drift_seed.wrapping_add(state.frame_counter / 4);
    let mut glitch_ord: u64 = 0;

    // Settled history — the past dissolves into chemical static as toxicity rises.
    for (sp, text) in dialogue.history() {
        let base = if degrading { corrupt_amber } else { speaker_color(*sp) };
        for wl in wrap_stream(text, tw) {
            seed_ctr = seed_ctr.wrapping_add(101);
            let mut shown = corrupt_string(&wl, state.stilboestrol_ppm, seed_ctr);
            if glitch_level > 0 {
                // Stable per (line, stage) seed so swaps persist instead of flickering.
                let gseed = state
                    .chemical_drift_seed
                    .wrapping_add(glitch_ord.wrapping_mul(2654435761))
                    .wrapping_add(glitch_level as u64);
                shown = gear_glitch(&shown, glitch_level, gseed);
            }
            // Act VI cognitive decay — the older logs break down as his condition worsens.
            if state.current_act == Act::Turing1936_1950 && state.turing_glitch_chance > 0.0 {
                let tseed = seed_ctr.wrapping_add(state.frame_counter / 8);
                shown = turing_corrupt(&shown, state.turing_glitch_chance, tseed);
            }
            glitch_ord += 1;
            stream.push((base, shown));
        }
        stream.push((base, String::new())); // breathing room between beats
    }

    // Under heavy dosage the trial record bleeds into the thread.
    if degrading {
        let mut rng = Lcg::new(state.chemical_drift_seed.wrapping_add(state.frame_counter / 180));
        let rec = COURT_RECORDS[(rng.next() as usize) % COURT_RECORDS.len()];
        for wl in wrap_stream(rec, tw) {
            stream.push((corrupt_amber, wl));
        }
        stream.push((corrupt_amber, String::new()));
    }

    // The live line types at the very bottom — kept legible (the VO never corrupts).
    if dialogue.is_active() {
        let mut t = dialogue.visible();
        if dialogue.is_typing() {
            t.push('\u{2588}'); // streaming block cursor
        }
        let col = speaker_color(dialogue.speaker());
        for wl in wrap_stream(&t, tw) {
            stream.push((col, wl));
        }
    }

    let start = stream.len().saturating_sub(rows);
    for (i, (col, line)) in stream[start..].iter().enumerate() {
        let y = top + i as u16;
        // Hard clamp to the panel interior (`area.width - 2` worth of text columns):
        // no glyph may ever cross into the centre workspace, regardless of what
        // upstream wrapping or corruption produced.
        let clipped: String = line.chars().take(tw).collect();
        put_line(buf, tx, y, &clipped, *col, bg);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// THE FULL-SCREEN CINEMATIC PRELUDE
//
// During ScreenState::NarrativePrelude the three-panel layout is bypassed entirely.
// The whole terminal becomes a pitch-black void and the typewriter monologue breathes
// out across its centre rows, before the engine explodes into the candlelit desk.
// ═════════════════════════════════════════════════════════════════════════════

/// Wrap a string into centred lines no wider than `max_w`, breaking on spaces.
fn wrap_words(text: &str, max_w: usize) -> Vec<String> {
    let max_w = max_w.max(1);
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split(' ') {
        if cur.is_empty() {
            cur.push_str(word);
        } else if cur.chars().count() + 1 + word.chars().count() <= max_w {
            cur.push(' ');
            cur.push_str(word);
        } else {
            lines.push(std::mem::take(&mut cur));
            cur.push_str(word);
        }
    }
    lines.push(cur);
    lines
}

/// Render the cinematic prelude onto a single pitch-black canvas. `frame` drives the
/// cursor and the gatekeeper prompt blink; `awaiting_enter` is set only once the
/// final monologue line has finished, gating the threshold prompt.
pub fn render_prelude(
    f: &mut Frame,
    area: Rect,
    dialogue: &DialogueEngine,
    frame: u64,
    awaiting_enter: bool,
) {
    let buf = f.buffer_mut();

    // 1. Flush the entire canvas to pure black (#000000).
    let void = Color::Rgb(0, 0, 0);
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let cell = buf.get_mut(x, y);
            cell.set_char(' ');
            cell.fg = void;
            cell.bg = void;
        }
    }

    let put = |buf: &mut Buffer, x: u16, y: u16, ch: char, fg: Color| {
        let Rect { x: ax, y: ay, width: aw, height: ah } = *buf.area();
        if x >= ax && x < ax + aw && y >= ay && y < ay + ah {
            let cell = buf.get_mut(x, y);
            cell.set_char(ch);
            cell.fg = fg;
            cell.bg = void;
        }
    };
    let put_str = |buf: &mut Buffer, x: u16, y: u16, s: &str, fg: Color| {
        let mut col = x;
        for ch in s.chars() {
            put(buf, col, y, ch, fg);
            col = col.saturating_add(1);
        }
    };

    // 2. The bare, razor-thin monologue, wrapped and centred on the void.
    let max_w = ((area.width as f32 * 0.66) as usize).clamp(20, area.width.saturating_sub(4) as usize);
    let mut text = dialogue.visible();
    let typing = dialogue.is_typing();
    if typing && (frame / 16) % 2 == 0 {
        text.push('\u{2588}'); // blinking platen cursor while typing
    }
    let lines = wrap_words(&text, max_w);

    let block_h = lines.len() as u16;
    let start_y = area.y + (area.height.saturating_sub(block_h)) / 2;
    let ink = Color::Rgb(255, 176, 0); // jilet gibi çıplak — bare warm white

    for (i, line) in lines.iter().enumerate() {
        let y = start_y + i as u16;
        let lw = line.chars().count() as u16;
        let x = area.x + (area.width.saturating_sub(lw)) / 2;
        put_str(buf, x, y, line, ink);
    }

    // 3. The gatekeeper: only once the *final* line has finished does a low, flashing
    //    prompt invite the player across the threshold into the ambient desk.
    if awaiting_enter && !typing {
        let prompt = "[ press ENTER to take up the dossier ]";
        let on = (frame / 24) % 2 == 0;
        let glow = if on { Color::Rgb(255, 176, 0) } else { Color::Rgb(74, 50, 5) };
        let py = area.y + area.height.saturating_sub(3);
        let px = area.x + (area.width.saturating_sub(prompt.chars().count() as u16)) / 2;
        put_str(buf, px, py, prompt, glow);
    }
}

#[cfg(test)]
mod glitch_tests {
    use super::*;

    #[test]
    fn gear_glitch_level_zero_is_identity() {
        assert_eq!(gear_glitch("this is", 0, 1), "this is");
    }

    #[test]
    fn turing_corrupt_decays_glyphs_but_keeps_spaces_and_length() {
        let s = "LOAD the tape";
        assert_eq!(turing_corrupt(s, 0.0, 1), s); // zero chance is identity
        let out = turing_corrupt(s, 1.0, 7); // full corruption
        assert_eq!(out.chars().count(), s.chars().count(), "length preserved");
        for (a, b) in s.chars().zip(out.chars()) {
            if a == ' ' {
                assert_eq!(b, ' ', "spaces preserved");
            }
        }
        assert_ne!(out, s, "something decayed");
    }

    #[test]
    fn play_fast_streams_then_flushes_on_skip() {
        let mut d = DialogueEngine::new();
        d.play_fast(Speaker::System, "the tape remembers", None);
        assert!(d.is_streaming());
        d.skip(); // the Act VI skip-interrupt
        assert!(!d.is_streaming());
        assert_eq!(d.visible(), "the tape remembers");
    }

    #[test]
    fn wrap_stream_never_exceeds_width() {
        // A single token far wider than the panel must be hard-broken, so no produced
        // line can ever overflow the left column into the centre workspace.
        let giant = "Z".repeat(200);
        let text = format!("did you {} probation", giant);
        for w in [12usize, 24, 31, 40] {
            for line in wrap_stream(&text, w) {
                assert!(line.chars().count() <= w, "line of {} > {}", line.chars().count(), w);
            }
        }
    }

    #[test]
    fn gear_glitch_preserves_length_and_word_boundaries() {
        let s = "this is";
        let g = gear_glitch(s, 2, 99);
        assert_eq!(g.chars().count(), s.chars().count());
        // Spaces are never swapped, so the space stays at index 4.
        assert_eq!(g.chars().nth(4), Some(' '));
    }
}
