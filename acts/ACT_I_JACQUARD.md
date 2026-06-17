# 🧵 ACT I SPECIFICATION: JACQUARD (1804) — THE LOOM

## 1. THE TOPMOST PAPER: VISUAL SKIN & PORTRAIT

When Act I is triggered, the Chaotic Desk pipeline draws Jacquard’s specific document at the absolute peak of the paper stack (Z-index: 0).

* **The 1-Bit Portrait:** Rendered via a high-contrast text block utilizing custom dithering characters (`█`, `▓`, `▒`, `░`) to depict Joseph Marie Jacquard as a weathered, working-class inventor from Lyon.
* **The Verification Signature:** Synthesized using continuous cursive-mimicking ASCII lines at the base of the sheet:
  ```text
  Hand-signed: J.M. Jacquard
  ```
* **The Static Workspace:** The ambient text on the paper frames the structural reality: *The instruction must be separated from the wood and rope. The pattern must live on card.*

---

## 2. THE 3-STATE CONSTRAINT FUNNEL (THE LESSON OF FAILURE)

As dictated by the puzzle pipeline, the player must hit two real-world historical dead-ends before the stable punched card interface is unlocked.

```text
  [STATE #1: CONTINUOUS ROLL]        [STATE #2: FALCON CHAIN]
       ┌────────────────┐                 ┌───┐ ┌───┐ ┌───┐
       │ ▓▓▓▓▓░░  ░░▓▓▓ │                 │ █ │ │ █ │ │ █ │ 
       │       /        │                 └───┘ └───┘ └───┘
       │ [ERR_STR_TEAR] │                 [MECHANISM_JAMMED]
       └────────────────┘                 ───────────────────
```

### State 1: The Bouchon Paper Roll (Continuous Stream)

* **The Interface:** The active workspace presents a long, single-line horizontal array of bits.
* **The Failure Mode:** When the player executes Run, the simulation attempts to spin the loom at high speed. The global engine state increases the tension flag linearly (`update_tension()`). At 6 cards or equivalent thread lengths, tension hits 1.0, triggering an immediate `snapped = true` condition. The terminal interface splits the ASCII boundary lines with a jagged `/` stroke and halts execution with a raw, blinking error: `[ERR_STR_TEAR]: Continuous paper roll torn under mechanical tension.`
* **Biting Scolding (Tier 1):**
  ```text
  [JACQUARD VIA MIND_LOG]: "A paper roll? A child's toy. Torn after two metres — you've ruined the silk."
  ```

### State 2: The Falcon Card Chain (Rigid Volumetric Blocks)

* **The Interface:** The layout switches to rigid blocks lacing card data together.
* **The Failure Mode:** If the player attempts to bypass the tension limit by instantiating more than 6 separate individual cards (`puzzle.cards.len() > 6`), the sheer weight of the virtual blocks causes a systemic overflow. The cell matrices freeze solid, and the engine issues a mechanical deadlock exception: `[MECHANISM_JAMMED]: Volumetric block-chain exceeds mass displacement boundaries.`
* **Biting Scolding (Tier 2):**
  ```text
  [JACQUARD VIA MIND_LOG]: "Strong, yes. And heavy as a cart. It seizes — feel it bind. The loom cannot move."
  ```

### State 3: The Punched Card (The Stable Paradigm)

* **The Optimization Condition:** Unlocked only when the player scales their card infrastructure down to ≤4 cards (`MAX_CARDS = 4`). The tension clears to nominal levels, and true computation begins.

---

## 3. THE CORE MEMORY-COMPRESSION PUZZLE

Once State 3 is active, the player is presented with an uncompromising bit-manipulation and optimization problem.

### 3.1 The Target Matrix

The engine demands the reproduction of a 4-row complex damask weave (`TARGET_MOTIF`):

```rust
pub const TARGET_MOTIF: [[u8; 8]; 4] = [
    [1, 0, 1, 0, 1, 0, 1, 0], // Row 0
    [0, 1, 0, 1, 0, 1, 0, 1], // Row 1
    [1, 0, 1, 0, 1, 0, 1, 0], // Row 2
    [0, 1, 0, 1, 0, 1, 0, 1], // Row 3
];
```

