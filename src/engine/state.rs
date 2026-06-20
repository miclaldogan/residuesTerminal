#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Act {
    Jacquard1804,
    Babbage1837,
    Lovelace1843,
    Boole1854,
    Shannon1937,
    // Final act — authored and routed, but not yet reachable in the current build.
    #[allow(dead_code)]
    Turing1936_1950,
}

// The deterioration phases are consumed by future Turing-act systems.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuringPhase {
    Clarity,  // 1936 - 1940 (Berrak Mantık)
    Tremor,   // 1950 (Fiziksel Titreme)
    Collapse, // 1951 (Kimyasal Çöküş)
}

/// The three top-level kinetic phases of the engine. The whole render/input
/// pipeline branches on this, gating the player through a deliberate progression:
/// black-void monologue → silent ambient study → live interactive puzzle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenState {
    /// Phase 0 — the candlelit main menu, drawn before any simulation runs. This is the
    /// default state on launch; the player crosses into the prelude or a resumed act
    /// from here.
    MainMenu,
    /// Phase 1 — pure #000000 canvas; the cinematic typewriter monologue alone.
    NarrativePrelude,
    /// Phase 2 — the full 3-panel split is visible, but [THE WORKSPACE] is a dark,
    /// dormant frame. The player studies the candle and papers in ambient silence.
    AmbientDesk,
    /// Phase 3 — [THE WORKSPACE] renders the live puzzle; the engine is interactive.
    ActivePuzzle,
    /// Cinematic — the historic figure's portrait + biography, shown when an act begins
    /// (New Game or after the prior act's outro). `act_id` is 1-based; `text_index`
    /// pages through the biography lines; `timer` counts frames since activation.
    ActIntro { act_id: u8, text_index: usize, timer: u64 },
    /// Cinematic — the tragic outcome of the act just completed, shown before the next
    /// act's intro. Same field semantics as [`ScreenState::ActIntro`].
    ActOutro { act_id: u8, text_index: usize, timer: u64 },
    /// The final black credits screen, reached after the Act VI bitten-apple finale.
    /// ENTER here returns to the main menu.
    FinalCredits,
}

impl ScreenState {
    /// True for any phase where the candlelit desk is on screen (phases 2 and 3).
    pub fn desk_visible(self) -> bool {
        matches!(self, ScreenState::AmbientDesk | ScreenState::ActivePuzzle)
    }
}

/// Tüm bulmaca ekranlarını ve arayüz manipülasyon vektörlerini
/// Arka planda asenkron olarak besleyen küresel durum bağlamı.
pub struct GlobalStateContext {
    pub current_act: Act,
    #[allow(dead_code)]
    pub active_turing_phase: TuringPhase,
    pub acts_completed: Vec<Act>,

    // Kinetik Sahne Durumu
    pub screen_state: ScreenState,   // Tam ekran daktilo mı, 3 panelli masa mı
    pub desk_reveal: f32,            // 0.0 → 1.0: masaya geçişte mum ışığının yumuşak açılışı
    
    // Kimyasal Zehirlenme ve Çürüme Metrikleri
    pub stilboestrol_ppm: f32,       // Klavye titremesini ve kararma hızını besler
    pub vision_blur_factor: f32,     // ANSI renk uzayını füme/griye bükme çarpanı
    pub apple_bites_taken: u8,       // Gizli ipucu mekanizması: 0 (bütün) - 4 (siluet core)
    pub candle_rows_remaining: u16,  // UI-less zamanlayıcı; mumun kalan satır yüksekliği
    
    // Görsel Konumlandırma Bayrakları
    pub chemical_drift_seed: u64,    // Hap karakterlerinin (o, .) masada rastgele dağılma seed'i
    pub lookup_active: bool,         // Workspace ile Masa yığını arasındaki fokus değişimi
    pub monologue_timer: u16,        // Zihin paneli monolog süresi
    
    // Asenkron Ses Hook Değişkenleri
    pub base_heartbeat_bpm: u32,     // Derinden gelen boğuk kalp atışının baz hızı
    pub arrhythmia_multiplier: f32,  // Faz 2/3'te kalp ritmine eklenecek düzensizlik sapması

    // ── Live vitals + Act VI decay, recomputed each tick and read by the renderers ──
    /// The displayed heartbeat BPM and the heart-icon pulse rate (single source of truth
    /// for the VITAL readout and the heartbeat metronome).
    pub current_bpm: u32,
    /// Act VI per-character log-corruption probability (0 elsewhere); rises with Turing's
    /// stabilised-residue progress and his panic spikes.
    pub turing_glitch_chance: f32,

    // Render Döngüsü Sayacı
    pub frame_counter: u64,          // Alev titreşimi ve animasyon fazı hesaplaması için kare sayacı

    // ── Akt-bağlamlı ilerleyen bozulma (progressive glitch) zamanlayıcısı ──
    pub act_elapsed_ticks: u64,      // Mevcut akt başladığından beri geçen tick sayısı
    timer_prev_act: Act,             // Akt değişimini yakalayıp zamanlayıcıyı sıfırlamak için
}

