use std::time::{Duration, Instant};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

mod engine;
use crate::engine::state::{Act, GlobalStateContext, ScreenState};
use crate::engine::audio::AudioEngine;
use crate::engine::menu::{self, MainMenu, MenuAction, MenuMode};
use crate::engine::cinematic;
use crate::engine::layout::EngineLayout;
use crate::engine::desk_render::render_desk;
use crate::engine::jacquard::{
    JacquardPuzzle, render_workspace as render_jacquard_workspace,
    handle_jacquard_input, tick_jacquard,
};
use crate::engine::babbage::{
    BabbagePuzzle, render_workspace as render_babbage_workspace,
    handle_babbage_input, tick_babbage,
};
use crate::engine::mind_log::{
    render_mind_log, render_prelude, DialogueEngine, Speaker, VoiceCue,
    PRELUDE_SCRIPT, AMBIENT_PROMPT,
};

// Decoupled structural act placeholders
use crate::engine::lovelace;
use crate::engine::boole;
use crate::engine::shannon;
use crate::engine::turing;

/// Frames the prelude holds between monologue lines (~0.6 s breath at the 16ms tick).
const LINE_GAP: u16 = 40;

/// The voice-over that plays the instant an act is taken up. Returns the speaker,
/// the line, and the deterministic sound cue. Never leaks numeric solutions.
fn act_intro(act: Act) -> (Speaker, &'static str, VoiceCue) {
    let text = match act {
        Act::Jacquard1804 =>
            "[TURING]: \"I thought to begin my day with this material, and I built this... a continuous roll. Go on then. Finish the damask.\"",
        Act::Babbage1837 =>
            "[TURING]: \"The Navy's tables are riddled with fatal errors. Banish multiplication \u{2014} build the squares from addition alone, and mind how the carry travels.\"",
        Act::Lovelace1843 =>
            "[TURING]: \"Ada saw it first. The engine can weave algebra as the loom weaves silk \u{2014} if the cards decide, and the loop closes upon itself.\"",
        Act::Boole1854 =>
            "[TURING]: \"Boole stripped thought to two values and a handful of operations. Find the minimum. Nothing spare survives.\"",
        Act::Shannon1937 =>
            "[TURING]: \"A switch is a proposition. Open or shut, true or false. Wire the logic and the relays will reason for us.\"",
        Act::Turing1936_1950 =>
            "[TURING]: \"And here, at the end of it: the machine that imitates any machine. Mine. If only they would let it be.\"",
    };
    (Speaker::Turing, text, VoiceCue::ActIntro(act))
}

/// Enter the cinematic act-intro: warm-decode the portrait, flip the screen state, and
/// begin streaming the figure's first biography line with its (async) voice cue. The
/// portrait + narration are drawn by the half-block image engine from the next frame.
fn enter_act_intro(
    state: &mut GlobalStateContext,
    dialogue: &mut DialogueEngine,
    audio: &AudioEngine,
    act_id: u8,
) {
    cinematic::preload_intro(act_id);
    state.screen_state = ScreenState::ActIntro { act_id, text_index: 0, timer: 0 };
    let act = cinematic::act_from_id(act_id);
    // First biography line, synced to its `<act>_intro1.mp3` voice take: the typewriter
    // is stretched to the audio length, and the cue fires the matching sample.
    let cue = VoiceCue::ActIntroLine(act, 1);
    let frames = audio.duration_frames(cue.marker());
    dialogue.play_timed(Speaker::System, cinematic::intro(act_id).lines[0], Some(cue), frames);
}

/// Enter the cinematic act-outro (the tragedy of the act just cleared). Mirrors
/// [`enter_act_intro`]; the outro scene has no dedicated voice cue, so it streams silent.
fn enter_act_outro(state: &mut GlobalStateContext, dialogue: &mut DialogueEngine, act_id: u8) {
    cinematic::preload_outro(act_id);
    state.screen_state = ScreenState::ActOutro { act_id, text_index: 0, timer: 0 };
    dialogue.play(Speaker::System, cinematic::outro(act_id).lines[0], None);
}

