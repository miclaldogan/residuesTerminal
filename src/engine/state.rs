#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// Phase 1 — pure #000000 canvas; the cinematic typewriter monologue alone.
    NarrativePrelude,
    /// Phase 2 — the full 3-panel split is visible, but [THE WORKSPACE] is a dark,
    /// dormant frame. The player studies the candle and papers in ambient silence.
    AmbientDesk,
    /// Phase 3 — [THE WORKSPACE] renders the live puzzle; the engine is interactive.
    ActivePuzzle,
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

    // Render Döngüsü Sayacı
    pub frame_counter: u64,          // Alev titreşimi ve animasyon fazı hesaplaması için kare sayacı
}

impl GlobalStateContext {
    pub fn new() -> Self {
        Self {
            current_act: Act::Jacquard1804,
            active_turing_phase: TuringPhase::Clarity,
            acts_completed: Vec::new(),
            screen_state: ScreenState::NarrativePrelude,
            desk_reveal: 0.0,
            stilboestrol_ppm: 0.0,
            vision_blur_factor: 0.0,
            apple_bites_taken: 0,
            candle_rows_remaining: 120, // Başlangıç mum dikey çözünürlüğü (satır sayısı)
            chemical_drift_seed: 42,
            lookup_active: false,
            monologue_timer: 0,
            base_heartbeat_bpm: 72,
            arrhythmia_multiplier: 0.0,
            frame_counter: 0,
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

        // Zamanla mumun erimesi (Stilboestrol ppm seviyesine göre hızlanır)
        let interval = if self.stilboestrol_ppm > 80.0 {
            200
        } else if self.stilboestrol_ppm > 50.0 {
            500
        } else if self.stilboestrol_ppm > 30.0 {
            1000
        } else {
            3000
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
