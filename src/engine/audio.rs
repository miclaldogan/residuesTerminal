use std::process::{Child, Command, Stdio};

// ─────────────────────────────────────────────────────────────────────────────
// Crate-free audio: we drive a detached command-line player (mpv / ffplay /
// mpg123). The TUI never touches the audio device directly; it just spawns and
// reaps short-lived child processes, keeping the alternate screen uncorrupted by
// redirecting every child's stdio to /dev/null.
// ─────────────────────────────────────────────────────────────────────────────

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
        "SFX_DOOR_SLIDE" => "sfx/A_single,_isolated_s_#1-1781700210070.mp3",
        _ => return None,
    })
}

/// The runtime audio mixer. Owns three voices: a looping heartbeat, a looping
/// typewriter clatter that runs only while text is streaming, and a one-shot speech
/// channel that replaces itself on each new line.
pub struct AudioEngine {
    player: Player,
    heartbeat: Option<Child>,
    speech: Option<Child>,
    clacks: Vec<Child>, // short-lived per-letter typewriter strikes, reaped lazily
    clack_wav: Option<String>, // pre-decoded WAV for low-latency paplay clacks
}

/// Pre-decode the typewriter clack to a WAV and confirm `paplay` exists. A WAV +
/// paplay starts in ~milliseconds, so firing one per keystroke can't choke the
/// heavier speech `mpv` the way spawning an mpv per letter would.
fn prepare_clack() -> Option<String> {
    let has = |bin: &str| {
        Command::new(bin)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
    };
    if !has("paplay") || !has("ffmpeg") {
        return None;
    }
    let src = format!("{}/audio/sfx/daktiloOne.mp3", env!("CARGO_MANIFEST_DIR"));
    let dst = std::env::temp_dir().join("residues_daktilo.wav");
    let ok = Command::new("ffmpeg")
        .args(["-y", "-loglevel", "quiet", "-i"])
        .arg(&src)
        .arg(&dst)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        Some(dst.to_string_lossy().into_owned())
    } else {
        None
    }
}

impl AudioEngine {
    /// Detect a player and start the perpetual heartbeat loop immediately.
    pub fn new() -> Self {
        let player = detect_player();
        let mut engine = Self {
            player,
            heartbeat: None,
            speech: None,
            clacks: Vec::new(),
            clack_wav: prepare_clack(),
        };
        // The suffocating bump-loop runs for the whole session.
        engine.heartbeat = engine.spawn("sfx/bump.mp3", true, 55);
        engine
    }

    /// Measure a cue's sample length in 62.5 fps frames via ffprobe, so the typewriter
    /// can be stretched to land its keystrokes under the spoken words. `None` if the
    /// cue has no sample or ffprobe is unavailable.
    pub fn duration_frames(&self, marker: &str) -> Option<u32> {
        let rel = marker_to_rel(marker)?;
        let path = format!("{}/audio/{}", env!("CARGO_MANIFEST_DIR"), rel);
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
        if self.player == Player::None {
            return None;
        }
        let path = format!("{}/audio/{}", env!("CARGO_MANIFEST_DIR"), rel);
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
                c.arg("-nodisp").arg("-autoexit").arg("-loglevel").arg("quiet");
                if looping {
                    c.arg("-loop").arg("0");
                }
                c.arg(&path);
                c
            }
            Player::Mpg123 => {
                let mut c = Command::new("mpg123");
                c.arg("-q");
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

    /// Fire a one-shot cue (a Turing speech line, the door slide). Any speech still
    /// playing is cut so lines never pile up on top of each other.
    pub fn play_marker(&mut self, marker: &str) {
        if let Some(rel) = marker_to_rel(marker) {
            Self::reap(&mut self.speech);
            self.speech = self.spawn(rel, false, 100);
        }
    }

    /// Fire one typewriter clack — called once per committed letter, so "was" makes
    /// three strikes. Finished clacks are reaped first so processes never pile up.
    pub fn daktilo_strike(&mut self) {
        // Reap finished strikes first so processes never pile up.
        self.clacks.retain_mut(|c| matches!(c.try_wait(), Ok(None)));
        let child = if let Some(wav) = &self.clack_wav {
            // Low-latency PulseAudio/PipeWire path — won't starve the speech stream.
            Command::new("paplay")
                .arg("--volume=42000")
                .arg(wav)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .ok()
        } else {
            self.spawn("sfx/daktiloOne.mp3", false, 80)
        };
        if let Some(c) = child {
            self.clacks.push(c);
        }
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        // Never leave detached players howling after the TUI exits.
        Self::reap(&mut self.heartbeat);
        Self::reap(&mut self.speech);
        for c in self.clacks.iter_mut() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}
