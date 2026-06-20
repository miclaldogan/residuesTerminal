use std::path::PathBuf;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::Frame;

use super::image_engine;
use super::mind_log::DialogueEngine;
use super::state::Act;

// ─────────────────────────────────────────────────────────────────────────────
// Cinematic act intros (the historic figure's portrait + biography) and tragic
// outros (their downfall). Both reuse the half-block pixel-art engine for the scene
// image and stream the narration as a typewriter line word-wrapped along the bottom.
// All scene data is static; the only per-scene cost is one cached image decode, warmed
// by `preload_intro`/`preload_outro` the instant the state activates.
// ─────────────────────────────────────────────────────────────────────────────

/// A single cinematic beat — a title, the scene image, and the lines to scroll.
pub struct Scene {
    pub title: &'static str,
    pub image_rel: &'static str,
    pub lines: &'static [&'static str],
}

/// One-based act id ↔ act enum. Ids are the canonical handle the `ScreenState`
/// cinematic variants carry, so the heavy `Act` value never rides inside the state.
pub fn id_from_act(act: Act) -> u8 {
    match act {
        Act::Jacquard1804 => 1,
        Act::Babbage1837 => 2,
        Act::Lovelace1843 => 3,
        Act::Boole1854 => 4,
        Act::Shannon1937 => 5,
        Act::Turing1936_1950 => 6,
    }
}

pub fn act_from_id(id: u8) -> Act {
    match id {
        1 => Act::Jacquard1804,
        2 => Act::Babbage1837,
        3 => Act::Lovelace1843,
        4 => Act::Boole1854,
        5 => Act::Shannon1937,
        _ => Act::Turing1936_1950,
    }
}

/// Resolve the `images/` asset directory (shared with the image engine).
fn images_base() -> PathBuf {
    image_engine::images_dir()
}

pub fn intro(act_id: u8) -> Scene {
    match act_id {
        1 => Scene {
            title: "ACT I · JOSEPH-MARIE JACQUARD · LYON, 1804",
            image_rel: "jacquardAct1.png",
            lines: &[
                "I was a weaver's son, bent over mechanical looms since my boyhood in Lyon. Where others saw only grueling labour, I saw a hidden thread of mathematics.",
                "My punched cards taught the wood and metal to read a pattern \u{2014} the first machine in human history instructed not by a human hand, but by cold holes in a card.",
            ],
        },
        2 => Scene {
            title: "ACT II · CHARLES BABBAGE · LONDON, 1837",
            image_rel: "babbage_intro.png",
            lines: &[
                "They called me a madman for wanting to compute with brass and steam. My Analytical Engine was designed to weave algebraic patterns, just as Jacquard's loom wove silk.",
                "Yet they cut my funding, leaving my gears to rust \u{2014} a mind, I am certain, a full century ahead of the crude tools of its own age.",
            ],
        },
        3 => Scene {
            title: "ACT III · AUGUSTA ADA, COUNTESS OF LOVELACE · 1843",
            image_rel: "lovelace_intro.png",
            lines: &[
                "My mother dreaded the poetry in my father's blood, so she drowned me in numbers. She never guessed the two would fuse inside me into something new.",
                "I gazed at Babbage's engine and saw what he could not: it need not merely reckon sums. It could weave any logic at all. In its margins I wrote the first program \u{2014} a loop folding upon itself \u{2014} for a machine not yet born.",
            ],
        },
        4 => Scene {
            title: "ACT IV · GEORGE BOOLE · CORK, 1854",
            image_rel: "boole_intro.png",
            lines: &[
                "I had little schooling and less money; all I truly possessed was an obsession with the silent laws that lie beneath human thought.",
                "I reduced the whole of reasoning to two values and a handful of operations \u{2014} true and false; and, or, not. I believed I was charting the mind of God. I had no notion I was drafting the alphabet of every machine to come.",
            ],
        },
        5 => Scene {
            title: "ACT V · CLAUDE SHANNON · CAMBRIDGE, 1937",
            image_rel: "shannon_intro.png",
            lines: &[
                "As a boy I loved nothing so much as relays clicking in the dark. Then, all at once, I saw it: a switch is a proposition \u{2014} open or shut, false or true.",
                "Wire Boole's logic into circuits of metal, and the circuit itself begins to reason. Thought and electricity, married at last. I confess I built it half for the sheer wonder of the thing.",
            ],
        },
        6 => Scene {
            title: "ACT VI · ALAN TURING · 1936\u{2013}1950",
            // The endgame portrait: a heavy, low-contrast silhouette of Alan at his
            // Wilmslow desk under failing candlelight. Falls back to text until shipped.
            image_rel: "turing_intro.png",
            lines: &[
                "I imagined a machine of infinite patience: a tape, a single head, a table of rules. From that bare skeleton, I proved, any computation whatever could be wrought.",
                "One machine to imitate all machines. I saw it whole \u{2014} universal, exact \u{2014} years before the wires existed to hold it; if only the world would let such a thing, and such a man, simply be.",
            ],
        },
        _ => Scene {
            title: "ACT VI · ALAN TURING · 1936\u{2013}1950",
            image_rel: "turing_intro.png",
            lines: &[
                "I imagined a machine of infinite patience \u{2014} universal, exact \u{2014} years before the wires existed to hold it.",
            ],
        },
    }
}