fn draw_top_bar(f: &mut ratatui::Frame, area: ratatui::layout::Rect, state: &GlobalStateContext) {
    let buf = f.buffer_mut();
    
    // Clear/fill the top bar area with deep amber-black bg
    let bg_color = ratatui::style::Color::Rgb(10, 8, 0);
    let text_color = ratatui::style::Color::Rgb(255, 176, 0);
    let dim_color = ratatui::style::Color::Rgb(74, 50, 5);
    let bright_color = ratatui::style::Color::Rgb(255, 213, 102);

    for x in area.x..(area.x + area.width) {
        if x >= buf.area().width { continue; }
        let cell = buf.get_mut(x, area.y);
        cell.set_char(' ');
        cell.bg = bg_color;
    }

    // Get current local time (using timezone offset +3:00 as per user local time metadata)
    let total_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let local_secs = total_secs + 3 * 3600;
    let hour = (local_secs / 3600) % 24;
    let minute = (local_secs / 60) % 60;
    let second = local_secs % 60;
    let time_str = format!("{:02}:{:02}:{:02}", hour, minute, second);

    // Map Act enum to roman numerals or titles
    let act_str = match state.current_act {
        Act::Jacquard1804 => "ACT I: JACQUARD 1804",
        Act::Babbage1837 => "ACT II: BABBAGE 1837",
        Act::Lovelace1843 => "ACT III: LOVELACE 1843",
        Act::Boole1854 => "ACT IV: BOOLE 1854",
        Act::Shannon1937 => "ACT V: SHANNON 1937",
        Act::Turing1936_1950 => "ACT VI: TURING 1936-1950",
    };

    // Construct the segments
    // Left: ◈ 17 JUN 2026 │ ACT I: JACQUARD 1804
    // Center: ▐ THE ENGINE ▌
    // Right: HH:MM:SS │ ♦ MEM: 4096K │ ◆ AUDIO: ON │ ◇
    
    let left_str = format!(" ◈ 17 JUN 2026 │ {}", act_str);
    let center_str = "▐ THE ENGINE ▌";
    let right_str = format!("{} │ ♦ MEM: 4096K │ ◆ AUDIO: ON │ ◇ ", time_str);

    // Write left segment
    let mut x = area.x;
    for ch in left_str.chars() {
        if x >= area.x + area.width { break; }
        let cell = buf.get_mut(x, area.y);
        cell.set_char(ch);
        if ch == '◈' || ch == '│' {
            cell.fg = dim_color;
        } else {
            cell.fg = text_color;
        }
        x += 1;
    }

    // Write center segment (centered in the area)
    let center_x = area.x + (area.width.saturating_sub(center_str.len() as u16) / 2);
    if center_x > x {
        let mut cx = center_x;
        for ch in center_str.chars() {
            if cx >= area.x + area.width { break; }
            let cell = buf.get_mut(cx, area.y);
            cell.set_char(ch);
            cell.fg = bright_color;
            cx += 1;
        }
    }

    // Write right segment (right-aligned)
    let right_len = right_str.chars().count() as u16;
    let right_x = (area.x + area.width).saturating_sub(right_len);
    let mut rx = right_x.max(x);
    for ch in right_str.chars() {
        if rx >= area.x + area.width { break; }
        let cell = buf.get_mut(rx, area.y);
        cell.set_char(ch);
        if ch == '│' || ch == '♦' || ch == '◆' || ch == '◇' {
            cell.fg = dim_color;
        } else {
            cell.fg = text_color;
        }
        rx += 1;
    }
}

/// Write a coloured segment starting at `x` on row `y`, clipped to `max_x`. Returns
/// the next free column so segments can be chained left-to-right.
fn put_seg(
    buf: &mut ratatui::buffer::Buffer,
    max_x: u16,
    mut x: u16,
    y: u16,
    s: &str,
    fg: ratatui::style::Color,
    bg: ratatui::style::Color,
) -> u16 {
    for ch in s.chars() {
        if x >= max_x { break; }
        let cell = buf.get_mut(x, y);
        cell.set_char(ch);
        cell.fg = fg;
        cell.bg = bg;
        x += 1;
    }
    x
}

/// The live heartbeat rate in BPM — the single source of truth shared by the on-screen
/// `VITAL: ♡ {bpm}` readout and the audio heartbeat metronome, so the audible pulse and
/// the displayed number can never drift apart. A return of `0.0` is a *skipped beat*
/// (the arrhythmia drop window): the telemetry shows `--` and the audio stays silent.
/// Rising chemical dosage (PPM) elevates the rate; arrhythmia injects spikes and drops.
fn current_bpm(state: &GlobalStateContext) -> f32 {
    let mut bpm = state.base_heartbeat_bpm as f32;
    bpm += ((state.frame_counter as f64 * 0.1).sin() * 2.0) as f32;
    // Deeper into the chemical dosage, the heart drives harder.
    bpm += state.stilboestrol_ppm * 0.40;
    if state.arrhythmia_multiplier > 0.0 {
        let cycle = state.frame_counter % 80;
        if cycle < 15 {
            bpm = 0.0; // skipped beat
        } else if cycle < 35 {
            bpm += 95.0 * state.arrhythmia_multiplier; // sudden spike
        } else {
            bpm += ((state.frame_counter as f32 * 0.4).sin() * 25.0) * state.arrhythmia_multiplier;
        }
    }
    bpm
}

