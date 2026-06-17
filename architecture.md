# THE ENGINE — MASTER TEXT USER INTERFACE (TUI) ARCHITECTURE SPECIFICATION

This master specification compiles the complete layout, engine architecture, state management structures, and psychological degradation mechanics of **THE ENGINE**. It explicitly ports the game’s core systems into an uncompromising, high-fidelity Text User Interface (TUI) optimized for terminal frameworks such as Rust’s `Ratatui` or Python’s `Textual`.

By stripping away the visual insulation of traditional graphics engines, the terminal itself becomes the primary storytelling instrument—a cold, bare-metal environment mapping the systematic dissolution of a brilliant mind.

---

## 1. DESIGN PILLARS & LUDONARRATIVE DISSONANCE

### 1.1 Core Principles

* **Instruction Separated from Mechanism:** The computational lineage is never decorative—it dictates the raw logic, memory layout, and operational constraints of every single puzzle.
* **Bright Puzzle, Dark Undertow:** The systems are completely functional, deep mathematical challenges. The historical, human horror is expressed silently through terminal degradation, mutating logs, and ambient audio cues.
* **The Desk is the Clock:** Traditional numeric countdown overlays, progress bars, and floating UI timers are strictly banned. Time is felt entirely through the physical erosion of text components on the terminal layout.
* **Absolute Interface Isolation:** There is zero mouse interaction, drag-and-drop convenience, or traditional help windowing. Interaction models are low-level and command-driven, evoking the claustrophobic feeling of a raw SSH session on bare hardware.

### 1.2 The Ludonarrative Trap

The game leverages ludonarrative dissonance as a psychological weapon. In the final act (Act VI), the player can formulate structurally perfect, mathematically flawless solutions, yet the engine will throw unpredictable `FATAL ERROR` runtime crashes. This forces the player to experience the profound, systemic frustration of having their agency stripped away by an unyielding administrative override—simulating exactly what the state's chemical castration (Stilboestrol) did to Turing's physical brain.

---

## 2. GLOBAL ENGINE STATE MACHINE (`GlobalStateContext`)

The engine relies on a unified, high-precision global state that simultaneously mutates core layout bounds, color models, text distortion multipliers, and audio playback streams based on current psychological and chemical criteria.

```rust
pub enum Act {
    Jacquard1804,
    Babbage1837,
    Lovelace1843,
    Boole1854,
    Shannon1937,
    Turing1936_1950,
}

pub enum TuringPhase {
    Clarity,  // 1936 - 1940
    Tremor,   // 1950
    Collapse, // 1951
}

pub struct GlobalStateContext {
    pub current_act: Act,
    pub active_turing_phase: TuringPhase,
    pub acts_completed: Vec<Act>,
    
    // The Poison & Chemistry Metrics
    pub stilboestrol_ppm: f32,       // Mutates text-erosion rate and screen jitter
    pub vision_blur_factor: f32,     // Interpolates ANSI color space towards charcoal gray
    pub apple_bites_taken: u8,       // Step decrementer: 0 (whole) to 4 (silhouetted core)
    pub candle_rows_remaining: u16,  // System timer; decrements on step or fault conditions
    
    // Layout Positioning Flags
    pub chemical_drift_seed: u64,    // Dictates random clustering of pill characters
    pub lookup_active: bool,         // Focus interpolation between workspace and paper stack
    
    // Audio Hook Variables
    pub base_heartbeat_bpm: u32,     // Base speed of the low-frequency sub-bass metronome
    pub arrhythmia_multiplier: f32,  // Random timing micro-offsets injected into audio loops
}
```

---

## 3. INTERFACE TOPOGRAPHY (THE ORGANIC GRID)

The terminal window is divided into three fluid, asynchronous rendering buffers. Rather than employing strict, boxed panel borders, the right half of the interface utilizes overlapping characters to construct an unstructured, visceral workspace representing a cluttered physical desk.