pub fn outro(act_id: u8) -> Scene {
    match act_id {
        1 => Scene {
            title: "\u{2014} THE LOOM REMEMBERS \u{2014}",
            image_rel: "jacquard_outro.png",
            lines: &[
                "The riots faded; the cards endured. Within a generation, Jacquard's looms clothed half of Europe.",
                "He died honoured \u{2014} yet never grasped that his holes in card had taught mankind to program a machine.",
            ],
        },
        2 => Scene {
            title: "\u{2014} THE UNFINISHED ENGINE \u{2014}",
            image_rel: "babbage_outro.png",
            lines: &[
                "The government withdrew its funding. The great engine was never completed in his lifetime.",
                "Babbage died embittered, his masterpiece a heap of precise brass \u{2014} a mind a century ahead of its tools.",
            ],
        },
        3 => Scene {
            title: "\u{2014} ENCHANTRESS OF NUMBERS \u{2014}",
            image_rel: "lovelace_outro.png",
            lines: &[
                "Cancer took Ada at thirty-six. Her notes gathered dust, dismissed as a poet's daydream.",
                "A hundred years would pass before the world understood she had written the first program of all.",
            ],
        },
        4 => Scene {
            title: "\u{2014} AN UNTIMELY RAIN \u{2014}",
            image_rel: "boole_outro.png",
            lines: &[
                "Caught in a downpour, Boole lectured soaked to the skin and fell to fever.",
                "He died at forty-nine \u{2014} never knowing his two values would become the alphabet of every computer.",
            ],
        },
        5 => Scene {
            title: "\u{2014} THE QUIET YEARS \u{2014}",
            image_rel: "shannon_outro.png",
            lines: &[
                "Shannon lived long, juggling and building whimsical machines, his information theory quietly remaking the world.",
                "In his final years a fog took his memory \u{2014} the man who measured information, slowly losing his own.",
            ],
        },
        _ => Scene {
            title: "\u{2014} THE BITTEN APPLE \u{2014}",
            image_rel: "turing_outro.png",
            lines: &[
                "Prosecuted for who he was, sentenced to chemical ruin, Turing was stripped of the secrets he had guarded.",
                "He died beside a half-eaten apple. Decades later, a nation he saved would beg his pardon.",
            ],
        },
    }
}

/// Preload (warm-decode) a scene's image so the first frame never stalls.
pub fn preload_intro(act_id: u8) {
    image_engine::preload(&images_base().join(intro(act_id).image_rel));
}
pub fn preload_outro(act_id: u8) {
    image_engine::preload(&images_base().join(outro(act_id).image_rel));
}

