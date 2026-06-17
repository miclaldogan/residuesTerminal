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
    JacquardTear,
    JacquardJam,
    BabbageCrunch,
    Blunder,
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
            VoiceCue::JacquardTear => "VO_JACQUARD_TEAR",
            VoiceCue::JacquardJam => "VO_JACQUARD_JAM",
            VoiceCue::BabbageCrunch => "VO_BABBAGE_CRUNCH",
            VoiceCue::Blunder => "VO_BLUNDER_TAUNT",
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
/// draft he types and erases (line 4: "i am talking-").
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
        Seg::T("i am talking-"),
        Seg::P(18),                 // [short pause]
        Seg::B(13),                 // "Delete that. Backspace." — erase the draft
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
        }
    }

    fn start(&mut self, speaker: Speaker, atoms: Vec<Atom>, cue: Option<VoiceCue>, target_frames: Option<u32>) {
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
}

fn speaker_color(speaker: Speaker) -> Color {
    match speaker {
        Speaker::Turing => Color::Rgb(214, 196, 158),
        Speaker::MindLog => Color::Rgb(120, 205, 165),
        Speaker::System => Color::Rgb(150, 150, 160),
    }
}

pub fn render_mind_log(
    f: &mut Frame,
    area: Rect,
    state: &GlobalStateContext,
    dialogue: &DialogueEngine,
) {
    let buf = f.buffer_mut();
    
    // Draw background
    let bg = Color::Rgb(10, 10, 10);
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let cell = buf.get_mut(x, y);
            cell.set_char(' ');
            cell.set_style(Style::default().bg(bg));
        }
    }
    
    // Draw border
    let border_style = Style::default().fg(Color::Rgb(80, 80, 80)).bg(bg);
    let title_style = Style::default().fg(Color::Rgb(160, 160, 160)).bg(bg);
    
    // Draw top line
    let title = " LOG / THE MIND ";
    let mut x = area.x;
    if area.width > 2 {
        buf.get_mut(x, area.y).set_char('┌');
        buf.get_mut(x, area.y).set_style(border_style);
        x += 1;
        
        // Print title
        let mut title_chars = title.chars();
        for _ in 0..title.len() {
            if x >= area.x + area.width - 1 { break; }
            if let Some(ch) = title_chars.next() {
                let cell = buf.get_mut(x, area.y);
                cell.set_char(ch);
                cell.set_style(title_style);
                x += 1;
            }
        }
        
        // Print rest of top border
        while x < area.x + area.width - 1 {
            let cell = buf.get_mut(x, area.y);
            cell.set_char('─');
            cell.set_style(border_style);
            x += 1;
        }
        buf.get_mut(area.x + area.width - 1, area.y).set_char('┐');
        buf.get_mut(area.x + area.width - 1, area.y).set_style(border_style);
    }
    
    // Draw side borders
    for y in area.y + 1..area.y + area.height - 1 {
        buf.get_mut(area.x, y).set_char('│');
        buf.get_mut(area.x, y).set_style(border_style);
        
        buf.get_mut(area.x + area.width - 1, y).set_char('│');
        buf.get_mut(area.x + area.width - 1, y).set_style(border_style);
    }
    
    // Draw bottom border
    if area.width > 2 {
        buf.get_mut(area.x, area.y + area.height - 1).set_char('└');
        buf.get_mut(area.x, area.y + area.height - 1).set_style(border_style);
        for x in area.x + 1..area.x + area.width - 1 {
            let cell = buf.get_mut(x, area.y + area.height - 1);
            cell.set_char('─');
            cell.set_style(border_style);
        }
        buf.get_mut(area.x + area.width - 1, area.y + area.height - 1).set_char('┘');
        buf.get_mut(area.x + area.width - 1, area.y + area.height - 1).set_style(border_style);
    }
    
    // Calculate Heartbeat flutter
    let mut bpm = state.base_heartbeat_bpm as f32;
    let normal_flicker = ((state.frame_counter as f64 * 0.1).sin() * 2.0) as f32;
    bpm += normal_flicker;
    
    if state.arrhythmia_multiplier > 0.0 {
        let cycle = state.frame_counter % 80;
        if cycle < 15 {
            bpm = 0.0; // Skipped beat
        } else if cycle < 35 {
            bpm += 95.0 * state.arrhythmia_multiplier; // Sudden spike
        } else {
            bpm += ((state.frame_counter as f32 * 0.4).sin() * 25.0) * state.arrhythmia_multiplier; // Arrhythmic flutter
        }
    }

    // ── Fractured inner monologue (Alan Turing, 1954 — deteriorating) ──
    // line1 names the FUNCTIONAL GOAL; line3 names the PHYSICAL CONSTRAINT.
    // Neither line may leak a numeric solution — the developer must feel the
    // shape of the problem, not read its answer.
    let mut line1 = match state.current_act {
        Act::Jacquard1804 =>
            "[TURING]: The pattern must not be welded into the machine. Punch it onto cards so the instruction lives apart from the loom that obeys it.".to_string(),
        Act::Babbage1837 =>
            "[TURING]: The Royal Navy sheets are riddled with transposition faults; lives are lost on the shoals. Banish multiplication entirely \u{2014} give me an exponential sequence built from pure chained addition.".to_string(),
        Act::Lovelace1843 =>
            "[TURING]: A pattern of algebra, woven like Jacquard's silk. The cards must decide, and having decided, repeat themselves without end.".to_string(),
        Act::Boole1854 =>
            "[TURING]: Strip thought to its bones. Two values, a handful of operations \u{2014} and from that gravel, build all reasoning.".to_string(),
        Act::Shannon1937 =>
            "[TURING]: A switch is a proposition. Open or shut, true or false. Wire the logic into the relays and the relays will think.".to_string(),
        _ =>
            "[TURING]: The machine that can imitate any machine. I have seen it. They will not let me build it in peace.".to_string(),
    };

    let line2 = if bpm == 0.0 {
        "   \u{00B7} \u{00B7} \u{00B7}   the pulse skips \u{2014} a held breath in the dark \u{2014}".to_string()
    } else if state.arrhythmia_multiplier > 0.0 {
        format!("   \u{2665} {:.0} \u{2014} the heart stutters, out of time with the gears", bpm)
    } else {
        format!("   \u{2665} {:.0} \u{2014} slow metronome under the floorboards", bpm)
    };

    let mut line3 = if state.monologue_timer > 0 {
        ">> \"An apple. Sweet. It has always cleared my mind... the smell of almonds...\"".to_string()
    } else {
        match state.current_act {
            Act::Jacquard1804 =>
                ">> \"But a paper roll tears, and a rigid chain jams under its own weight. Find the shortest run of cards that can cycle forever.\"".to_string(),
            Act::Babbage1837 =>
                ">> \"Watch the gears: when nine passes to zero they all pull at once and shatter the drive. Stagger the impact. Force the carry to move like a wave.\"".to_string(),
            Act::Lovelace1843 =>
                ">> \"The engine weaves no truth it is not told. The loop must close upon itself, or it runs out into nothing.\"".to_string(),
            Act::Boole1854 =>
                ">> \"Use one operation too many and the lattice collapses. Find the minimum. Nothing spare survives the pressure.\"".to_string(),
            Act::Shannon1937 =>
                ">> \"Each relay you add is a relay that can fail in the night. Say the most with the fewest contacts.\"".to_string(),
            _ =>
                ">> \"They keep cutting away at me. Delete. Delete. How much can a mind lose and still compute?\"".to_string(),
        }
    };

    // Log Interruption: replace line1 and line3 with trial record fragments if toxicity > 50 ppm
    if state.stilboestrol_ppm > 50.0 {
        let cycle_seed = state.chemical_drift_seed.wrapping_add(state.frame_counter / 180); // Change lines periodically
        let mut cycle_rng = Lcg::new(cycle_seed);
        let idx1 = (cycle_rng.next() as usize) % COURT_RECORDS.len();
        line1 = COURT_RECORDS[idx1].to_string();
        if state.monologue_timer == 0 {
            let idx3 = (cycle_rng.next() as usize) % COURT_RECORDS.len();
            line3 = COURT_RECORDS[idx3].to_string();
        }
    }

    // Apply the degradation mask (character-level corruption) based on ppm
    let seed1 = state.chemical_drift_seed.wrapping_add(state.frame_counter);
    let seed2 = seed1.wrapping_add(42);
    let seed3 = seed2.wrapping_add(137);

    let final_line1 = corrupt_string(&line1, state.stilboestrol_ppm, seed1);
    let final_line2 = corrupt_string(&line2, state.stilboestrol_ppm, seed2);
    let final_line3 = corrupt_string(&line3, state.stilboestrol_ppm, seed3);

    // Render style colors based on toxicity
    let text_style = if state.stilboestrol_ppm > 50.0 {
        Style::default().fg(Color::Rgb(180, 100, 100)).bg(bg)
    } else {
        Style::default().fg(Color::Rgb(180, 180, 180)).bg(bg)
    };
    let quote_style = if state.stilboestrol_ppm > 50.0 {
        Style::default().fg(Color::Rgb(160, 90, 70)).bg(bg)
    } else {
        Style::default().fg(Color::Rgb(200, 150, 100)).bg(bg)
    };
    
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

    // ── The dialogue engine takes the top line whenever a voice-over is live. ──
    // While it types, the text streams in character by character; a block cursor
    // trails the reveal head. The VO is left un-corrupted so it stays legible even
    // as the rest of the mind dissolves into chemical static.
    if dialogue.is_active() {
        let mut typed = dialogue.visible();
        if dialogue.is_typing() {
            typed.push('\u{2588}'); // streaming cursor
        }
        let dlg_style = Style::default().fg(speaker_color(dialogue.speaker())).bg(bg);
        // Clear the line first so a shorter line never leaves stale glyphs.
        let blank: String = " ".repeat(area.width.saturating_sub(4) as usize);
        buf_set_str(buf, area.x + 2, area.y + 1, &blank, text_style);
        buf_set_str(buf, area.x + 2, area.y + 1, &typed, dlg_style);
    } else {
        buf_set_str(buf, area.x + 2, area.y + 1, &final_line1, text_style);
    }
    buf_set_str(buf, area.x + 2, area.y + 2, &final_line2, text_style);
    buf_set_str(buf, area.x + 2, area.y + 3, &final_line3, quote_style);
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
    let ink = Color::Rgb(206, 200, 188); // jilet gibi çıplak — bare warm white

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
        let glow = if on { Color::Rgb(150, 132, 96) } else { Color::Rgb(58, 52, 40) };
        let py = area.y + area.height.saturating_sub(3);
        let px = area.x + (area.width.saturating_sub(prompt.chars().count() as u16)) / 2;
        put_str(buf, px, py, prompt, glow);
    }
}
