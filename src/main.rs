use std::time::{Duration, Instant};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

mod engine;
use crate::engine::state::{Act, GlobalStateContext, ScreenState};
use crate::engine::audio::AudioEngine;
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = GlobalStateContext::new();
    let mut jacquard_puzzle = JacquardPuzzle::new();
    let mut babbage_puzzle = BabbagePuzzle::new();
    let mut dialogue = DialogueEngine::new();
    // Real audio: looping heartbeat starts immediately; speech/daktilo follow play.
    let mut audio = AudioEngine::new();

    // ── Narrative sequencing bookkeeping ──
    let mut prelude_idx: usize = 0;
    let mut prelude_hold: u16 = LINE_GAP;
    let mut last_act = state.current_act;
    let mut pending_intro: Option<Act> = None;
    let mut prev_snapped = false;
    let mut prev_jammed = false;

    // Phase 1 begins: the first monologue line streams full-screen on black, its
    // keystrokes stretched to land under the matching turingSpeech1 take.
    {
        let cue = VoiceCue::PreludeLine(1);
        let frames = audio.duration_frames(cue.marker());
        dialogue.play_script(Speaker::Turing, PRELUDE_SCRIPT[0], Some(cue), frames);
    }

    let tick_rate = Duration::from_millis(16);
    let mut last_tick = Instant::now();

    loop {
        let size = terminal.size()?;
        let on_last_line = prelude_idx + 1 >= PRELUDE_SCRIPT.len();
        let awaiting_enter = on_last_line && !dialogue.is_typing();

        terminal.draw(|f| {
            match state.screen_state {
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
                    render_mind_log(f, layout.mind_rect, &state, &dialogue);
                    match state.current_act {
                        Act::Jacquard1804 =>
                            render_jacquard_workspace(f, layout.workspace_rect, &mut state, &jacquard_puzzle),
                        Act::Babbage1837 =>
                            render_babbage_workspace(f, layout.workspace_rect, &mut state, &babbage_puzzle),
                        Act::Lovelace1843 => lovelace::render_workspace(f, layout.workspace_rect, &mut state),
                        Act::Boole1854 => boole::render_workspace(f, layout.workspace_rect, &mut state),
                        Act::Shannon1937 => shannon::render_workspace(f, layout.workspace_rect, &mut state),
                        Act::Turing1936_1950 => turing::render_workspace(f, layout.workspace_rect, &mut state),
                    }
                    render_desk(f, layout.desk_rect, &state);
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
                    crossterm::terminal::disable_raw_mode()?;
                    crossterm::execute!(terminal.backend_mut(), crossterm::terminal::LeaveAlternateScreen)?;
                    return Ok(());
                } else {
                    match state.screen_state {
                        // ── Phase 1: gated monologue; the workspace is sealed off. ──
                        ScreenState::NarrativePrelude => {
                            if dialogue.is_typing() {
                                dialogue.skip(); // reveal the rest of this line at once
                            } else if !on_last_line {
                                prelude_hold = 0; // a key skips the breath to the next line
                            } else if key.code == KeyCode::Enter {
                                // Cross the threshold into the ambient desk (Phase 2).
                                state.screen_state = ScreenState::AmbientDesk;
                                state.desk_reveal = 0.0;
                                state.lookup_active = false;
                                dialogue.play(Speaker::System, AMBIENT_PROMPT, None);
                            }
                        }
                        // ── Phase 2: study in silence. ANY key (Esc already handled
                        //    above as the only system shortcut) takes up the dossier:
                        //    slide the door, flip to ActivePuzzle, and let the NEXT
                        //    frame be the first to render the puzzle. No puzzle input is
                        //    routed or cached while ambient. ──
                        ScreenState::AmbientDesk => {
                            audio.play_marker(VoiceCue::DoorSlide.marker()); // safe-door slide
                            state.screen_state = ScreenState::ActivePuzzle;
                            state.lookup_active = false;
                            let (sp, txt, cue) = act_intro(state.current_act);
                            dialogue.play(sp, txt, Some(cue));
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
                                        handle_jacquard_input(key, &mut jacquard_puzzle, &mut state),
                                    Act::Babbage1837 =>
                                        handle_babbage_input(key, &mut babbage_puzzle, &mut state, &mut dialogue),
                                    Act::Lovelace1843 => lovelace::handle_input(key, &mut state),
                                    Act::Boole1854 => boole::handle_input(key, &mut state),
                                    Act::Shannon1937 => shannon::handle_input(key, &mut state),
                                    Act::Turing1936_1950 => turing::handle_input(key, &mut state),
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

            if let Some(cue) = dialogue.take_cue() {
                audio.play_marker(cue.marker());
            }
            // One daktilo clack per committed letter — in lockstep with the text.
            if dialogue.took_keystroke() {
                audio.daktilo_strike();
            }

            match state.screen_state {
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
                // Phase 2: nothing advances; the world simply breathes and reveals.
                ScreenState::AmbientDesk => {}
                // Phase 3: the live simulation runs.
                ScreenState::ActivePuzzle => {
                    match state.current_act {
                        Act::Jacquard1804 => tick_jacquard(&mut jacquard_puzzle, &mut state, &mut dialogue),
                        Act::Babbage1837 => tick_babbage(&mut babbage_puzzle, &mut state, &mut dialogue),
                        _ => {}
                    }

                    // Act transition → queue the next act's entrance typewriter.
                    if state.current_act != last_act {
                        last_act = state.current_act;
                        state.lookup_active = false;
                        pending_intro = Some(state.current_act);
                    }
                    if let Some(act) = pending_intro {
                        if !dialogue.is_typing() {
                            let (sp, txt, cue) = act_intro(act);
                            dialogue.play(sp, txt, Some(cue));
                            pending_intro = None;
                        }
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
