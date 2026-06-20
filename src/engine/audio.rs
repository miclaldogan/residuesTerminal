use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// Resolve the `audio/` asset directory at runtime so the binary works from any CWD
/// and survives being moved/distributed: first look for an `audio/` folder next to the
/// executable (the shipping layout), then fall back to the crate source tree (dev).
fn resolve_audio_base() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("audio");
            if candidate.is_dir() {
                return candidate;
            }
        }
    }
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/audio"))
}

// ─────────────────────────────────────────────────────────────────────────────
// Crate-free audio: we drive detached command-line players (mpv / ffplay / mpg123)
// plus low-latency `paplay` for keystrokes. The TUI never touches the audio device
// directly — it spawns and reaps short-lived child processes, redirecting every
// child's stdio to /dev/null so the alternate screen is never corrupted.
//
// Concurrency model: `AudioEngine` is owned and touched by exactly one thread (the
// main loop). There are no Rust threads, channels, locks or shared mutable state —
// the only "async" is the detached OS processes — so there are no data races and no
// channel deadlocks possible, even under fast keyboard input. Process handles are
// pooled and reaped lazily so nothing piles up or zombifies.
// ─────────────────────────────────────────────────────────────────────────────

// ── Asset paths (relative to `audio/`) ──────────────────────────────────────
/// Perpetual looping ambient beds: `(relative path, volume%)`. Raised from the old
/// 15% "whisper" because at that level the rain, the low bump and the background score
/// were inaudible under the clacks — these now sit as a present-but-background layer.
const AMBIENT_TRACKS: &[(&str, u8)] = &[
    ("ambient/rain_wilmslow_loop.mp3", 55),
    ("ambient/backgroundMusic.mp3", 62),
    ("sfx/bump.mp3", 72),
    ("ambient/clock_pendulum_loop.mp3", 45),
];
/// Short, single typewriter clack — one strike per committed character.
const SFX_KEY: &str = "sfx/daktiloOne.mp3";
/// High-frequency clatter for rapid binary entry (Shannon bit masks).
const SFX_KEY_FAST: &str = "sfx/daktiloFast.mp3";
/// Heavy, deliberate mechanical click — the Turing tape head sliding cell to cell.
const SFX_HEAD: &str = "sfx/Slow_deliberate_key__#4-1781700108983.mp3";
/// Ada's backspace — the clean, ultra-short mechanical snap (the long #1 take looped
/// for ~5 s and bled over the editing flow; #2 is clamped to a tight ~200 ms window).
const SFX_BACKSPACE: &str = "sfx/daktilo_backspace_snap#2.mp3";
const SFX_GLITCH: &str = "sfx/electrical_short_glitch.mp3";
const SFX_HEARTBEAT: &str = "sfx/heartbeat_base.mp3";

// ── Tactile interface SFX ────────────────────────────────────────────────────
/// Short subtle click on main-menu navigation (up/down/WASD).
const SFX_MENU_NAV: &str = "sfx/menu_nav.mp3";
/// Vintage terminal key-tap on every puzzle-grid cursor move (Arrows/WASD).
const SFX_GRID_NAV: &str = "sfx/grid_nav.mp3";
/// Heavy mechanical latch — menu selection + Space heavy state toggles (gate cycle, locks).
const SFX_MENU_CONFIRM: &str = "sfx/menu_confirm.mp3";
/// Rhythmic wooden shuttle slide — Jacquard loom card/carriage operations.
const SFX_LOOM: &str = "sfx/loom_shuttle.mp3";
/// Rolling brass cog sequence — the Babbage engine cranking/compiling.
const SFX_GEARS: &str = "sfx/babbage_gears.mp3";
/// Sharp metallic gear jam — a Babbage out-of-phase calculation fault.
const SFX_GEAR_JAM: &str = "sfx/daktilo_clack_fault.mp3";