/// Engine tick cadence — the main loop ticks every 16ms (~62.5 fps); we use 62 so a
/// "second" of progressive decay is measured deterministically against `act_elapsed_ticks`.
const TICKS_PER_SEC: u64 = 62;
/// Full candle height in internal wax rows (the desk renderer maps this onto the visible
/// sub-cell wax shaft). The starting value for Act I and the divisor for every per-act cap.
pub const CANDLE_MAX_ROWS: u16 = 120;

impl GlobalStateContext {
    pub fn new() -> Self {
        Self {
            current_act: Act::Jacquard1804,
            active_turing_phase: TuringPhase::Clarity,
            acts_completed: Vec::new(),
            screen_state: ScreenState::MainMenu,
            desk_reveal: 0.0,
            stilboestrol_ppm: 0.0,
            vision_blur_factor: 0.0,
            apple_bites_taken: 0,
            candle_rows_remaining: CANDLE_MAX_ROWS, // Başlangıç mum dikey çözünürlüğü (satır sayısı)
            chemical_drift_seed: 42,
            lookup_active: false,
            monologue_timer: 0,
            base_heartbeat_bpm: 72,
            arrhythmia_multiplier: 0.0,
            current_bpm: 72,
            turing_glitch_chance: 0.0,
            frame_counter: 0,
            act_elapsed_ticks: 0,
            timer_prev_act: Act::Jacquard1804,
        }
    }

    /// Real-time seconds elapsed inside the current act (since the last act change).
    pub fn act_seconds(&self) -> u64 {
        self.act_elapsed_ticks / TICKS_PER_SEC
    }

    /// The master progressive-decay scalar, gated strictly by act context. This is the
    /// single source of truth for time-based corruption; each act reads it differently.
    ///
    ///   Act I  (Jacquard) → 0.0 always: absolute mechanical sanity.
    ///   Act II (Babbage)  → +1.0 every 30s: mind-log gear-alignment text errors.
    ///   Act III(Lovelace) → +1.0 every 45s: editor-line adjacent-swap hand tremor.
    ///   Others            → 0.0 (Boole/Shannon drive their own chemical systems).
    pub fn corruption_factor(&self) -> f32 {
        match self.current_act {
            Act::Jacquard1804 => 0.0,
            Act::Babbage1837 => self.act_seconds() as f32 / 30.0,
            Act::Lovelace1843 => self.act_seconds() as f32 / 45.0,
            _ => 0.0,
        }
    }

    /// Number of adjacent character swaps to bleed into the mind-log buffer (Act II
    /// only). Each 30-second stage adds one more gear-misalignment swap, capped so the
    /// log degrades but never becomes pure noise. Puzzle variables are never touched.
    pub fn mind_log_glitch_level(&self) -> usize {
        if self.current_act == Act::Babbage1837 {
            (self.corruption_factor().floor() as usize).min(8)
        } else {
            0
        }
    }

    /// Probability that committing an editor line transposes an adjacent character
    /// pair (Act III only) — the rising hand tremor. Ramps in 0.10 steps every 45s,
    /// capped at 0.6 so the player can always correct it with Backspace.
    pub fn lovelace_input_swap_chance(&self) -> f32 {
        if self.current_act == Act::Lovelace1843 {
            (self.corruption_factor().floor() * 0.10).clamp(0.0, 0.6)
        } else {
            0.0
        }
    }

    /// The candle's per-chapter starting height in wax rows (out of [`CANDLE_MAX_ROWS`]).
    /// The life-line gutters one notch per act — a full candle in Act I down to a ~5%
    /// critical stub by the Turing endgame. Set on every act entry, so the across-act
    /// decline is deterministic; the within-act time/fault melt then drains from there.
    pub fn candle_cap_rows(act: Act) -> u16 {
        match act {
            Act::Jacquard1804 => CANDLE_MAX_ROWS,           // 100%
            Act::Babbage1837 => CANDLE_MAX_ROWS * 80 / 100, // 80%
            Act::Lovelace1843 => CANDLE_MAX_ROWS * 60 / 100, // 60%
            Act::Boole1854 => CANDLE_MAX_ROWS * 40 / 100,   // 40%
            Act::Shannon1937 => CANDLE_MAX_ROWS * 20 / 100, // 20%
            Act::Turing1936_1950 => CANDLE_MAX_ROWS * 5 / 100, // 5% — critical flicker
        }
    }

    /// Zaman döngüsü veya hatalı derlemelerde mumu eriten fonksiyon
    pub fn melt_candle(&mut self, rows: u16) {
        let multiplier = if self.apple_bites_taken > 0 { 1.5 } else { 1.0 };
        let final_melt = (rows as f32 * multiplier) as u16;
        self.candle_rows_remaining = self.candle_rows_remaining.saturating_sub(final_melt);
    }

