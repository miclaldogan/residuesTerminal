# ⚙️ ACT III SPECIFICATION: LOVELACE (1843) — THE FIRST ALGORITHM

## 1. THE ACTIVE PAPER: VISUAL SKIN & PORTRAIT

When Act III triggers, the engine sets `GlobalStateContext.current_act = Act::Lovelace1843`. The Chaotic Desk layout engine pushes Ada Lovelace’s specific document to the absolute foreground (Z-index: 0), plunging Babbage's difference schemas into the dimmed background shadow stack.

* **The 1-Bit Portrait:** Formed via meticulous character dithering (`█`, `▓`, `▒`, `░`), depicting Ada Lovelace as young, aristocratic, and feverishly sharp, yet exhibiting the subtle structural strain of her compounding illness.
* **The Verification Signature:** Rendered smoothly at the bottom boundary of the text block using fine cursive ASCII line combinations:
  ```text
  Hand-signed: Augusta Ada Lovelace
  ```
* **The Static Workspace:** The ambient text on the paper frames the conceptual pivot: *The engine may weave algebraic patterns just as the Jacquard loom weaves flowers and leaves. The cards must decide and repeat.*

---

## 2. THE 2-STATE PROGRAMMING FUNNEL

The compilation workspace subjects the user to the brutal limitations of linear control flow before granting conditional looping capabilities.

```text
  [STATE #6: LINEAR SPAGHETEL FLOW]     [STATE #7: THE SNAKE LOOP]
         ┌───────────────────┐               ┌───────────────────┐
         │ 1. LOAD  R1, 0x01 │               │ 1. LOAD  R1, 0x01 │
         │ 2. ADD   R1, R2   │               │ 2. SUB   R1, R2   │
         │ 3. STORE R1, R3   │        ┌───── │ 3. IF_Z  R1, JMP  │
         │ [ERR_MEM_OVERRUN] │        └────> │ 4. JMP   2        │
         └───────────────────┘               └───────────────────┘
```

### State 6: Spaghetti / Linear Flow (The Structural Wall)

* **The Interface:** A linear pseudo-assembly terminal lacking structural conditional jump execution flags.
* **The Failure Mode:** The user is tasked with tabulating the complex Bernoulli numbers (matching the scope of the historic Note G). Without conditional branches, instructions must be explicitly written sequentially line-by-line. As the computation scales, the text grid hits a hard allocation limit. The memory registers flood, logging a low-level failure: `[ERR_MEM_OVERRUN]: Execution sequence out of bounds. Linear allocation bounds broken.`
* **Biting Scolding (Tier 1):**
  ```text
  [LOVELACE VIA MIND_LOG]: "Did you take this for a flat reading-card? How far will you stretch the machine as the numbers grow? You need a loop."
  ```

### State 7: Loop & Branch (The Stable Paradigm)

* **The Optimization Condition:** Unlocked once the core compiler registers valid conditional loop tokens (`IF`, `JMP`). Linear redundancy collapses instantly into a tight, cyclic instruction array.

---

## 3. THE 3-REGISTER MEMORY CEILING (THE HARDCORE PUZZLE)

Once State 7 is fully active, the core programming challenge shifts to a ruthless memory-space optimization constraint.

### 3.1 The Low-Level Token Set

The interpreter parses exactly seven distinct low-level instructions:
* `LOAD r, v` — Insert raw value into target register cylinder.
* `STORE src, dst` — Copy bit values between cylinders.
* `ADD r, s` — Perform arithmetic addition on registers.
* `SUB r, s` — Perform arithmetic subtraction on registers.
* `IFZeroJmp r, label` — Branch control flow if register resolves to zero.
* `JMP label` — Force control flow jump to explicit line index.
* `HALT` — Cleanly terminate current loop execution.

### 3.2 The Chessboard Constraint

The engine enforces a rigid, non-negotiable memory limit: **strictly 3 variables (`R1`, `R2`, `R3`) are instantiated**.
* `R1` functions as the active accumulator.
* `R2` tracks the loop iteration sequence.
* `R3` operates as a single temporary buffer slot.