// ── Volume ceilings (player-percent, 100 = nominal) ─────────────────────────
const HEARTBEAT_VOLUME: u8 = 62;
const BACKSPACE_VOLUME: u8 = 88;
const GLITCH_VOLUME: u8 = 92;
const SPEECH_VOLUME: u8 = 100;
const KEY_FALLBACK_VOLUME: u8 = 80;
/// Cap on simultaneously-sounding clacks, so mashing a key can't pile detached players
/// into an overlapping smear — extra strikes past this are dropped until some finish.
const MAX_CONCURRENT_CLACKS: usize = 4;
/// Same idea for the one-shot interface SFX pool (nav taps, latches, shuttles).
const MAX_CONCURRENT_SFX: usize = 6;
/// paplay linear volume scale: 0x10000 == 100%.
const PAPLAY_FULL: u32 = 65_536;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Player {
    Mpv,
    Ffplay,
    Mpg123,
    None,
}

fn detect_player() -> Player {
    let candidates = [
        ("mpv", Player::Mpv),
        ("ffplay", Player::Ffplay),
        ("mpg123", Player::Mpg123),
    ];
    for (bin, player) in candidates {
        let ok = Command::new(bin)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok();
        if ok {
            return player;
        }
    }
    Player::None
}

/// Map a deterministic [`VoiceCue`](crate::engine::mind_log::VoiceCue) marker onto a
/// concrete asset path under `audio/`. Returns `None` for cues with no sample.
fn marker_to_rel(marker: &str) -> Option<&'static str> {
    Some(match marker {
        "VO_TURING_SPEECH_1" => "turing/turingSpeech1.mp3",
        "VO_TURING_SPEECH_2" => "turing/turingSpeech2.mp3",
        "VO_TURING_SPEECH_3" => "turing/turingSpeech3.mp3",
        "VO_TURING_SPEECH_4" => "turing/turingSpeech4.mp3",
        "VO_TURING_SPEECH_5" => "turing/turingSpeech5Cough.mp3",
        "VO_TURING_SPEECH_6" => "turing/turingSpeech6.mp3",
        // Cinematic act-intro biography takes — two spoken lines per act, synced to the
        // typewriter. Turing has no take, so its markers fall through to `None` (silent).
        "VO_INTRO_JACQUARD_1" => "speechs/jacquard_intro1.mp3",
        "VO_INTRO_JACQUARD_2" => "speechs/jacquard_intro2.mp3",
        "VO_INTRO_BABBAGE_1" => "speechs/babbage_intro1.mp3",
        "VO_INTRO_BABBAGE_2" => "speechs/babbage_intro2.mp3",
        "VO_INTRO_LOVELACE_1" => "speechs/lovelace_intro1.mp3",
        "VO_INTRO_LOVELACE_2" => "speechs/lovelace_intro2.mp3",
        "VO_INTRO_BOOLE_1" => "speechs/boole_intro1.mp3",
        "VO_INTRO_BOOLE_2" => "speechs/boole_intro2.mp3",
        "VO_INTRO_SHANNON_1" => "speechs/shannon_intro1.mp3",
        "VO_INTRO_SHANNON_2" => "speechs/shannon_intro2.mp3",
        "SFX_DOOR_SLIDE" => "sfx/A_single,_isolated_s_#1-1781700210070.mp3",
        // Heavy interrogation bootstep — reuse the deep bump as a one-shot thud.
        "SFX_POLICE_BOOTSTEP" => "sfx/bump.mp3",
        _ => return None,
    })
}