    /// Her render karesinde çağrılacak zaman tabanlı güncelleme fonksiyonu.
    /// Alev titreşimi, hap dağılımı ve metronom fazı bu sayaca bağlıdır.
    pub fn tick(&mut self) {
        self.frame_counter = self.frame_counter.wrapping_add(1);

        // ── Progressive-glitch timer: reset on act change, otherwise accumulate. ──
        if self.current_act != self.timer_prev_act {
            self.timer_prev_act = self.current_act;
            self.act_elapsed_ticks = 0;
            // Step the candle to this chapter's life-line ceiling on entry, so it visibly
            // gutters down across the six acts (full → ~5% critical by Act VI). The
            // within-act time/fault melt below then drains it further from this start.
            self.candle_rows_remaining = Self::candle_cap_rows(self.current_act);
        } else {
            self.act_elapsed_ticks = self.act_elapsed_ticks.saturating_add(1);
        }

        // ── Heartbeat recovery: ease the base rate back toward a resting 70 BPM at
        //    ~1 BPM/sec, so an interrogation spike (140) decays as the player calms
        //    instead of staying pinned forever. ──
        const RESTING_BPM: u32 = 70;
        if self.frame_counter % TICKS_PER_SEC == 0 && self.base_heartbeat_bpm > RESTING_BPM {
            self.base_heartbeat_bpm -= 1;
        }

        // Zamanla mumun erimesi (Stilboestrol ppm seviyesine göre hızlanır). One internal
        // wax row burns every `interval` frames; the wax shaft is ~120 rows mapped onto a
        // dozen visible cells, so these rates are tuned to read as a clearly-shortening
        // candle over tens of seconds (and to accelerate sharply under chemical load).
        let interval = if self.stilboestrol_ppm > 80.0 {
            45
        } else if self.stilboestrol_ppm > 50.0 {
            90
        } else if self.stilboestrol_ppm > 30.0 {
            160
        } else {
            240
        };

        if self.frame_counter % interval == 0 {
            self.melt_candle(1);
        }

        if self.monologue_timer > 0 {
            self.monologue_timer = self.monologue_timer.saturating_sub(1);
        }

        // Görme bulanıklığı bir darbe (impulse) olarak tetiklenir ve zamanla söner.
        // Jacquard'ın aşırı yüklü tezgâhı ya da kimyasal sersemlik bunu yükseltir;
        // her kare biraz daha berraklaşana dek titreşim devam eder.
        if self.vision_blur_factor > 0.0 {
            let decay = 0.012 + self.stilboestrol_ppm * 0.00005; // kimyasal yük sönümü yavaşlatır
            self.vision_blur_factor = (self.vision_blur_factor - decay).max(0.0);
        }

        // Masaya geçildiğinde (Faz 2 veya 3) mum ışığı 24-bit gradyanı yavaşça
        // karanlıktan açılır.
        if self.screen_state.desk_visible() && self.desk_reveal < 1.0 {
            self.desk_reveal = (self.desk_reveal + 0.02).min(1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn act_one_has_absolute_sanity() {
        let mut s = GlobalStateContext::new();
        s.current_act = Act::Jacquard1804;
        s.act_elapsed_ticks = TICKS_PER_SEC * 600; // ten minutes in
        assert_eq!(s.corruption_factor(), 0.0);
        assert_eq!(s.mind_log_glitch_level(), 0);
        assert_eq!(s.lovelace_input_swap_chance(), 0.0);
    }

    #[test]
    fn babbage_log_glitch_unlocks_every_30s() {
        let mut s = GlobalStateContext::new();
        s.current_act = Act::Babbage1837;
        s.act_elapsed_ticks = TICKS_PER_SEC * 29;
        assert_eq!(s.mind_log_glitch_level(), 0);
        s.act_elapsed_ticks = TICKS_PER_SEC * 60; // two full 30s stages
        assert_eq!(s.mind_log_glitch_level(), 2);
        assert_eq!(s.lovelace_input_swap_chance(), 0.0); // wrong act
    }

    #[test]
    fn lovelace_input_swap_ramps_every_45s_and_is_capped() {
        let mut s = GlobalStateContext::new();
        s.current_act = Act::Lovelace1843;
        s.act_elapsed_ticks = TICKS_PER_SEC * 44;
        assert_eq!(s.lovelace_input_swap_chance(), 0.0);
        s.act_elapsed_ticks = TICKS_PER_SEC * 90; // two stages → 0.20
        assert!((s.lovelace_input_swap_chance() - 0.20).abs() < 1e-6);
        s.act_elapsed_ticks = TICKS_PER_SEC * 10_000; // far in → clamped
        assert!(s.lovelace_input_swap_chance() <= 0.6);
        assert_eq!(s.mind_log_glitch_level(), 0); // wrong act
    }

    #[test]
    fn timer_resets_on_act_change() {
        let mut s = GlobalStateContext::new();
        s.tick();
        s.tick();
        assert_eq!(s.act_elapsed_ticks, 2);
        s.current_act = Act::Babbage1837;
        s.tick(); // change detected → reset to 0
        assert_eq!(s.act_elapsed_ticks, 0);
        s.tick();
        assert_eq!(s.act_elapsed_ticks, 1);
    }
}
