# RESIDUES — An Engine of Residual Minds

A terminal-native narrative puzzle game about the birth of computation, written in Rust.

> *"The most terrifying part is learning how to forget."*

RESIDUES runs entirely inside your terminal — no graphics window, no mouse — rendering a truecolor (24-bit) amber-phosphor aesthetic. You are inside the failing mind of Alan Turing on the night of his death (Wilmslow, 8 June 1954). As his cognition collapses, he replays the entire lineage of computing as six interactive puzzles, each one rebuilding the actual idea its pioneer gave the world.

The design principle is **"Bright Puzzle, Dark Undertow":** every act is a clean, genuinely deep technical challenge, while underneath runs the human tragedy of the mind that invented it.

---

## The Six Acts

Each act is a real computational toy — not a reskinned button-press. You don't read *about* the machine; you operate one.

| Act | Mind | Year | What you actually do |
|-----|------|------|----------------------|
| **I** | Joseph-Marie Jacquard | 1804 | Punch an 8×8 bit matrix into modular cards. A continuous roll tears and a block-chain jams — only a looping card deck survives. Compress a 4-row damask into a 2-card loop. |
| **II** | Charles Babbage | 1837 | Derive the method-of-differences baseline for n², then survive the carry crisis by installing delay buffers in a staggered cascade so the carry propagates as a wave. |
| **III** | Ada Lovelace | 1843 | Write real assembly (`LOAD/STORE/ADD/SUB/IF_Z/JMP/HALT`) on a 3-register machine. Phase 1 caps you at 3 linear instructions; overrun it to unlock loops, then compute a checksum with a loop that devours its own counter. |
| **IV** | George Boole | 1854 | Steer a grid of logic gates (AND/OR/XOR/NOT) to a target checksum. As toxicity rises the *displayed* output starts lying — but the truth always holds beneath. |
| **V** | Claude Shannon | 1937 | Assign prefix-free binary codes within a fixed channel capacity (`C = W·log₂(1+SNR)`), then add a Hamming(7,4) parity guard against bit-flip noise. Real information theory. |
| **VI** | Alan Turing | 1936 | Operate a real Turing machine — 4-state control, a head over a tape — stabilizing the five prior acts' corrupted "residues" until the machine reaches HALT, interleaved with the timed Imitation Game interrogation. |

---

## Atmosphere & Presentation

The screen is a three-panel cockpit: a left **narrative log** that types Turing's collapsing mind-stream a character at a time, the central **workspace** where the act's machine lives, and a right **candlelit desk** — an overlapping dossier stack lit by a single half-block candle that acts as a true light source, with a radial light-degradation pass bathing the whole panel. The candle physically **melts down** as the night wears on (full in Act I, a guttering ~5% ember by Act VI), and the dossier card carries the act's atmospheric scene, dimmed so it reads as lit by the flame rather than self-lit. As the chemical dosage climbs, the text and imagery decay into structural static.

Each act opens with a **cinematic portrait + biography** of its mind and closes with a **tragic outro**, both rendered by a real PNG → truecolor half-block engine and synced to spoken voice-overs. Underneath it all runs a live **heartbeat metronome** whose tempo tracks Turing's vitals, an ambient rain-and-score bed, and a layer of binaural "Memory Echo" whispers that pan between your ears on headphones. The whole thing resolves in a hardcoded closing sequence — a falling-binary dissolve and a quiet final reckoning.

---

## Running the Game

### Quick start (from source)

Requires the [Rust toolchain](https://rustup.rs/).

```bash
git clone https://github.com/miclaldogan/residuesTerminal.git
cd residuesTerminal
cargo run --release
```

That's it — no build flags, no environment variables. Run it from the repo root so the asset folders (`audio/`, `images/`) are found.

### Zero-install

```bash
curl -sSL https://raw.githubusercontent.com/miclaldogan/residuesTerminal/main/run.sh | bash
```

Downloads a prebuilt tarball into a temp dir, runs it, and self-destructs.

### Controls

- **Menu:** `↑/↓` or number keys to select, `Enter` to confirm, `Esc` to go back
- **Puzzles:** `W/A/S/D` or arrows to move the cursor, `Space` to punch / cycle a gate, `Enter` to run
- The narrative log on the left types itself out; press any key to fast-forward it

---

## Audio (recommended)

The audio layer — binaural ghost whispers, voice-overs, a live heartbeat metronome, and an ambient score — is a core part of the experience and **best with headphones in a truecolor terminal.**

It needs **one** external CLI player. The engine probes in order:

1. **`mpv`** — preferred, best result
2. `ffplay` (from FFmpeg) — alternative
3. `mpg123` — alternative

If none is found, the game runs **completely silent but fully playable** — no crash, no error. (`paplay` + `ffmpeg` are an optional Linux-only low-latency path for keystroke SFX; absent them it falls back to the main player.)

```bash
# Arch
sudo pacman -S mpv
# Debian/Ubuntu
sudo apt install mpv
# macOS
brew install mpv
```

---

## Platform Support

| Platform | Status |
|----------|--------|
| **Linux** | Primary — built and tested here |
| **macOS** | Expected to work (`run.sh` supports Darwin; `mpv`/`ffplay` available). The PulseAudio low-latency SFX path is absent, so those fall back to the main player — degraded, not broken. |
| **Windows** | Untested / unsupported for now |

68/68 unit tests passing (`cargo test`), covering the assembly interpreter, Shannon capacity/entropy, Boole gate algebra, the Turing machine's transition completeness and halt conditions, the interrogation prompt layout, the desk dossier word-wrap, the audio filter chain, and layout smoke tests down to 20×8 terminals.

---

## Built With

- **Rust** — ~11,700 lines across 17 modules
- **[ratatui](https://ratatui.rs/)** + **[crossterm](https://github.com/crossterm-rs/crossterm)** — the TUI
- A crate-free, lock-free audio engine that spawns detached OS players
- A real PNG/JPEG → truecolor half-block pixel-art renderer for the cinematic portraits
- Voice and whisper audio generated with **[ElevenLabs](https://elevenlabs.io/)** (Starter plan, commercial license)

Built solo during the [June Solstice Game Jam](https://dev.to/challenges/june-game-jam-2026-06-03), June 2026.

---

## License

Code is released under the **MIT License** (see `LICENSE`).

The bundled audio (`audio/`) and image (`images/`) assets are **not** covered by MIT. The voice and music tracks were generated by the author using ElevenLabs on a paid plan that grants a commercial license; all rights to those generated outputs are held by the author. Please do not redistribute the assets separately from this project.

---

*Developed by Iclal Doğan.*