/// The consolidated bottom telemetry bar (3 rows): a separator rule, a read-only
/// vitals/telemetry line (with the heartbeat metronome that used to clutter the
/// narrative box), and an interactive shell + control-legend line.
fn render_telemetry_bar(f: &mut ratatui::Frame, area: ratatui::layout::Rect, state: &GlobalStateContext) {
    use ratatui::style::Color;
    if area.height == 0 || area.width < 4 { return; }
    let buf = f.buffer_mut();

    let bg     = Color::Rgb(10, 8, 0);
    let bright = Color::Rgb(255, 176, 0);
    let label  = Color::Rgb(153, 104, 10);
    let dim    = Color::Rgb(74, 50, 5);
    let prompt = Color::Rgb(255, 213, 102);
    let heart  = Color::Rgb(255, 90, 40);
    let rule   = Color::Rgb(92, 68, 0);

    // Fill the whole bar.
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let cell = buf.get_mut(x, y);
            cell.set_char(' ');
            cell.fg = bg;
            cell.bg = bg;
        }
    }

    let max_x = area.x + area.width;

    // Row 0 — separator rule between the play columns and the bar.
    {
        let y = area.y;
        let mut x = area.x;
        while x < max_x {
            let cell = buf.get_mut(x, y);
            cell.set_char('─');
            cell.fg = rule;
            cell.bg = bg;
            x += 1;
        }
    }

    // Heartbeat — single shared source of truth, also drives the audio metronome.
    let bpm = current_bpm(&state);

    let (status, thread) = match state.current_act {
        Act::Jacquard1804 => ("Jacquard Loom Active", "0x01"),
        Act::Babbage1837 => ("Babbage Engine Active", "0x02"),
        Act::Lovelace1843 => ("Analytical Engine Active", "0x03"),
        Act::Boole1854 => ("Boolean Lattice Active", "0x04"),
        Act::Shannon1937 => ("Relay Network Active", "0x05"),
        Act::Turing1936_1950 => ("Universal Machine Active", "0x06"),
    };

    // Row 1 — read-only telemetry.
    if area.height >= 2 {
        let y = area.y + 1;
        let mut x = area.x + 2;
        x = put_seg(buf, max_x, x, y, "STATUS: ", label, bg);
        x = put_seg(buf, max_x, x, y, status, bright, bg);
        x = put_seg(buf, max_x, x, y, "   │   ", dim, bg);
        x = put_seg(buf, max_x, x, y, "VITAL: ", label, bg);
        x = put_seg(buf, max_x, x, y, "\u{2661} ", heart, bg);
        if bpm <= 0.5 {
            x = put_seg(buf, max_x, x, y, "-- BPM", heart, bg);
            x = put_seg(buf, max_x, x, y, " \u{00B7} SKIPPED", label, bg);
        } else {
            x = put_seg(buf, max_x, x, y, &format!("{:.0} BPM", bpm), heart, bg);
            x = put_seg(buf, max_x, x, y, " \u{00B7} METRONOME", label, bg);
        }
        x = put_seg(buf, max_x, x, y, "   │   ", dim, bg);
        x = put_seg(buf, max_x, x, y, "THREAD: ", label, bg);
        let _ = put_seg(buf, max_x, x, y, thread, bright, bg);
    }

    // Row 2 — interactive shell prompt + control legend.
    if area.height >= 3 {
        let y = area.y + 2;
        let mut x = area.x + 2;
        x = put_seg(buf, max_x, x, y, "residues@terminal:~/$ ", prompt, bg);
        let cursor = if (state.frame_counter / 24) % 2 == 0 { "\u{2588}" } else { " " };
        x = put_seg(buf, max_x, x, y, cursor, bright, bg);
        x = put_seg(buf, max_x, x, y, "   [W/A/S/D]", bright, bg);
        x = put_seg(buf, max_x, x, y, " Move Cursor  ", label, bg);
        x = put_seg(buf, max_x, x, y, "│", dim, bg);
        x = put_seg(buf, max_x, x, y, "  [Space]", bright, bg);
        x = put_seg(buf, max_x, x, y, " Punch  ", label, bg);
        x = put_seg(buf, max_x, x, y, "│", dim, bg);
        x = put_seg(buf, max_x, x, y, "  [Enter]", bright, bg);
        let _ = put_seg(buf, max_x, x, y, " Run", label, bg);
    }
}

