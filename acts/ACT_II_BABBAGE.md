# ⚙️ ACT II SPECIFICATION: BABBAGE (1837) — THE DIFFERENCE ENGINE

## 1. THE ACTIVE PAPER: VISUAL SKIN & PORTRAIT

When Act II initializes, the engine updates `GlobalStateContext.current_act = Act::Babbage1837`. The Chaotic Desk rendering stack pushes Charles Babbage’s specific document to the absolute foreground (Z-index: 0), shifting Jacquard's card stack downward into the dimmed background shadow stack.

* **The 1-Bit Portrait:** Formed via a precise, dithered ANSI block matrix portraying Charles Babbage as old, bitter, and clipped.
* **The Verification Signature:** Drawn at the bottom boundary of the text layer using continuous, rigid script strokes:
  ```text
  Hand-signed: Charles Babbage
  ```
* **The Static Workspace:** The background text layers establish the ideological pivot: *A process repeated mechanically can tabulate any polynomial. The machine only adds, but its memory must carry the wave.*

---

## 2. THE 2-PHASE METHOD OF DIFFERENCES & THE CARRY CRISIS

The act splits the computation workspace into two distinct engineering phases, forcing the user to bridge pure mathematics with raw physical hardware limitations.

### Phase 1: The Method of Differences (Mathematical Reduction)

The workspace drops a numerical sequence problem that is historically impossible to reliably compute by hand without errors sinking ships (e.g., logarithmic navigation tables). The user must compute the differences for a polynomial sequence ($n^2$: 1, 4, 9, 16, 25) to find the absolute constant row ($\Delta^2 = 2$).
* By working backward, the player realizes that multiplication and squaring operations can be completely bypassed by chained addition loops.
* Once the columns are filled with the baseline vector `[1, 3, 2]` (representing `[value, Δ¹, Δ²]`), Phase 2 initializes.

### Phase 2: The Hardware Bug (Carry Propagation Failure)

The moment the mathematical totals are stepped forward by executing the manual engine crank (`babbage_crank()`), the architecture hits a critical mechanical barrier:

```text
  [STATE #4: NAÏVE RIPPLE CARRY]       [STATE #5: CASCADING WAVE]
         ┌───┐ ┌───┐ ┌───┐                 ┌───┐   ┌───┐   ┌───┐
         │ 9 │ │ 9 │ │ 9 │                 │ 9 │──>│ 9 │──>│ 9 │
         └───┘ └───┘ └───┘                 └───┘   └───┘   └───┘
     [GEAR_LOCK_VAL_OVERFLOW]                [ t0 ]  [ t1 ]  [ t2 ]
```

* **State 4: Naïve Ripple Carry (The Failure Condition):** When column indices cross from 9 → 0, they attempt to propagate a +1 carry value to the adjacent column simultaneously. If the carry buffers are uninstalled (`!puzzle.carry_buffers[0]`), the massive physical force required to rotate every column wheel at once causes a structural failure. The console workspace changes its font color to a blinding crimson and logs a fatal crash:
  ```text
  [FATAL ERROR]: Carry propagation failed — all gears lock at once. [GEAR_LOCK_VAL_OVERFLOW]
  ```
* **Biting Scolding (Tier 2):**
  ```text
  [BABBAGE VIA MIND_LOG]: "Can you not see to push the neighbouring gear when you pass nine? You'll sink the navy with this trash."
  ```

---

## 3. THE FINAL ARCHITECTURAL OPTIMIZATION (STATE 5)

To pass the lock, the player must alter the physical hardware array by calling `babbage_install_carry()`.

* **The Delay Solution:** The user inserts mechanical delay/buffer objects (`[D]`) between adjacent columns.
* **The Mechanical Wave:** This prevents simultaneous execution. The carry is forced to progress sequentially, step-by-step, across independent time steps (`t0`, `t1`, `t2`), converting a catastrophic mechanical spike into a smooth, cascading carry wave that ripples elegantly through the character matrix.

---

## 4. TUI RENDER LAYOUT & METRIC GEAR MOTION

The active interface cell within `[THE WORKSPACE]` renders the multi-column difference calculator using dynamic, rotating character structures.

```text
 ┌─ [THE WORKSPACE: ACT II DIFFERENCE ENGINE] ─────────────────────────────────────────┐
 │  [ GEAR ENGINE STATE: CRANK ACTIVE ]         [ HARDWARE TIMING: STAGGERED ]         │
 │                                                                                     │
 │    COLS:     [ VALUE ]       [ DELTA 1 ]     [ DELTA 2 ]                            │
 │    AXIS:        (│)             (/)             (─)      <- Rotating ASCII          │
 │   ┌───┐        ┌───┐           ┌───┐           ┌───┐        Gears                   │
 │   │004│        │009│           │005│           │002│                                │
 │   └───┘        └───┘           └───┘           └───┘                                │
 │    [D0] [✓]     [D1] [✓]        [D2] [✓]                 <- Carry Delay Buffers     │
 │                                                                                     │
 │  [ MECHANICAL STREAM ]:                                                             │
 │  Crank Position Factor: 0.25 / Cycle Phase: Forward Addition Step                   │
 └─────────────────────────────────────────────────────────────────────────────────────┘
```

### 4.1 Frame Execution & Rotational Dynamics

* **ASCII Gear Rotation:** The current numeric values inside `puzzle.columns` are structurally linked to rotating string wheels. As the crank advances, the lines directly alter their angles sequentially based on the current calculation phase (`│` → `/` → `─` → `\`), generating fluid, calculated mechanical velocity entirely via text interpolation.
* **Live Delay Diagnostics:** Beneath each column, the current status of the sequencing blocks is mapped out (`[D0] [✓]`). If a delay buffer is missing or deactivated, a flashing `[D_ERR]` icon alerts the user.

### 4.2 Error-As-Collapse Dynamics

When a gear lock occurs, the engine triggers a deep bass metal grind audio effect, and the entire workspace panel shudders violently via a calculated horizontal shift offset tied to `GlobalStateContext.stilboestrol_ppm`. The column where the carry stalled begins to bleed random hexadecimal and corrupt character artifacts down its column axis first, serving as a silent indicator of the precise structural flaw.

---

## 5. NARRATIVE RECOVERY & LOVELACE VOICE BLEED

The moment `babbage_is_solved()` evaluates as true (meaning all sequencing delays are installed and the columns output the terminal target sequence flawlessly), the system locks down the computational gears. The mechanical rhythm drops to silence.

### 5.1 The Victory Whisper

The Mind Panel updates its text logs to clear all engineering parameters, surfacing Babbage's bitter, historical monologue:
```text
[BABBAGE]: "The columns sing. Clean tables. At last. But the government has cut our funding. A hundred years... this world is a hundred years too early to understand this engine. I die with nothing but the blueprints."
```

### 5.2 Cross-Act Residue Allocation

A solid, heavy ASCII Brass Gear token is deposited onto the right side of The Chaotic Desk, incrementing `GlobalStateContext.acts_completed`. If the player focuses their selection controls to inspect this token, Babbage's sharp, age-matched voice triggers via the high-pass static radio filters.

Concurrently, a faint, rhythmic voice bleed from Ada Lovelace's locked paper stack leaks from the adjacent quadrant of the terminal screen, establishing the conceptual bridge to the next system architecture:

```text
[LOVELACE VOICE BLEED VIA COOPERATING THREAD]: "...she saw what I did not. The cards could decide, not merely record..."
```