```text
┌─ LOG / THE MIND ─────────────────────────────────────────────────────────────────────┐
│ [INFO] System clock: 1952hz. Core memory allocation: stable.                          │
│ [INFO] Internal Metronome Tracker: 72 bpm.                                           │
│ >> "The rhythm of the looms echoes from Lyon..."                                     │
└──────────────────────────────────────────────────────────────────────────────────────┘
┌─ THE WORKSPACE ───────────────────────────┐┌─ THE DESK & PAPERS ─────────────────────────┐
│                                           ││   ,───────────────────────────────.     │
│   NEEDLE MATRIX [ROW: 04/08]              ││  (  ACT I: JACQUARD (1804)         )    │
│   [1] [0] [1] [1] [0] [0] [1] [0] -> 0xB4 ││   `───────────────────────────────'     │
│   [0] [1] [0] [0] [1] [1] [0] [1] -> 0x4D ││    │  [ ░░▄▄▄░░ ]                  │     │
│         ▲                                 ││    │  [ ░█▓▒▓█░ ] <- 1-Bit Portrait│     │
│                                           ││    │  [ ░░▀▄▀░░ ]                  │     │
│                                           ││    │                               │     │
│                                           ││    │ Hand-signed: J.M. Jacquard    │     │
│                                           ││    └──────────────────────────────┘     │
│                                           ││     === [Babbage — 1837] ======         │
│                                           ││      === [Lovelace — 1843] =====        │
│                                           ││                                         │
│                                           ││      ( )        ▄██▄                    │
│                                           ││       │        ▀████▀  <- The Apple     │
│                                           ││     [░░░]       ▀▀                      │
│                                           ││   o   .  o oOo .  <- Scattered Pills    │
└───────────────────────────────────────────┘└─────────────────────────────────────────┘
```

### 3.1 Region 1: The Mind (`[LOG / THE MIND]`)

* **Technical Class:** Continuous `stdout`/`stderr` asynchronous text pipeline.
* **Operational Flow:** At the start of the night, this panel outputs cold, highly accurate system execution metrics, thread definitions, and cycle timings. As the `stilboestrol_ppm` value increases, the rendering thread intercepts these logs. Technical info messages are systematically overridden by unverified memory fragments, court order fragments from the 1952 gross indecency trial, and raw historical journal records.

### 3.2 Region 2: The Workspace (`[THE WORKSPACE]`)

* **Technical Class:** Interactive modal execution buffer.
* **Operational Flow:** The structural environment where all low-level computation occurs. Depending on the current Act, this area reconfigures into a binary matrix editor, an assembly registry console, or a logical gate grid. It is entirely driven by key event handlers; floating dialogue trees or instructional popups are completely omitted.

### 3.3 Region 3: The Desk & Papers (`[THE DESK & PAPERS]`)

* **Technical Class:** High-density, 16-million TrueColor ANSI canvas mapping custom layered text blocks.
* **The Document Stack (Staggered Layers):** Historical documents are drawn mathematically overlapping each other using customized Unicode edge strokes (`┌`, `─`, `┐`, `│`). When an act is cleared, its specific paper element receives a dimmed color profile and drops lower in the render queue, sinking into the shadow stack. The active paper on top displays a meticulously dithered 1-bit pixel art profile of the figure, a textual description of their specific system architecture, and an elegant digital rendering of their cursive signature.
* **The Candle (UI-less Timer):** Rendered using half-block characters (`█`, `▄`, `▀`) to implement a high-gradient wax column with an active, pulsing flame node `( )`. On every loop step or runtime exception, the engine reduces `candle_rows_remaining`. If an infinite loop is executed, the row removal frequency spikes drastically, melting the asset down before the player's eyes.
* **The Chemical Drift (Asynchronous Pills):** Pill artifacts (`o`, `.`, `oOo`) are handled via dynamic coordinate allocation rather than a static layout grid. As toxicity rises, the layout engine executes randomized displacement algorithms using a changing seed value. Pills will spontaneously clutter around the paper stacks, drift into the terminal borders, or cluster into threatening geometries at the screen boundaries, directly breaking the clinical alignment of the interface to mimic failing spatial awareness.

---

## 4. THE CYANO-CLARITY SYSTEM (HIDDEN HINT MECHANIC)

The meşum apple is completely anonymous. The interface presents no blinking prompt, contextual string, keyboard tooltip, or system mention indicating that it is an interactive component.

```text
[ Whole ]          [ Bitten ]          [ Final Phase ]
  ▄██▄                ▄██▄                ▄
 ▀████▀              ▀████░░             ▀██░░
   ▀▀                  ▀▀                  ▀