/// Render an intro scene: portrait centred via the half-block engine under a low glitch
/// whisper, title across the top, the current biography line word-wrapped at the foot.
pub fn render_intro(
    f: &mut Frame,
    area: Rect,
    act_id: u8,
    dialogue: &DialogueEngine,
    frame: u64,
    awaiting_enter: bool,
) {
    let scene = intro(act_id);
    render_scene(f, area, &scene, dialogue, frame, awaiting_enter, 0.08, 0);
}

/// Render an outro scene: the figure's downfall, with a touch more interference to
/// underscore the tragedy.
pub fn render_outro(
    f: &mut Frame,
    area: Rect,
    act_id: u8,
    dialogue: &DialogueEngine,
    frame: u64,
    awaiting_enter: bool,
) {
    let scene = outro(act_id);
    // The bitten-apple finale (Act VI) hoists its image high into the centre-top, well
    // clear of the bottom narrative box (a generous 12-row lift from the dialogue band).
    let img_gap = if act_id >= 6 { 12 } else { 0 };
    render_scene(f, area, &scene, dialogue, frame, awaiting_enter, 0.16, img_gap);
}

/// Frames per character for the credits typewriter — the same incremental cadence as the
/// rest of the game's platen, applied to the closing roll.
const CREDIT_FRAMES_PER_CHAR: u64 = 3;

/// Centre a partially-revealed line: the prefix is laid out at the position the *full*
/// line would occupy (so it settles dead-centre), with a blinking platen cursor while it
/// is still the actively-streaming line. Returns `true` once fully revealed.
fn stream_center(buf: &mut Buffer, area: Rect, y: u16, full: &str, budget: &mut usize, col: Color, frame: u64) -> bool {
    let total = full.chars().count();
    let shown = (*budget).min(total);
    *budget = budget.saturating_sub(shown);
    let x = area.x + area.width.saturating_sub(total as u16) / 2;
    let mut s: String = full.chars().take(shown).collect();
    let streaming = shown < total;
    // Show the platen cursor only on the line that is actively streaming (it has begun
    // but not finished) — lines not yet reached stay blank, not littered with cursors.
    if streaming && shown > 0 && (frame / 8) % 2 == 0 {
        s.push('\u{2588}');
    }
    let void = Color::Rgb(0, 0, 0);
    put_str(buf, x, y, &s, col, void);
    !streaming
}

// ── Finale timing (frames @ ~62.5 fps). ──
const HALT_HOLD: u64 = 100;     // "FATAL ERROR: SYSTEM HALTED" frozen, in silence
const WATERFALL_DUR: u64 = 240; // the binary cascade draining to black
const APPLE_HOLD: u64 = 70;     // the apple alone before the syllogism types
const PHASE_GAP: u64 = 100;     // a held beat between movements

/// The 1952 letter to Norman Routledge — the structural syllogism.
const SYLLOGISM: &[&str] = &[
    "Turing believes machines think.",
    "Turing lies with men.",
    "Therefore, machines do not think.",
    "",
    "\u{2014} Alan Turing, letter to Norman Routledge, 1952",
];

/// The administrative seal.
const PARDON: &[&str] = &[
    "2013  \u{2014}  Royal Pardon.",
    "It took fifty-nine years to apologize.",
];

/// A tiny inline LCG for the deterministic-per-cell waterfall (no deps, no state).
fn cell_rng(x: u16, y: u16, t: u64) -> u64 {
    let mut s = (x as u64)
        .wrapping_mul(0x9E3779B97F4A7C15)
        .wrapping_add((y as u64).wrapping_mul(0xC2B2AE3D27D4EB4F))
        .wrapping_add(t);
    s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    s
}

/// Total characters in a block (for deriving when a streamed block is done).
fn block_chars(lines: &[&str]) -> usize {
    lines.iter().map(|l| l.chars().count()).sum()
}

