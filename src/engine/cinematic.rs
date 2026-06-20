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
    render_scene(f, area, &scene, dialogue, frame, awaiting_enter, 0.08);
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
    render_scene(f, area, &scene, dialogue, frame, awaiting_enter, 0.16);
}

fn render_scene(
    f: &mut Frame,
    area: Rect,
    scene: &Scene,
    dialogue: &DialogueEngine,
    frame: u64,
    awaiting_enter: bool,
    glitch: f32,
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
    let img_h = area.height.saturating_sub(2 + text_h + 1);

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