```

### 4.1 The Loop Mechanics

* **The Choice:** When a player hits a logical bottleneck (e.g., struggling to execute Bernoulli numbers inside the 3-register limit of Act III), they must actively perform a Look Down command sequence via keyboard navigation to shift input focus to the desk coordinates.
* **The Execution Event:** Triggering the interaction key overrides the Mind Panel text buffer. It erases all technical logs for 3 seconds to print a single, fragile string of inner monologue: *“Oh, eating an apple has always cleared my mind...”*
* **The Mechanical Trade-Off:**
  * **The Bright Puzzle (The Clue):** The engine modifies the active paper stack array, rendering a handwritten, highly technical micro-hint or architectural shortcut into the ASCII document margins.
  * **The Dark Undertow (The Price):** The engine calls a bite function on the apple sprite, mutating the character array from a full profile (`▀████▀`) to an eroded variant. Crucially, the global `candle_rows_remaining` step-reduction multiplier is scaled up by 1.5x for the remainder of the night, permanently cutting down the player's macro time window.

---

## 5. TUI PSYCHOLOGICAL PRESSURE ENGINE

The terminal framework maps Turing’s compounding physical breakdown directly to character layout mutations across the three panels.

### 5.1 Vision Blur & Character Erosion

* **Contrast Deceleration:** The engine calculates a dynamic color interpolation map based on `vision_blur_factor`. The initial bright amber (`#FFB000`) and emerald green (`#33FF33`) terminal standards are systematically degraded into low-contrast, clinical grays and deep, muddy charcoal (`#2A2A2A`), forcing physical eye strain on the user.
* **Structural Bit-Flipping:** When `stilboestrol_ppm` hits high thresholds, a runtime mask targets standard text rows. Every N frames, text sequences are fractured into Braille patterns (`⠓`, `⠙`, `⠻`, `⠵`) or pure binary values (`1` / `0`), simulating acute visual aphasia before snapping back to legibility.
* **Layout Stuttering:** The main render pipeline introduces horizontal character cell row-shifting. Independent lines will randomly slide 1 to 2 cells left or right during high-load compilation routines, breaking the geometric sanity of the terminal and simulating intense physical vertigo.

### 5.2 Sound Integration & Tremor Modeling

* **The Heartbeat Metronome:** A dedicated background thread runs a low-frequency, heavy sub-bass metronome loop. Each time a beat is logged inside the Mind Panel, a deep, boğuk pulse triggers. As Act VI approaches, the timing intervals face systemic disruption, injecting rhythmic arrhythmias, skipped beats, and sudden spikes in tempo to build intense physical panic.
* **Voice Bleed Processing:** Character audio logs are fed through narrow high-pass, shortwave radio static filters. The resulting fısıltı tracks are localized to left/right audio channels based on player cursor coordinates, making the historical inventors sound like ghosts embedded within the circuit logic.
* **Keyboard Double-Strike Injection:** To communicate Turing's severe chemical tremor, the input loop systematically alters keypress intervals during the final phases of Act VI. The terminal will register a single physical key press as a rapid double-strike (double-click audio + character repetition), forcing the player to manually step back and delete corrupted input lines.

---

## 6. THE 13-MACHINE PORTING MAP

Every act presents historically valid failures before the player can unlock the optimized architecture. The terminal handles these transitions purely through automated text motion.

```text
[ACT I STATE #1: REEL TEAR]        [ACT II STATE #4: CARRY LOCK]
    ┌────────────────┐                 ┌───┐ ┌───┐ ┌───┐
    │ ▓▓▓▓▓░░  ░░▓▓▓ │                 │ 9 │ │ 9 │ │ 9 │ 
    │       /        │                 └───┘ └───┘ └───┘
    │ [ERR_STR_TEAR] │                 [GEAR_LOCK_VAL_OVERFLOW]
    └────────────────┘                 ─────────────────────────
```

### 6.1 Act I: Jacquard (1804) — Bit Manipulation & Squeezing

* **TUI Puzzle Interface:** An 8x8 interactive matrix window where `1` and `0` cells are toggled with the arrow keys.
* **Historical Failures:**
  * **State 1 (Bouchon Roll):** Fast processing speeds generate excessive text tension. The layout shows ASCII canvas lines split, logging a hard `[ERR_STR_TEAR]` system halt.
  * **State 2 (Falcon Chain):** Allocating separate lines for every single row causes cell density to spike. The window grid freezes with a `[MECHANISM_JAMMED]` flag.
* **The Final Optimization:** The user must compress a complex 100-row damask pattern into a 4-card loop by exploiting the geometric parity and symmetry mirrors of the grid.

### 6.2 Act II: Babbage (1837) — Carry Propagation Debugging

* **TUI Puzzle Interface:** Vertical arrays representing numeric gear columns. Tapping Space advances the main crank total.
* **The Historical Failure (State 4 - Ripple Carry):** When columns cross the 9 → 0 threshold, they attempt to carry simultaneously. The engine flags a severe `[GEAR_LOCK_VAL_OVERFLOW]` alert, turns the workspace font blood-red, and freezes input.
* **The Final Optimization (State 5):** The player must insert specific delay blocks (`[D]`) between column calculations. This reorganizes the simultaneous carry into a sequential, cascading wave that steps text arrays across the cells over explicit clock cycles.

### 6.3 Act III: Lovelace (1843) — 3-Register Pseudo-Assembly