/// Best-effort restore of the terminal to a sane cooked state. Safe to call more than
/// once; used by both the panic hook and the normal exit path.
fn restore_terminal() {
    let _ = crossterm::terminal::disable_raw_mode();
    let _ = crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show,
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── Raw-mode protection: if the game panics mid-frame, the terminal would
    //    otherwise be left in raw mode with the alternate screen up — corrupting the
    //    user's shell. Chain a hook that always cooks the terminal back first, then
    //    delegates to the default hook so the backtrace still prints. ──
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        default_hook(info);
    }));

    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = GlobalStateContext::new();

    // ── Phase 0: the menu owns the launch. We probe the checkpoint once and hand the
    //    snapshot to the menu, which labels option one (Continue vs. New Game) and, on
    //    Continue, hands the saved act/progress back to the global state. Nothing about
    //    the simulation — prelude, ambient beds, puzzles — runs until the player picks. ──
    let mut menu = MainMenu::new(engine::save::load().as_ref());

    let mut jacquard_puzzle = JacquardPuzzle::new();
    let mut babbage_puzzle = BabbagePuzzle::new();
    let mut lovelace_puzzle = lovelace::LovelacePuzzle::new();
    let mut boole_puzzle = boole::BoolePuzzle::new();
    let mut shannon_puzzle = shannon::ShannonPuzzle::new();
    let mut turing_core = turing::TuringCore::new();
    let mut dialogue = DialogueEngine::new();
    // Real audio: ambient beds spin up once past the prelude; the heartbeat is now
    // scheduled beat-by-beat from the BPM accumulator below; speech/clacks follow play.
    let mut audio = AudioEngine::new();

    // ── Narrative sequencing bookkeeping ──
    let mut prelude_idx: usize = 0;
    let mut prelude_hold: u16 = LINE_GAP;
    let mut last_act = state.current_act;
    // The furthest act ever reached this session — the checkpoint high-water mark. It is
    // the resume point and the cap on the act picker; replaying an *earlier* act never
    // regresses it, so the saved progress is preserved.
    let mut furthest_act = state.current_act;
    let mut prev_snapped = false;
    let mut prev_jammed = false;
    // Fractional-beat accumulator: each tick adds `bpm/3750` of a beat (62.5 fps × 60s);
    // when it crosses 1.0 a heartbeat fires. Skipped beats (bpm 0) simply stop adding.
    let mut heartbeat_accum: f32 = 0.0;

    // The simulation opens dormant on the main menu (Phase 0). The cinematic prelude
    // and the resumed-act ambient desk are both bootstrapped from the menu's selection
    // handler below, never here — so no monologue or ambient bed leaks before a choice.

    let tick_rate = Duration::from_millis(16);
    let mut last_tick = Instant::now();

    loop {
        let size = terminal.size()?;
        let on_last_line = prelude_idx + 1 >= PRELUDE_SCRIPT.len();
        let awaiting_enter = on_last_line && !dialogue.is_typing();

        terminal.draw(|f| {
            match state.screen_state {
                // ── Phase 0: the candlelit main menu, before any simulation runs. ──
                ScreenState::MainMenu => {
                    menu::render_main_menu(f, size, &menu, state.frame_counter);
                }
                // ── Phase 1: pitch-black cinematic typewriter; 3-panel split bypassed. ──
                ScreenState::NarrativePrelude => {
                    render_prelude(f, size, &dialogue, state.frame_counter, awaiting_enter);
                }
                // ── Phase 2 & 3: the 3-panel split. The workspace render self-guards
                //    on state.screen_state — under AmbientDesk it draws only the dormant
                //    blueprint grid + gauge and returns; the puzzle appears only once
                //    the dossier is taken up (ActivePuzzle). ──
                ScreenState::AmbientDesk | ScreenState::ActivePuzzle => {
                    let layout = EngineLayout::compute(size);
                    
                    // Draw top bar and the consolidated bottom telemetry bar
                    draw_top_bar(f, layout.top_bar_rect, &state);
                    render_telemetry_bar(f, layout.bottom_bar_rect, &state);

                    render_mind_log(f, layout.mind_rect, &state, &dialogue);
                    match state.current_act {
                        Act::Jacquard1804 =>
                            render_jacquard_workspace(f, layout.workspace_rect, &mut state, &jacquard_puzzle),
                        Act::Babbage1837 =>
                            render_babbage_workspace(f, layout.workspace_rect, &mut state, &babbage_puzzle),
                        Act::Lovelace1843 => lovelace::render_lovelace(f, layout.workspace_rect, &mut state, &lovelace_puzzle),
                        Act::Boole1854 => boole::render_boole(f, layout.workspace_rect, &mut state, &boole_puzzle),
                        Act::Shannon1937 => shannon::render_shannon(f, layout.workspace_rect, &mut state, &shannon_puzzle),
                        Act::Turing1936_1950 => turing::render_workspace(f, layout.workspace_rect, &mut state, &turing_core),
                    }
                    render_desk(f, layout.desk_rect, &state);
                }
                // ── Cinematic: portrait + biography (intro) or downfall (outro),
                //    drawn full-frame by the half-block pixel-art engine. ──
                ScreenState::ActIntro { act_id, .. } => {
                    let awaiting = !dialogue.is_typing();
                    cinematic::render_intro(f, size, act_id, &dialogue, state.frame_counter, awaiting);
                }
                ScreenState::ActOutro { act_id, .. } => {
                    let awaiting = !dialogue.is_typing();
                    cinematic::render_outro(f, size, act_id, &dialogue, state.frame_counter, awaiting);
                }
            }
        })?;

        // ── INPUT ──
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press && key.kind != KeyEventKind::Repeat {
                    // Ignore key-release / non-press events.
                } else if key.code == KeyCode::Esc {
                    // In the act picker, Esc steps back to the main menu instead of
                    // quitting; everywhere else it exits the game.
                    if state.screen_state == ScreenState::MainMenu && menu.mode == MenuMode::ActSelect {
                        menu.mode = MenuMode::Main;
                    } else {
                        restore_terminal();
                        return Ok(());
                    }
                } else {
                    match state.screen_state {
                        // ── Phase 0: main-menu navigation. Number keys jump-and-select;
                        //    arrows/W-S move the cursor; Enter/Space commit. Every commit
                        //    fires the deliberate confirmation click. ──
                        ScreenState::MainMenu => match menu.mode {
                            // ── Top-level choices. Number keys jump-and-select;
                            //    arrows/W-S move the cursor; Enter/Space commit. ──
                            MenuMode::Main => {
                                let n = menu.option_count();
                                let mut activate = false;
                                match key.code {
                                    KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => { menu.up(); audio.menu_nav(); }
                                    KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => { menu.down(); audio.menu_nav(); }
                                    // Bound-checked against the active list length so an
                                    // out-of-range digit is simply ignored — never a panic.
                                    KeyCode::Char(c @ '1'..='9') => {
                                        let idx = c as usize - '1' as usize;
                                        if idx < n {
                                            menu.selected = idx;
                                            activate = true;
                                        }
                                    }
                                    KeyCode::Enter | KeyCode::Char(' ') => activate = true,
                                    _ => {}
                                }
                                if activate {
                                    audio.menu_confirm();
                                    match menu.selected_action() {
                                        // Open the act picker (cursor on the latest act).
                                        MenuAction::SelectAct => menu.open_act_select(),
                                        // Fresh start (incl. "Erase Memory Core"): purge the
                                        // save, reset progress, boot the cinematic prelude.
                                        MenuAction::NewGame => {
                                            engine::save::clear();
                                            state.current_act = Act::Jacquard1804;
                                            state.acts_completed.clear();
                                            last_act = state.current_act;
                                            furthest_act = state.current_act;
                                            state.screen_state = ScreenState::NarrativePrelude;
                                            prelude_idx = 0;
                                            prelude_hold = LINE_GAP;
                                            let cue = VoiceCue::PreludeLine(1);
                                            let frames = audio.duration_frames(cue.marker());
                                            dialogue.play_script(Speaker::Turing, PRELUDE_SCRIPT[0], Some(cue), frames);
                                        }
                                        // Toggle the audio master mute (stay on the menu).
                                        MenuAction::AudioToggle => {
                                            menu.audio_on = !menu.audio_on;
                                            audio.set_muted(!menu.audio_on);
                                        }
                                        // Quit to the shell.
                                        MenuAction::Exit => {
                                            restore_terminal();
                                            return Ok(());
                                        }
                                    }
                                }
                            }
                            // ── Act picker: choose any unlocked act to re-enter. ──
                            MenuMode::ActSelect => {
                                let n = menu.unlocked_count();
                                let mut play = false;
                                match key.code {
                                    KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => { menu.act_up(); audio.menu_nav(); }
                                    KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => { menu.act_down(); audio.menu_nav(); }
                                    KeyCode::Char(c @ '1'..='9') => {
                                        let idx = c as usize - '1' as usize;
                                        if idx < n {
                                            menu.act_cursor = idx;
                                            play = true;
                                        }
                                    }
                                    KeyCode::Enter | KeyCode::Char(' ') => play = true,
                                    _ => {}
                                }
                                if play {
                                    audio.menu_confirm();
                                    // Re-enter the chosen act's desk. The high-water mark is
                                    // preserved (= the saved furthest act), so replaying an
                                    // earlier act never regresses the saved progress.
                                    let chosen = menu.selected_act();
                                    furthest_act = menu.resume_act;
                                    state.current_act = chosen;
                                    state.acts_completed = menu.acts_completed.clone();
                                    last_act = chosen;
                                    state.screen_state = ScreenState::AmbientDesk;
                                    state.desk_reveal = 0.0;
                                    state.lookup_active = false;
                                    prelude_idx = PRELUDE_SCRIPT.len().saturating_sub(1);
                                    dialogue.play(Speaker::System, AMBIENT_PROMPT, None);
                                }
                            }
                        },
                        // ── Phase 1: gated monologue; the workspace is sealed off. ──
                        ScreenState::NarrativePrelude => {
                            if dialogue.is_typing() {
                                // First press: autocomplete the line AND cut its voice
                                // take, so the skipped audio never plays on past the text.
                                audio.stop_voice_tracks();
                                dialogue.skip(); // reveal the rest of this line at once
                            } else if !on_last_line {
                                prelude_hold = 0; // a key skips the breath to the next line
                            } else if key.code == KeyCode::Enter {
                                // Prelude complete → Act I's figure intro (Jacquard's
                                // wide-cropped portrait), which then opens the ambient desk.
                                audio.stop_voice_tracks();
                                enter_act_intro(&mut state, &mut dialogue, &audio, 1);
                            }
                        }
                        // ── Phase 2: study in silence. ANY key (Esc already handled
                        //    above as the only system shortcut) takes up the dossier:
                        //    slide the door, flip to ActivePuzzle, and let the NEXT
                        //    frame be the first to render the puzzle. No puzzle input is
                        //    routed or cached while ambient. ──
                        ScreenState::AmbientDesk => {
                            if dialogue.is_typing() {
                                // The ambient prompt is still streaming — the first key
                                // flushes it to full length (never leaving a clipped
                                // "[SYSTEM]: The desk is yo" frozen in the log); only the
                                // next key takes up the dossier.
                                dialogue.skip();
                            } else {
                                audio.play_marker(VoiceCue::DoorSlide.marker()); // safe-door slide
                                state.screen_state = ScreenState::ActivePuzzle;
                                state.lookup_active = false;
                                let (sp, txt, cue) = act_intro(state.current_act);
                                dialogue.play(sp, txt, Some(cue));
                            }
                        }
                        // ── Phase 3: full puzzle routing. ──
                        ScreenState::ActivePuzzle => {
                            if dialogue.is_typing() {
                                dialogue.skip();
                            } else if key.code == KeyCode::Tab {
                                state.lookup_active = !state.lookup_active;
                            } else if state.lookup_active {
                                match key.code {
                                    KeyCode::Char('a') | KeyCode::Char('A')
                                    | KeyCode::Char('e') | KeyCode::Char('E')
                                    | KeyCode::Enter | KeyCode::Char(' ') => {
                                        if state.apple_bites_taken < 4 {
                                            state.apple_bites_taken += 1;
                                            state.melt_candle(10);
                                            state.monologue_timer = 180;
                                        }
                                    }
                                    _ => {}
                                }
                            } else {
                                match state.current_act {
                                    Act::Jacquard1804 =>
                                        handle_jacquard_input(key, &mut jacquard_puzzle, &mut state, &mut audio),
                                    Act::Babbage1837 =>
                                        handle_babbage_input(key, &mut babbage_puzzle, &mut state, &mut dialogue, &mut audio),
                                    Act::Lovelace1843 => lovelace::handle_input(key, &mut lovelace_puzzle, &mut state, &mut dialogue, &mut audio),
                                    Act::Boole1854 => boole::handle_input(key, &mut boole_puzzle, &mut state, &mut dialogue, &mut audio),
                                    Act::Shannon1937 => shannon::handle_input(key, &mut shannon_puzzle, &mut state, &mut dialogue, &mut audio),
                                    Act::Turing1936_1950 => turing::handle_input(key, &mut turing_core, &mut state, &mut dialogue, &mut audio),
                                }
                            }
                        }
                        // ── Cinematic intro: a key skips the typewriter; ENTER/Space
                        //    pages through the biography, then crosses into the act. ──
                        ScreenState::ActIntro { act_id, text_index, timer } => {
                            if dialogue.is_typing() {
                                // First press: flush the line in full and cut its voice.
                                audio.stop_voice_tracks();
                                dialogue.skip();
                            } else if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                                let lines = cinematic::intro(act_id).lines;
                                if text_index + 1 < lines.len() {
                                    // Next press: advance to the next biography paragraph,
                                    // synced to its own voice take (<act>_intro{n}.mp3).
                                    audio.stop_voice_tracks();
                                    let ni = text_index + 1;
                                    state.screen_state = ScreenState::ActIntro { act_id, text_index: ni, timer };
                                    let cue = VoiceCue::ActIntroLine(cinematic::act_from_id(act_id), (ni + 1) as u8);
                                    let frames = audio.duration_frames(cue.marker());
                                    dialogue.play_timed(Speaker::System, lines[ni], Some(cue), frames);
                                } else {
                                    // Intro complete → onto that act's ambient desk.
                                    audio.stop_voice_tracks();
                                    state.current_act = cinematic::act_from_id(act_id);
                                    state.screen_state = ScreenState::AmbientDesk;
                                    state.desk_reveal = 0.0;
                                    state.lookup_active = false;
                                    last_act = state.current_act;
                                    prelude_idx = PRELUDE_SCRIPT.len().saturating_sub(1);
                                    dialogue.play(Speaker::System, AMBIENT_PROMPT, None);
                                }
                            }
                        }
                        // ── Cinematic outro: page through the downfall, then hand off to
                        //    the next act's intro (current_act has already advanced). ──
                        ScreenState::ActOutro { act_id, text_index, timer } => {
                            if dialogue.is_typing() {
                                audio.stop_voice_tracks();
                                dialogue.skip();
                            } else if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                                let lines = cinematic::outro(act_id).lines;
                                if text_index + 1 < lines.len() {
                                    audio.stop_voice_tracks();
                                    let ni = text_index + 1;
                                    state.screen_state = ScreenState::ActOutro { act_id, text_index: ni, timer };
                                    dialogue.play(Speaker::System, lines[ni], None);
                                } else {
                                    audio.stop_voice_tracks();
                                    let next_id = cinematic::id_from_act(state.current_act);
                                    enter_act_intro(&mut state, &mut dialogue, &audio, next_id);
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── TICK ──
        if last_tick.elapsed() >= tick_rate {
            state.tick();
            dialogue.tick();

            // The ambient beds (rain, low bump, background score, pendulum) run
            // continuously from the very first frame — the main menu included — and
            // loop for the whole session. Idempotent + mute-guarded, so it is safe to
            // call every tick regardless of screen state.
            audio.ensure_ambient();

            // Heartbeat metronome — fire beats at the live BPM. A `0.0` BPM is the
            // arrhythmia skip window, where no beat fires (the silence is the skip).
            let bpm = current_bpm(&state);
            // The pulse runs only inside the lived simulation — the menu and the
            // cinematic intros/outros are still, calm antechambers.
            let pulse_active = matches!(
                state.screen_state,
                ScreenState::NarrativePrelude | ScreenState::AmbientDesk | ScreenState::ActivePuzzle
            );
            if pulse_active && bpm > 0.5 {
                heartbeat_accum += bpm / 3750.0; // beats per 16ms tick (62.5 fps × 60s)
                if heartbeat_accum >= 1.0 {
                    heartbeat_accum -= 1.0;
                    audio.heartbeat_beat();
                }
            }

            if let Some(cue) = dialogue.take_cue() {
                audio.play_marker(cue.marker());
            }
            // One daktilo clack per committed letter — in lockstep with the text.
            if dialogue.took_keystroke() {
                audio.daktilo_strike();
            }

            match state.screen_state {
                // Phase 0: the menu only breathes (frame counter drives the candle
                // flicker); nothing in the simulation advances.
                ScreenState::MainMenu => {}
                // Phase 1: auto-advance the monologue line by line, holding a beat
                // between each. The final line waits for an explicit ENTER.
                ScreenState::NarrativePrelude => {
                    if dialogue.is_typing() {
                        prelude_hold = LINE_GAP;
                    } else if prelude_idx + 1 < PRELUDE_SCRIPT.len() {
                        if prelude_hold > 0 {
                            prelude_hold -= 1;
                        } else {
                            prelude_idx += 1;
                            let cue = VoiceCue::PreludeLine((prelude_idx + 1) as u8);
                            let frames = audio.duration_frames(cue.marker());
                            dialogue.play_script(
                                Speaker::Turing,
                                PRELUDE_SCRIPT[prelude_idx],
                                Some(cue),
                                frames,
                            );
                            prelude_hold = LINE_GAP;
                        }
                    }
                }
                // Cinematic: the world is frozen; only the per-state timer advances
                // (the typewriter line is driven by `dialogue.tick()` above).
                ScreenState::ActIntro { act_id, text_index, timer } => {
                    state.screen_state = ScreenState::ActIntro { act_id, text_index, timer: timer.wrapping_add(1) };
                }
                ScreenState::ActOutro { act_id, text_index, timer } => {
                    state.screen_state = ScreenState::ActOutro { act_id, text_index, timer: timer.wrapping_add(1) };
                }
                // Phase 2: nothing advances; the world simply breathes and reveals.
                ScreenState::AmbientDesk => {}
                // Phase 3: the live simulation runs.
                ScreenState::ActivePuzzle => {
                    match state.current_act {
                        Act::Jacquard1804 => tick_jacquard(&mut jacquard_puzzle, &mut state, &mut dialogue),
                        Act::Babbage1837 => tick_babbage(&mut babbage_puzzle, &mut state, &mut dialogue),
                        Act::Boole1854 => boole::tick_boole(&mut boole_puzzle, &mut state, &mut dialogue),
                        Act::Turing1936_1950 => turing::tick_turing(&mut turing_core, &mut state, &mut dialogue, &mut audio),
                        _ => {}
                    }

                    // Act transition → intercept with the cinematic bridge: the tragic
                    // outro of the act just cleared, which then hands off to the next
                    // act's intro before the player resumes on its ambient desk.
                    if state.current_act != last_act {
                        let completed = last_act;
                        last_act = state.current_act;
                        state.lookup_active = false;
                        // Advance the high-water mark, but never below it — replaying an
                        // earlier act and finishing it must not regress the saved furthest.
                        if cinematic::id_from_act(state.current_act) > cinematic::id_from_act(furthest_act) {
                            furthest_act = state.current_act;
                        }
                        // Checkpoint: persist the furthest act reached, not the act in play.
                        engine::save::save(&engine::save::SaveState {
                            current_act: furthest_act,
                            acts_completed: state.acts_completed.clone(),
                        });
                        enter_act_outro(&mut state, &mut dialogue, cinematic::id_from_act(completed));
                    }

                    // Jacquard failure voice hooks (rising-edge detection).
                    if state.current_act == Act::Jacquard1804 {
                        if jacquard_puzzle.snapped && !prev_snapped {
                            dialogue.play(
                                Speaker::Turing,
                                "[TURING]: \"Of course it would end like this. Nonsense. The paper roll is structurally too weak for the loom's carriage tension. I must find another medium... something rigid. Falcon blocks.\"",
                                Some(VoiceCue::JacquardTear),
                            );
                        }
                        if jacquard_puzzle.jammed && !prev_jammed {
                            dialogue.play(
                                Speaker::Turing,
                                "[TURING]: \"Rigid, yes \u{2014} but a chain of blocks is monstrous; it jams under its own mass. The medium must be modular. Separate cards, fed in sequence. Jacquard's cards.\"",
                                Some(VoiceCue::JacquardJam),
                            );
                        }
                    }
                    prev_snapped = jacquard_puzzle.snapped;
                    prev_jammed = jacquard_puzzle.jammed;
                }
            }

            last_tick = Instant::now();
        }
    }
}

#[cfg(test)]
mod render_tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};
    use crate::engine::mind_log::DialogueEngine;

    // Render the post-prelude 3-column layout once and assert no panic (bounds safety).
    fn smoke(w: u16, h: u16) {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        let state = GlobalStateContext::new();
        let dlg = DialogueEngine::new();
        term.draw(|f| {
            let size = f.size();
            let layout = EngineLayout::compute(size);
            draw_top_bar(f, layout.top_bar_rect, &state);
            render_telemetry_bar(f, layout.bottom_bar_rect, &state);
            render_mind_log(f, layout.mind_rect, &state, &dlg);
            render_desk(f, layout.desk_rect, &state);
        }).unwrap();
    }

    #[test]
    fn renders_at_various_sizes() {
        smoke(120, 40); // wide
        smoke(80, 24);  // classic
        smoke(40, 14);  // tight
        smoke(20, 8);   // degenerate (panels self-guard)
    }
}