Attempting to define a 4th slot causes an immediate compilation failure. To evaluate Bernoulli numbers within 3 slots, brute-force linear programming is physically impossible. The user is forced to perform **sacrificial overwriting**—actively deleting active calculations inside `R2` or `R3` midway through a loop execution to borrow the slot for secondary processing. It forces developers to manage registers like high-precision chess pieces.

---

## 4. TUI RENDER LAYOUT & TIME-INTRINSIC ANIMATION

The active cell inside `[THE WORKSPACE]` renders the program assembly console alongside live ASCII register cylinder blocks.

```text
 ┌─ [THE WORKSPACE: ACT III PROGRAM CONSOLE] ──────────────────────────────────────────┐
 │  [ RUN STATE: EXECUTING ]                      [ ACTIVE INSTRUCTION VALUE: PC 02 ]  │
 │                                                                                     │
 │  LINE ASSEMBLY EDITOR:                           CYLINDER ARRAY (REGISTERS):        │
 │  01. LOAD  R1, 0x01                             ┌────────┐  ┌────────┐  ┌────────┐  │
 │  02. SUB   R1, R2                               │  0x05  │  │  0x02  │  │  0x00  │  │
 │  03. IF_Z  R1, 05   ◄ (Current PC)              │  [██]  │  │  [▓░]  │  │  [░░]  │  │
 │  04. JMP   01                                   └────────┘  └────────┘  └────────┘  │
 │  05. HALT                                        [ R1 ]      [ R2 ]      [ R3 ]     │
 │                                                                                     │
 │  [ DIAGNOSTIC INTERPOLATION ]:                                                      │
 │  Infinite Loop Step Counter: 0024 / 1000 MAX | Candle Burn Scale: 1.5x Multiplier  │
 └─────────────────────────────────────────────────────────────────────────────────────┘
```

### 4.1 Frame Execution & Mechanical Dance

* **Cylinder Rotation:** The `puzzle.cylinders_spin` offsets drive dynamic terminal cell transformations. Toggling an arithmetic loop step triggers a text dance: the selected ASCII cylinder frames smoothly step their cell rows up and down (`[██]` → `[▓░]` → `[░░]`), computing metric changes through live terminal movement.
* **The Infinite Loop Burn:** If the user logs an invalid exit condition, step execution hits infinite iterations. The register cylinders are thrashed at maximum refresh rates. No error text is generated; the engine directly targets `GlobalStateContext.candle_rows_remaining`, melting the pixel-art candle down 5 times faster than baseline to make time loss physical.

### 4.2 Silent Error Dynamics

If a line runs into an invalid register state, the specific cylinder that missed its update loop stops dead-still while the others thrash erratically. This structural visual misalignment serves as a silent indicator of the bug, pointing the user directly to the missing `STORE` line without breaking immersion.

---

## 5. THE SHADOW: REFUSAL & BOOLEAN FORESHADOW

The moment the program evaluates as true (meaning the first 5 Bernoulli numerators are extracted from the 3 registers successfully), the cylinders drop to a cold standstill.

### 5.1 The Victory Whisper

The Mind Panel flushes out all engineering metrics to print Lovelace's tragic, historical monologue:
```text
[LOVELACE]: "...a perfect loop. The Bernoulli numbers unroll from three cylinders and seven lines. I was right. Yet they called me his 'interpreter' for a century after. I was thirty-six when the cancer finished what the laudanum began. None of this was supposed to matter, and it will matter forever."
```

### 5.2 Cross-Act Residue Deposition

An ASCII Inkwell & Folded Letter token drops onto the right boundary of The Chaotic Desk, incrementing the macro completion index. Focusing controls on this item triggers her shortwave-filtered voice over the metronome.

Concurrently, a cold, ironic voice bleed from George Boole's locked rain-damp page leaks from the next terminal corner, establishing the shift from hardware arithmetic to pure logical reduction:

```text
[BOOLE VOICE BLEED VIA COOPERATING THREAD]: "...two values. Every door open or closed. Every thought true or false..."
```