* **TUI Puzzle Interface:** A low-level text line editor mapping precisely to three giant ASCII cylinder graphics representing registers `R1`, `R2`, and `R3`.
* **The Architectural Crisis:** Computing Bernoulli numbers requires writing clean code utilizing strictly `LOAD`, `STORE`, `ADD`, `SUB`, `IF`, `JMP`, and `HALT`.
* **The Memory Ceiling:** The engine enforces a rigid 3-variable ceiling. The moment the user attempts to define a 4th temporary register string, the compiler errors out. The user is forced to perform sacrificial overwriting—intentionally deleting active values inside `R2` or `R3` to hold current loop calculations, managing memory like an extreme, modern edge-device.

### 6.4 Act IV: Boole (1854) — Algebraic Text Refactoring

* **TUI Puzzle Interface:** The entire workspace is flooded by an immense, chaotic, multi-line string equation representing the raw spaghetti thinking of an unoptimized logical tree.
* **The Core Mechanic:** No logic wires are present. The user guides a text cursor over literal sub-expressions, applying De Morgan’s laws and distribution steps via key inputs. Successful logical reductions make massive blocks of redundant text instantly collapse inward with sharp terminal animations, whittling down the chaos into a tight, elegant logical statement.

### 6.5 Act V: Shannon (1937) — Asynchronous Wire Routing

* **TUI Puzzle Interface:** An ASCII circuit board displaying logic symbols connected by custom line paths.
* **The Asynchronous Kabus:** The engine models real-world physics; switches do not transition instantly; they have unique tick-delays built into their logic states. If the series and parallel routing lines are misaligned, current paths hit gates out of phase, causing a race condition. The terminal triggers a localized `[SHORT_CIRCUIT]` explosion of random symbols across the grid, rendering the board dead.

### 6.6 Act VI: Turing (1936/1950) — Universal Automata

* **TUI Puzzle Interface:** A single, continuously scrolling horizontal text strip (The Tape) beneath a state transition matrix.
* **The Convergence:** The user structures a state table that reads, writes, and shifts the tape, effectively implementing a layout capable of processing and replicating every prior engine mechanical model.

---

## 7. COMPUTED TEXT MOTION (ASSET REJECTION)

The engine bans pre-baked graphical frame sets or video extraction files to achieve interactive motion. All visual dynamics are computed frame-by-frame directly inside the text cell matrices.

```text
 Frame $t_0$:                 Frame $t_1$:                 Frame $t_2$:
 ┌─[GEAR]─┐                   ┌─[GEAR]─┐                   ┌─[GEAR]─┐
 │   │    │                   │   /    │                   │   ─    │
 └────────┘                   └────────┘                   └────────┘
```

* **Rotational Dynamics:** Gear components are mapped using mathematical character rotations. The column values directly translate to angles, shifting characters from `│` to `/`, then `─`, then `\`, creating flawless rotational velocity definitions without asset overhead.
* **Electrical Path Highlighting:** Voltage conduction along Shannon’s relays maps directly to true-color ANSI escape sequences. When a path evaluates as active, the engine parses the coordinate rows, injecting raw bright voltage color headers (`\x1b[38;2;51;255;51m`) and active block symbols (`█`), causing live energy arrays to pulse dynamically through the schematic lines.

---

## 8. THE CLOSING FRAME & FINALE ANATOMY

The exact sequence of the final system teardown is non-negotiable, unskippable, and hardcoded into the layout engine:

1. **The Execution Collapse:** Upon forcing the `HALT` condition through the high-intensity vision blur of Act VI Phase 3, the engine locks input loops completely. The low-frequency metronome immediately cuts out, throwing the environment into absolute, deafening silence. A frozen message is left: `FATAL ERROR: SYSTEM HALTED`.
2. **The Binary Waterfall:** The panel borders, ASCII document stacks, text lines, and workspace matrix grids break apart. Every single character cell on the screen systematically mutates into rapid, downward-cascading streams of `1`s and `0`s, drenching the console in binary code until the interface completely flushes to black.
3. **The Center Isolation:** A highly precise, solitary ANSI true-color portrait of a half-eaten green apple forms in the exact center of the black terminal screen.
4. **The Structural Syllogism:** Below the centered apple graphic, the console draws the devastating text of Turing’s actual 1952 formulation line-by-line using a rigid, typewriter-style output cadence:
   ```text
   "Turing believes machines think."
   "Turing lies with men."
   "Therefore, machines do not think."

   — Alan Turing, letter to Norman Routledge, 1952
   ```
5. **The Administrative Seal:** The syllogism clears silently. The console outputs two final lines of historical data in an affectless, bureaucratic system font:
   ```text
   2013 — Royal Pardon.
   It took fifty-nine years to apologize.
   ```

The terminal prompt completely erases. The system locks execution, forcing the player to manually terminate their own terminal session.