/// The frame (since FinalCredits began) at which the whole teardown has played out and the
/// player is finally allowed to leave. The finale is unskippable per spec §8, so the main
/// loop gates the ENTER key against this. Derived purely from the phase constants.
pub fn credits_exit_frame() -> u64 {
    let w_end = HALT_HOLD + WATERFALL_DUR;
    let syll_start = w_end + APPLE_HOLD;
    let syll_done = syll_start + block_chars(SYLLOGISM) as u64 * CREDIT_FRAMES_PER_CHAR;
    let pardon_start = syll_done + PHASE_GAP;
    let pardon_done = pardon_start + block_chars(PARDON) as u64 * CREDIT_FRAMES_PER_CHAR;
    pardon_done + PHASE_GAP
}

/// The closing teardown, hardcoded and unskippable per spec §8 (the metronome is cut to
/// silence in the audio layer the instant this state is entered). Driven by `elapsed`
/// frames since FinalCredits began:
///   FATAL ERROR → binary waterfall → centred apple → 1952 syllogism → 2013 pardon → exit.
pub fn render_final_credits(f: &mut Frame, area: Rect, frame: u64, elapsed: u64) {
    let buf = f.buffer_mut();
    let void = Color::Rgb(0, 0, 0);
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let cell = buf.get_mut(x, y);
            cell.set_char(' ');
            cell.fg = void;
            cell.bg = void;
        }
    }

    let cy = area.y + area.height / 2;

    // ── Phase 1 — the frozen halt. ──
    if elapsed < HALT_HOLD {
        let blink = (frame / 12) % 2 == 0;
        let col = if blink { Color::Rgb(255, 70, 40) } else { Color::Rgb(120, 30, 15) };
        put_center(buf, area, cy, "FATAL ERROR: SYSTEM HALTED", col);
        return;
    }

    // ── Phase 2 — the binary waterfall: every cell mutates into a downward stream of
    //    1s and 0s, draining to black column by column. ──
    let w_start = HALT_HOLD;
    let w_end = HALT_HOLD + WATERFALL_DUR;
    if elapsed < w_end {
        let local = elapsed - w_start;
        for x in area.x..area.x + area.width {
            // Per-column phase offset + a 70%-of-duration fall so trailing columns finish
            // slightly later — the cascade reads as rain, not a hard wipe.
            let offset = (cell_rng(x, 0, 7) % 40) as u64;
            let prog = (local.saturating_sub(offset)) as f32 / (WATERFALL_DUR as f32 * 0.70);
            let black_top = (prog.clamp(0.0, 1.0) * area.height as f32) as u16;
            for row in 0..area.height {
                let y = area.y + row;
                if row < black_top {
                    continue; // already drained to black
                }
                let digit = if cell_rng(x, y, elapsed / 2) & 1 == 0 { '0' } else { '1' };
                // Bright leading edge, fading to dim emerald down the stream.
                let lead = row == black_top;
                let depth = (row - black_top) as f32 / area.height.max(1) as f32;
                let g = (235.0 - depth * 150.0) as u8;
                let col = if lead { Color::Rgb(200, 255, 200) } else { Color::Rgb(40, g.max(60), 40) };
                put(buf, x, y, digit, col, void);
            }
        }
        return;
    }

    // ── Post-waterfall — the apple centrepiece, held above the text band. ──
    let apple_y = cy.saturating_sub(5);
    draw_apple(buf, area, apple_y);

    // Streaming clocks derived from the (deterministic) phase boundaries.
    let syll_start = w_end + APPLE_HOLD;
    let syll_done = syll_start + block_chars(SYLLOGISM) as u64 * CREDIT_FRAMES_PER_CHAR;
    let pardon_start = syll_done + PHASE_GAP;
    let pardon_done = pardon_start + block_chars(PARDON) as u64 * CREDIT_FRAMES_PER_CHAR;
    let exit_start = pardon_done + PHASE_GAP;

    if elapsed < pardon_start {
        // ── Phase 3 — the syllogism, typed line by line below the apple. ──
        let mut budget = (elapsed.saturating_sub(syll_start) / CREDIT_FRAMES_PER_CHAR) as usize;
        let ink = Color::Rgb(222, 198, 150);
        let mut ty = cy;
        for line in SYLLOGISM {
            let col = if line.starts_with('\u{2014}') { Color::Rgb(150, 130, 96) } else { ink };
            stream_center(buf, area, ty, line, &mut budget, col, frame);
            ty += 1;
        }
    } else {
        // ── Phase 4 — the syllogism clears silently; the administrative seal types in an
        //    affectless, bureaucratic gray. ──
        let mut budget = (elapsed.saturating_sub(pardon_start) / CREDIT_FRAMES_PER_CHAR) as usize;
        let seal = Color::Rgb(150, 150, 150);
        let mut ty = cy + 1;
        let mut all_done = true;
        for line in PARDON {
            let done = stream_center(buf, area, ty, line, &mut budget, seal, frame);
            all_done &= done;
            ty += 2;
        }

        // ── Phase 5 — the dead session: title, author, and the exit prompt, only once the
        //    whole teardown has settled. ──
        if elapsed >= exit_start && all_done {
            let py = area.y + area.height.saturating_sub(3);
            put_center(buf, area, py, "RESIDUES \u{2014} An Engine of Residual Minds", Color::Rgb(90, 84, 72));
            put_center(buf, area, py + 1, "Developed by Iclal Dogan", Color::Rgb(70, 66, 56));
            let on = (frame / 24) % 2 == 0;
            let prompt = if on { Color::Rgb(120, 112, 96) } else { Color::Rgb(50, 47, 40) };
            put_center(buf, area, py + 2, "[ the terminal will not answer \u{2014} press ENTER to leave ]", prompt);
        }
    }
}