### 3.2 The Constraint Optimization Trap

The naive approach is to use 4 separate cards to map the 4 rows. However, the engine tracks user input strictly through an error counter. If the player loops the execution using 4 full cards, Jacquard fires Tier 3 scolding, indicating a failure to find the elegant algorithmic truth.

The target motif has a period-2 repeat pattern. The optimal program requires the user to compress the 4-row layout into exactly 2 cards, forcing the evaluation thread to loop back (`row_idx % puzzle.cards.len()`). The core puzzle is won not by typing more, but by compressing the instruction to its absolute bare-metal essence.

---

## 4. TUI RENDER LAYOUT & COMPUTED METRIC MOTION

The active interface cell within `[THE WORKSPACE]` renders the running loom program via high-density character arrays parsed directly from state memory.

```text
 ┌─ [THE WORKSPACE: ACT I INTERACTIVE] ────────────────────────────────────────────────┐
 │  [ PROGRAM STATUS: WEAVING ]    [ HARDWARE TENSION: ■■■■■■░░░ 62% ]                 │
 │                                                                                     │
 │  CARD PROGRAM ROW-BY-ROW:                                                           │
 │  [00]  ► ▓▓░░▓▓▓▓░░░░▓▓░░  ✓                                                        │
 │  [01]    ░░▓▓░░░░▓▓▓▓░░▓▓  ✓                                                        │
 │                                                                                     │
 │  [ FEEDBACK MATRIX ]:                                                               │
 │  Active Row Index: [02/04]                                                          │
 │  Current Hex Fetch: 0xB4 -> 0x4D                                                    │
 └─────────────────────────────────────────────────────────────────────────────────────┘
```

### 4.1 Frame Execution (`render_weave`)

* **Active Row Indicator:** When `puzzle.weaving` is flagged as true, a processing cursor `►` steps through the card indices sequentially.
* **Computed Text Representation:** The active bits are printed on the grid using triplets of high-density blocks (`▓▓▓` for a hole / active warp lift, `░░░` for no hole / thread stays down).
* **Live Hex Verification:** Beside every text block, the terminal prints the live evaluation status (`✓` or `✗`) along with the evaluated hexadecimal conversion of the binary string (`0xB4`, `0x4D`).

### 4.2 Silent Error Dynamics

If a card row matches incorrectly against the `TARGET_MOTIF`, no red warning boxes or warning text pops up. The matching flag drops a hard `✗` character at the terminal boundary row. Simultaneously, the specific row line begins to tremble slightly (horizontal line cell shifting via `stilboestrol_ppm` noise calculation), signaling to the technical user exactly where the logical thread is unoptimized.

---

## 5. THE SHADOW: NARRATIVE BREAKPOINT & AUDIO BLEED

The moment `lovelace_is_solved()` or the respective Jacquard validation evaluates as a complete, perfect 2-card loop, the execution loops halt instantly. The high-frequency clicking of the loom shuttle cuts out.

### 5.1 The Victory Whisper

The Mind Panel clears its system metrics to drop Jacquard's original dramatized prose over the console:
```text
[JACQUARD]: "Faster. Flawless. A perfect loop. But do you hear the street outside? Thousands of weavers will starve for these perfect cards. You optimized the machine — and deleted the man from it, like a bug."
```

### 5.2 The Residue Object Selection

An ASCII punched card artifact is deposited permanently onto the right edge of The Chaotic Desk. If the player focuses input text controls onto this object to inspect its residue letter, the engine triggers an audio stream: Jacquard's working-class, weathered voice echoes in the headphones with heavy shortwave radio filtering, while a faint, low-frequency voice bleed from the locked Babbage sheet starts muttering from the adjacent screen boundary.

```text
[BABBAGE VOICE BLEED VIA COOPERATING THREAD]: "...she saw what I did not. The cards could decide, not merely record..."
```