/// The runtime audio mixer. Owns: two looping ambient beds, a BPM-scheduled one-shot
/// heartbeat, a one-shot speech channel, a low-latency typewriter-clack pool, and a
/// generic one-shot SFX pool (backspace snap, electrical glitch).
pub struct AudioEngine {
    player: Player,
    ambient: Vec<Child>,        // perpetual rain + pendulum beds (started past prelude)
    ambient_started: bool,
    heartbeat: Option<Child>,   // one-shot per scheduled beat (replaces a fixed loop)
    speech: Option<Child>,      // one-shot voice line; replaces itself each play
    clacks: Vec<Child>,         // per-letter typewriter strikes, reaped lazily
    sfx: Vec<Child>,            // misc one-shot SFX (backspace, glitch), reaped lazily
    prep: HashMap<&'static str, String>, // pre-trimmed WAVs for low-latency paplay
    base: PathBuf,              // resolved `audio/` directory (CWD-independent)
    muted: bool,                // master mute, toggled from the main menu
}

/// Short SFX to pre-trim into low-latency WAVs at startup: `(rel, max_ms)`. `max_ms`
/// hard-caps the output length (e.g. Ada's backspace → 200 ms). Every one also has its
/// leading silence below −45 dB stripped, so the snap lands frame-perfect on the keypress.
const PREP_SET: &[(&str, Option<u32>)] = &[
    (SFX_KEY, None),
    (SFX_KEY_FAST, None),
    (SFX_BACKSPACE, Some(200)),
    (SFX_MENU_NAV, Some(220)),
    (SFX_GRID_NAV, Some(200)),
    (SFX_MENU_CONFIRM, Some(500)),
    (SFX_LOOM, Some(500)),
    (SFX_GEARS, None),
    (SFX_GEAR_JAM, Some(350)),
];

/// Is `bin` runnable on PATH?
fn has_bin(bin: &str) -> bool {
    Command::new(bin)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

/// Pre-decode one SFX to a WAV with its leading silence (< −45 dB) stripped and, when
/// given, its length hard-clamped. A WAV + `paplay` starts in ~milliseconds with zero
/// leading dead-air, so the feedback lands on the exact tick of the input event.
fn prepare_short(base: &Path, rel: &str, max_ms: Option<u32>) -> Option<String> {
    let src = base.join(rel);
    if !src.is_file() {
        return None;
    }
    let stem: String = rel
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let dst = std::env::temp_dir().join(format!("residues_{}.wav", stem));
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-y", "-loglevel", "quiet", "-i"])
        .arg(&src)
        // Strip leading silence below −45 dB → no input-to-sound latency.
        .arg("-af")
        .arg("silenceremove=start_periods=1:start_threshold=-45dB");
    if let Some(ms) = max_ms {
        cmd.arg("-t").arg(format!("{:.3}", ms as f32 / 1000.0));
    }
    cmd.arg(&dst);
    let ok = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok && dst.is_file() {
        Some(dst.to_string_lossy().into_owned())
    } else {
        None
    }
}

/// Pre-trim the whole short-SFX set once (requires `paplay` + `ffmpeg`). Missing files or
/// missing tools simply leave the entry absent → the player falls back to spawning the
/// original mp3, so the engine degrades gracefully.
fn prepare_short_set(base: &Path) -> HashMap<&'static str, String> {
    let mut m = HashMap::new();
    if !has_bin("paplay") || !has_bin("ffmpeg") {
        return m;
    }
    for (rel, max_ms) in PREP_SET {
        if let Some(wav) = prepare_short(base, rel, *max_ms) {
            m.insert(*rel, wav);
        }
    }
    m
}

/// Fire a pre-trimmed WAV through low-latency `paplay`.
fn play_paplay(wav: &str, volume: u32) -> Option<Child> {
    Command::new("paplay")
        .arg(format!("--volume={}", volume))
        .arg(wav)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()
}

impl AudioEngine {
    /// Detect a player and pre-decode the keystroke clack. Ambient beds are deferred
    /// until the game crosses past the prelude (see [`AudioEngine::ensure_ambient`]),
    /// and the heartbeat is now driven beat-by-beat from the main loop, not looped.
    pub fn new() -> Self {
        let player = detect_player();
        let base = resolve_audio_base();
        let prep = prepare_short_set(&base);
        Self {
            player,
            ambient: Vec::new(),
            ambient_started: false,
            heartbeat: None,
            speech: None,
            clacks: Vec::new(),
            sfx: Vec::new(),
            prep,
            base,
            muted: false,
        }
    }