/// The half-eaten green apple silhouette, centred horizontally at `top`. Bite + stem.
fn draw_apple(buf: &mut Buffer, area: Rect, top: u16) {
    let body = Color::Rgb(120, 200, 80); // living green
    let dark = Color::Rgb(40, 80, 30);   // the bitten shadow
    let stem = Color::Rgb(110, 80, 40);
    let void = Color::Rgb(0, 0, 0);
    let art: [(&str, Color); 4] = [
        ("  \u{2572}      ", stem), // the stem ╲
        (" \u{2584}\u{2588}\u{2588}\u{2584}   ", body),
        ("\u{2580}\u{2588}\u{2588}\u{2588}\u{2588}\u{2591}\u{2591} ", body),
        ("  \u{2580}\u{2580}    ", body),
    ];
    let w = 8u16;
    let x = area.x + area.width.saturating_sub(w) / 2;
    for (i, (line, col)) in art.iter().enumerate() {
        // The bite (░) is rendered in the darker shade for depth.
        let mut cx = x;
        for ch in line.chars() {
            let c = if ch == '\u{2591}' { dark } else { *col };
            put(buf, cx, top + i as u16, ch, c, void);
            cx += 1;
        }
    }
}

fn render_scene(
    f: &mut Frame,
    area: Rect,
    scene: &Scene,
    dialogue: &DialogueEngine,
    frame: u64,
    awaiting_enter: bool,
    glitch: f32,
    img_bottom_gap: u16,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    // Black the whole canvas first so nothing from a prior screen bleeds through.
    {
        let buf = f.buffer_mut();
        let void = Color::Rgb(0, 0, 0);
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                let cell = buf.get_mut(x, y);
                cell.set_char(' ');
                cell.fg = void;
                cell.bg = void;
            }
        }
    }

    // Vertical budget: title row at top, an image band, then a narration band at the
    // foot. On very short terminals the image band collapses to nothing gracefully.
    let title_y = area.y;
    let text_h: u16 = 6.min(area.height.saturating_sub(2));
    let img_y = area.y + 2;
    // `img_bottom_gap` lifts the image clear of the narration band (used by the apple
    // finale) by shrinking the image band from the bottom — extra black breathing room.
    let img_h = area.height.saturating_sub(2 + text_h + 1 + img_bottom_gap);

    // Title.
    {
        let buf = f.buffer_mut();
        let amber = Color::Rgb(205, 150, 70);
        put_center(buf, area, title_y, scene.title, amber);
    }

    // Portrait / scene image via the pixel-art engine (its own bounds clipping).
    if img_h > 0 {
        let img_w = area.width.min(area.width); // full width; engine letterboxes inside
        let img_area = Rect { x: area.x, y: img_y, width: img_w, height: img_h };
        image_engine::draw_pixel_art(f, img_area, &images_base().join(scene.image_rel), glitch, frame);
    }

    // Narration band — kept perfectly clear and pure black so the typewriter text reads
    // cleanly beneath the wide-cropped scene above it.
    let buf = f.buffer_mut();
    let band_y = area.y + area.height.saturating_sub(text_h);
    let band_bg = Color::Rgb(0, 0, 0);
    for y in band_y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if x < buf.area().width && y < buf.area().height {
                let cell = buf.get_mut(x, y);
                cell.set_char(' ');
                cell.bg = band_bg;
                cell.fg = band_bg;
            }
        }
    }

    // The current narration line, typewritten and word-wrapped.
    let ink = Color::Rgb(222, 198, 150);
    let max_w = area.width.saturating_sub(6).max(8) as usize;
    let mut text = dialogue.visible();
    let typing = dialogue.is_typing();
    if typing && (frame / 16) % 2 == 0 {
        text.push('\u{2588}');
    }
    let wrapped = wrap_words(&text, max_w);
    let lines_to_show = (text_h.saturating_sub(1)) as usize;
    let start = wrapped.len().saturating_sub(lines_to_show);
    for (i, line) in wrapped[start..].iter().enumerate() {
        let y = band_y + i as u16;
        let x = area.x + 3;
        put_str(buf, x, y, line, ink, band_bg);
    }

    // ENTER prompt once the line has finished streaming.
    if awaiting_enter && !typing {
        let prompt = "[ press ENTER to continue ]";
        let on = (frame / 24) % 2 == 0;
        let glow = if on { Color::Rgb(205, 150, 70) } else { Color::Rgb(70, 56, 36) };
        let py = area.y + area.height.saturating_sub(1);
        put_center(buf, area, py, prompt, glow);
    }
}

