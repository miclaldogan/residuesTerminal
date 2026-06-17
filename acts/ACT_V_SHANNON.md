# ⚙️ ACT V SPECIFICATION: SHANNON (1937) — LOGIC AS CIRCUIT

## 1. THE ACTIVE PAPER: VISUAL SKIN & PORTRAIT

When Act V triggers, the engine shifts `GlobalStateContext.current_act = Act::Shannon1937`. The Chaotic Desk pipeline draws Claude Shannon’s specific document at the absolute peak of the paper stack (Z-index: 0), pushing George Boole's refactored text page down into the dimmed lower shadow stacks.

* **The 1-Bit Portrait:** Rendered via custom high-contrast character dithering (`█`, `▓`, `▒`, `░`), portraying Claude Shannon as young, inventive, and playful.
* **The Verification Signature:** Synthesized seamlessly at the base of the sheet using fine, precise cursive ASCII line elements:
  ```text
  Hand-signed: Claude E. Shannon
  ```
* **The Static Workspace:** The background text layer defines the physical paradigm shift: *Logic leaves the page and enters physics. Boole's two values are physical — a switch open or shut, one or zero.*

---

## 2. THE 2-STATE CIRCUIT FUNNEL (PHYSICS VS. LOGIC)

The circuit workspace forces the technical user to wrestle with the raw constraints of physical electrical delays and hardware paths before unlocking clean logic synchronization.

```text
  [STATE #10: FAULTY RELAY NET]        [STATE #11: THE HALF-ADDER]
     ┌───┐       ┌───┐                     ┌───┐       ┌───┐
  ───┤   ├───────┤   ├───               ───┤   ├───────┤   ├───
     └───┘       └───┘                     └───┘   ▲   └───┘
       ▲ (Race Condition)                          │ (Delay Synced)
     [SHORT_CIRCUIT / SPARKS]                 [RHYTHMIC_CARRY_PULSE]
```

### State 10: Faulty Relay Net (The Physical Overrun)

* **The Interface:** An interactive schematic canvas where logic nodes (`[AND]`, `[OR]`, `[NOT]`) are wired using manual copper path strings.
* **The Failure Mode:** The user must assemble an electric Half-Adder to unite Boole's algebra with Babbage's carry mechanism. If the wiring paths ignore real-world physics—routing signals through improper series or parallel loops that neglect switching lag—current values misalign. The system triggers an immediate layout exception: `[SHORT_CIRCUIT]: Voltage sag and race condition detected across parallel paths.`
* **Biting Scolding (Tier 1):**
  ```text
  [SHANNON VIA MIND_LOG]: "Your logic may be perfect, but your physics is a disaster. You burned the circuit."
  ```

### State 11: The Half-Adder (The Coordinated Pulse)

* **The Optimization Condition:** Unlocked once the series/parallel wires are accurately routed and delayed. Current pulses rhythmically through the designated carry paths without blowing out the layout.

---

## 3. CORE MECHANIC: THE ASYNCHRONOUS KABUS (RACE CONDITIONS)

The circuit puzzle is designed to punish flat logical thinking and reward meticulous timing management.

### 3.1 The Switch Toggle Input

The player navigates across copper nodes and relays using direct keyboard shortcuts, toggling binary gate states from open to shut (`shannon_toggle()`).

### 3.2 The Microsecond Timing Defeat

Relays inside the engine do not transition instantaneously; they have explicit, immutable tick-delays built into their logic evaluation states.
* If the user triggers two input switches (`a` and `b`) simultaneously without balancing the path lengths, the electrical signals arrive out of phase.
* This causes a devastating **race condition**. Voltage sags, and the console workspace triggers a localized `[SHORT_CIRCUIT]` explosion of random symbols across the grid rows.
* To win, the user must arrange the wiring arrays so that the carry paths settle cleanly over synchronized clock cycle thresholds.

---

## 4. TUI RENDER LAYOUT & LIVE VOLTAGE PULSING

The interactive workspace within `[THE WORKSPACE]` renders the relay grid using automated, true-color ANSI voltage propagation.

```text
 ┌─ [THE WORKSPACE: ACT V RELAY SCHEMATIC] ────────────────────────────────────────────┐
 │  [ CIRCUIT CURRENT: ACTIVE ]                 [ SYSTEM POWER LOAD: stable ]          │
 │                                                                                     │
 │  LIVE COPPER ROUTING MATRIX:                                                        │
 │  (INPUT_A) ──█▓▒░ [AND] ───────┐                                                    │
 │               \                ├─── \x1b[32m█████\x1b[0m [SUM_OUT] -> [ 1 ]             │
 │  (INPUT_B) ───┴── [OR]  ───────┘                                                    │
 │               \                                                                     │
 │                └── [RELAY_CARRY] ───► [CARRY_OUT] -> [ 0 ]                           │
 │                                                                                     │
 │  [ TIMING SPECTRUM ]:                                                               │
 │  Relay A Delay: 12ms | Relay B Delay: 04ms | Phase Drift Offset: CRITICAL RACE      │
 └─────────────────────────────────────────────────────────────────────────────────────┘
```

### 4.1 Frame Execution & Flow Highlighting

* **Live Current Highlighting:** Electrical path conduction evaluates dynamically frame-by-frame from numerical relay states. When a wire segment is active, the rendering thread injects raw green true-color headers (`\x1b[38;2;51;255;51m`) and active block indicators (`█`) directly across the coordinate cell rows, causing live, glowing energy pulses to race down the schematic strings.
* **Pre-Short Warnings:** One tick before a race condition explodes the board, the specific logic gate where the signals collide will flare bright white and flash erratically, serving as a silent, immersive indicator of the precise timing fault.

---

## 5. THE TIME-CLASH & THE HAUNTING DEBRIS (THE SHADOW)

As the current coordinates stabilize and the half-adder pulses rhythmically, a severe narrative distortion takes hold of the system variables.

### 5.1 The Forking Monologue

The global `vision_blur_factor` surges. The light flickers, and 1954 Turing’s tired inner narrator voice drops over Shannon's youthful circuit prose like a shadow, highlighting their tragic asynchronous convergence:
```text
[TURING'S INNER VOICE]: "The same span of years… you joining relays in America while I imagined an endless tape in Cambridge. We were building the same mind, Claude. Why could we never hear each other?"
```

### 5.2 The Victory Whisper

Upon complete resolution, the metrics freeze, and the Mind Panel logs the celebratory milestone:
```text
[SHANNON]: "The switch closes. The current chooses. Babbage's carry, now electric. Boole's algebra, now copper. It all fits."
```

### 5.3 The Alzheimer's Collapse (The Softest Gut-Punch)

An ASCII Relay token drops permanently onto the right boundary of The Chaotic Desk, incrementing `GlobalStateContext.acts_completed` to 5. Inspecting this item drops the absolute psychological hammer: Shannon's youthful, playful voice suddenly transforms over the shortwave filters into an aged, trembling, word-losing whisper (Alzheimer's):
```text
[SHANNON VIA AGED RESIDUE]: "Open or closed… one or zero… relays… what were relays for?"
```
Followed immediately by Turing's clinical, agonizing realization:
```text
[TURING]: "The man who built the information age could not remember the world he made. And I — I remember everything too well."
```

---

*Act V is finished, validated, and tightly locked into the system specification matrices. All five seals are broken, and the ghost jury is now fully assembled in the dark corners of the screen, staring directly into the camera.*