    /// Flip the master mute. Muting silences everything currently sounding and resets
    /// the ambient latch, so the rain/pendulum beds restart cleanly on unmute. Driven
    /// by the main-menu AUDIO toggle.
    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
        if muted {
            Self::reap(&mut self.heartbeat);
            Self::reap(&mut self.speech);
            for pool in [&mut self.ambient, &mut self.clacks, &mut self.sfx] {
                for c in pool.iter_mut() {
                    let _ = c.kill();
                    let _ = c.wait();
                }
                pool.clear();
            }
            self.ambient_started = false;
        }
    }

    /// Heavy mechanical latch — a committed menu selection, or a Space-bar heavy state
    /// toggle (Boole gate cycle, structural puzzle locks).
    pub fn menu_confirm(&mut self) {
        self.fire_short(SFX_MENU_CONFIRM, (PAPLAY_FULL * 9 / 10) as u32, 90);
    }

    /// Short subtle click on main-menu navigation (up/down/WASD).
    pub fn menu_nav(&mut self) {
        self.fire_short(SFX_MENU_NAV, PAPLAY_FULL * 7 / 10, 72);
    }

    /// Vintage terminal key-tap on a puzzle-grid cursor move (Arrows/WASD).
    pub fn grid_nav(&mut self) {
        self.fire_short(SFX_GRID_NAV, PAPLAY_FULL * 7 / 10, 72);
    }

    /// Rhythmic wooden shuttle slide — a Jacquard loom card/carriage operation.
    pub fn loom_shuttle(&mut self) {
        self.fire_short(SFX_LOOM, PAPLAY_FULL * 4 / 5, 84);
    }

    /// Rolling brass cogs — the Babbage engine cranking through a compilation run.
    pub fn babbage_gears(&mut self) {
        self.fire_short(SFX_GEARS, PAPLAY_FULL * 4 / 5, 84);
    }

    /// Sharp metallic jam — a Babbage out-of-phase calculation fault.
    pub fn gear_jam(&mut self) {
        self.fire_short(SFX_GEAR_JAM, PAPLAY_FULL, 92);
    }

    /// Spin up the perpetual ambient beds (rain, bump, background score, pendulum).
    /// Safe to call every frame — it only does work the first time (idempotent), so the
    /// caller can simply invoke it whenever the desk is visible (past the prelude).
    pub fn ensure_ambient(&mut self) {
        if self.ambient_started || self.player == Player::None || self.muted {
            return;
        }
        self.ambient_started = true;
        for (path, volume) in AMBIENT_TRACKS {
            if let Some(c) = self.spawn(path, true, *volume) {
                self.ambient.push(c);
            }
        }
    }

    /// Measure a cue's sample length in 62.5 fps frames via ffprobe, so the typewriter
    /// can be stretched to land its keystrokes under the spoken words. `None` if the
    /// cue has no sample or ffprobe is unavailable.
    pub fn duration_frames(&self, marker: &str) -> Option<u32> {
        let rel = marker_to_rel(marker)?;
        let path = self.base.join(rel);
        let out = Command::new("ffprobe")
            .args(["-v", "quiet", "-show_entries", "format=duration", "-of", "csv=p=0"])
            .arg(&path)
            .stderr(Stdio::null())
            .output()
            .ok()?;
        let secs: f32 = String::from_utf8_lossy(&out.stdout).trim().parse().ok()?;
        Some((secs * 62.5) as u32)
    }

    fn spawn(&self, rel: &str, looping: bool, volume: u8) -> Option<Child> {
        if self.player == Player::None || self.muted {
            return None;
        }
        let path = self.base.join(rel);
        let mut cmd = match self.player {
            Player::Mpv => {
                let mut c = Command::new("mpv");
                c.arg("--no-terminal")
                    .arg("--really-quiet")
                    .arg("--no-video")
                    .arg(format!("--volume={}", volume));
                if looping {
                    c.arg("--loop-file=inf");
                }
                c.arg(&path);
                c
            }
            Player::Ffplay => {
                let mut c = Command::new("ffplay");
                c.arg("-nodisp")
                    .arg("-autoexit")
                    .arg("-loglevel")
                    .arg("quiet")
                    .arg("-volume")
                    .arg(volume.to_string());
                if looping {
                    c.arg("-loop").arg("0");
                }
                c.arg(&path);
                c
            }
            Player::Mpg123 => {
                let mut c = Command::new("mpg123");
                c.arg("-q");
                // mpg123 scales 0..32768; map the 0..100 percent onto that range.
                let scale = (volume as u32 * 327).min(32768);
                c.arg("-f").arg(scale.to_string());
                if looping {
                    c.arg("--loop").arg("-1");
                }
                c.arg(&path);
                c
            }
            Player::None => return None,
        };
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()
    }

    fn reap(child: &mut Option<Child>) {
        if let Some(mut c) = child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }

    /// Hard-terminate any in-flight voice/speech take. The voice samples are detached
    /// child processes that would otherwise keep playing to their natural end even after
    /// the screen state moves on — so when a player skips a monologue or leaves a
    /// narrative state, the main loop calls this to cut the audio dead immediately.
    pub fn stop_voice_tracks(&mut self) {
        Self::reap(&mut self.speech);
    }

    /// Fire a one-shot cue (a Turing speech line, the door slide, a bootstep). Any
    /// speech still playing is cut so lines never pile up on top of each other.
    pub fn play_marker(&mut self, marker: &str) {
        if let Some(rel) = marker_to_rel(marker) {
            Self::reap(&mut self.speech);
            self.speech = self.spawn(rel, false, SPEECH_VOLUME);
        }
    }

    /// Fire one typewriter clack — called once per committed letter (dialogue stream
    /// or Lovelace editor line). Finished clacks are reaped first so processes never
    /// pile up under fast input.
    pub fn daktilo_strike(&mut self) {
        if self.muted {
            return;
        }
        self.clacks.retain_mut(|c| matches!(c.try_wait(), Ok(None)));
        // Drop the strike entirely if a smear of clacks is already sounding — keeps
        // fast key-repeat from stacking detached players into an overlapping mush.
        if self.clacks.len() >= MAX_CONCURRENT_CLACKS {
            return;
        }
        let child = if let Some(wav) = self.prep.get(SFX_KEY) {
            // Low-latency, silence-stripped PulseAudio/PipeWire path: the clack lands on
            // the exact tick the glyph is pushed, with no leading dead-air — so the
            // typewriter audio stays in lockstep with the appearing letters.
            play_paplay(wav, PAPLAY_FULL * 2 / 3)
        } else {
            self.spawn(SFX_KEY, false, KEY_FALLBACK_VOLUME)
        };
        if let Some(c) = child {
            self.clacks.push(c);
        }
    }

    /// A high-frequency typewriter click for rapid binary-path entry (Shannon). Pooled
    /// and capped exactly like the regular clack so fast 0/1 entry can't smear.
    pub fn daktilo_fast(&mut self) {
        if self.muted {
            return;
        }
        self.clacks.retain_mut(|c| matches!(c.try_wait(), Ok(None)));
        if self.clacks.len() >= MAX_CONCURRENT_CLACKS {
            return;
        }
        let child = if let Some(wav) = self.prep.get(SFX_KEY_FAST) {
            play_paplay(wav, PAPLAY_FULL * 2 / 3)
        } else {
            self.spawn(SFX_KEY_FAST, false, KEY_FALLBACK_VOLUME)
        };
        if let Some(c) = child {
            self.clacks.push(c);
        }
    }

    /// The clean, ultra-short mechanical snap of a Backspace correction (clamped to a
    /// ~200 ms window — no more 5-second bleed in Ada's editor).
    pub fn backspace_snap(&mut self) {
        self.fire_short(SFX_BACKSPACE, PAPLAY_FULL * 4 / 5, BACKSPACE_VOLUME);
    }

    /// Fire a short interface SFX: the low-latency pre-trimmed WAV via `paplay` when
    /// available, else the original mp3 through the detached player. Pooled and capped so
    /// rapid input can't pile detached processes into a smear.
    fn fire_short(&mut self, rel: &str, paplay_vol: u32, fallback_vol: u8) {
        if self.muted {
            return;
        }
        self.sfx.retain_mut(|c| matches!(c.try_wait(), Ok(None)));
        if self.sfx.len() >= MAX_CONCURRENT_SFX {
            return;
        }
        let child = if let Some(wav) = self.prep.get(rel) {
            play_paplay(wav, paplay_vol)
        } else {
            self.spawn(rel, false, fallback_vol)
        };
        if let Some(c) = child {
            self.sfx.push(c);
        }
    }

    /// The sharp electrical blowout — a mis-struck gate or a failed `R` verification.
    pub fn glitch(&mut self) {
        self.fire_oneshot(SFX_GLITCH, GLITCH_VOLUME);
    }

    /// The heavy mechanical click of the Turing read/write head sliding one cell.
    pub fn head_click(&mut self) {
        self.fire_oneshot(SFX_HEAD, BACKSPACE_VOLUME);
    }

    /// Fire one scheduled heartbeat. Called from the main loop at the live BPM cadence
    /// (skipped beats simply omit the call), so the audible pulse tracks the on-screen
    /// `VITAL: ♡ {bpm}` exactly, including arrhythmia spikes and dropped beats. The
    /// previous beat is reaped so rapid (spiked) beats never overlap into mush.
    pub fn heartbeat_beat(&mut self) {
        Self::reap(&mut self.heartbeat);
        self.heartbeat = self.spawn(SFX_HEARTBEAT, false, HEARTBEAT_VOLUME);
    }

    /// Spawn a pooled one-shot SFX, reaping any finished siblings first.
    fn fire_oneshot(&mut self, rel: &str, volume: u8) {
        self.sfx.retain_mut(|c| matches!(c.try_wait(), Ok(None)));
        if let Some(c) = self.spawn(rel, false, volume) {
            self.sfx.push(c);
        }
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        // Never leave detached players howling after the TUI exits.
        Self::reap(&mut self.heartbeat);
        Self::reap(&mut self.speech);
        for pool in [&mut self.ambient, &mut self.clacks, &mut self.sfx] {
            for c in pool.iter_mut() {
                let _ = c.kill();
                let _ = c.wait();
            }
        }
    }
}

#[cfg(test)]
mod intro_voice_tests {
    use super::*;
    use crate::engine::mind_log::VoiceCue;
    use crate::engine::state::Act;

    #[test]
    fn act_intro_lines_resolve_to_existing_speech_files() {
        let base = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/audio"));
        // Five acts have a two-line spoken intro; each cue must map to a real file.
        let voiced = [
            Act::Jacquard1804,
            Act::Babbage1837,
            Act::Lovelace1843,
            Act::Boole1854,
            Act::Shannon1937,
        ];
        for act in voiced {
            for n in 1..=2u8 {
                let marker = VoiceCue::ActIntroLine(act, n).marker();
                let rel = marker_to_rel(marker).expect("intro line should map to a file");
                assert!(base.join(rel).exists(), "missing speech asset: {}", rel);
            }
        }
        // Turing has no take → no mapping, so the intro stays gracefully silent.
        assert!(marker_to_rel(VoiceCue::ActIntroLine(Act::Turing1936_1950, 1).marker()).is_none());
    }
}