// ── Small local text helpers (kept here so the module is self-contained) ──────

fn wrap_words(text: &str, max_w: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
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
    if !cur.is_empty() {
        lines.push(cur);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn put(buf: &mut Buffer, x: u16, y: u16, ch: char, fg: Color, bg: Color) {
    let Rect { x: ax, y: ay, width: aw, height: ah } = *buf.area();
    if x >= ax && x < ax + aw && y >= ay && y < ay + ah {
        let cell = buf.get_mut(x, y);
        cell.set_char(ch);
        cell.fg = fg;
        cell.bg = bg;
    }
}

fn put_str(buf: &mut Buffer, mut x: u16, y: u16, s: &str, fg: Color, bg: Color) {
    for ch in s.chars() {
        put(buf, x, y, ch, fg, bg);
        x = x.saturating_add(1);
    }
}

fn put_center(buf: &mut Buffer, area: Rect, y: u16, s: &str, fg: Color) {
    let w = s.chars().count() as u16;
    let x = area.x + area.width.saturating_sub(w) / 2;
    let bg = if y < buf.area().height && x < buf.area().width { buf.get(x, y).bg } else { Color::Rgb(0, 0, 0) };
    put_str(buf, x, y, s, fg, bg);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn id_act_roundtrip() {
        for id in 1..=6u8 {
            assert_eq!(id_from_act(act_from_id(id)), id);
        }
    }

    #[test]
    fn scenes_have_content_for_every_act() {
        for id in 1..=6u8 {
            assert!(!intro(id).lines.is_empty());
            assert!(!outro(id).lines.is_empty());
            assert!(!intro(id).title.is_empty());
        }
    }

    #[test]
    fn intro_and_outro_render_without_panic() {
        let dlg = DialogueEngine::new();
        for (w, h) in [(120u16, 40u16), (80, 24), (30, 10), (12, 5)] {
            let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
            term.draw(|f| {
                let a = f.size();
                render_intro(f, a, 1, &dlg, 5, true);
            })
            .unwrap();
            term.draw(|f| {
                let a = f.size();
                render_outro(f, a, 3, &dlg, 9, false);
            })
            .unwrap();
        }
    }
}
