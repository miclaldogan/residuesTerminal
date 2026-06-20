use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::state::Act;

// ─────────────────────────────────────────────────────────────────────────────
// Lightweight checkpoint persistence.
//
// The game is a linear act-by-act marathon; this writes a tiny JSON file each time
// an act is cleared so a returning player resumes at the act they reached instead of
// replaying every solved puzzle. Deliberately minimal — only the progress vector and
// the active act — and every fs/serde operation is best-effort: a missing, unreadable
// or malformed save simply yields a fresh start, never a crash.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveState {
    pub current_act: Act,
    pub acts_completed: Vec<Act>,
}

/// The checkpoint file, placed next to the executable so it travels with the build and
/// is independent of the working directory.
fn save_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            return dir.join("residues_save.json");
        }
    }
    PathBuf::from("residues_save.json")
}

/// Load the checkpoint, or `None` if there is no valid save (fresh start).
pub fn load() -> Option<SaveState> {
    let data = std::fs::read_to_string(save_path()).ok()?;
    serde_json::from_str(&data).ok()
}

/// Persist the checkpoint. Best-effort: any IO/serialisation error is silently ignored
/// so a read-only disk can never take down the game.
pub fn save(state: &SaveState) {
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(save_path(), json);
    }
}

/// Erase the checkpoint entirely (the "Erase Memory Core" restart). Best-effort: a
/// missing file or unremovable path is ignored, so a fresh game always proceeds.
pub fn clear() {
    let _ = std::fs::remove_file(save_path());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_state_round_trips_through_json() {
        let s = SaveState {
            current_act: Act::Boole1854,
            acts_completed: vec![Act::Jacquard1804, Act::Babbage1837, Act::Lovelace1843],
        };
        let json = serde_json::to_string(&s).expect("serialise");
        let back: SaveState = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(back.current_act, s.current_act);
        assert_eq!(back.acts_completed, s.acts_completed);
    }

    #[test]
    fn malformed_save_does_not_panic() {
        // Garbage in → None out (fresh start), never a crash.
        let bad: Result<SaveState, _> = serde_json::from_str("{ not json");
        assert!(bad.is_err());
    }
}
