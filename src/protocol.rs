use serde::{Deserialize, Serialize};

pub const DEFAULT_HOST: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 7878;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccelerationMode {
    /// High phase stability, progressive linear acceleration, low beam divergence.
    LinearLinac,
    /// Magnetic synchrotron ring: balances Lorentz factor (γ) and collision luminosity.
    SynchrotronHarmonic,
    /// Laser plasma wakefield surfing: extreme TeV energy spikes with higher beam entropy.
    PlasmaWakefield,
    /// Quantum vacuum harmonic modulation: trades raw stability for quantum tunneling at high γ.
    QuantumResonance,
}

impl AccelerationMode {
    pub fn name(&self) -> &'static str {
        match self {
            Self::LinearLinac => "Stable RF Linac",
            Self::SynchrotronHarmonic => "Harmonic Synchrotron Ring",
            Self::PlasmaWakefield => "Laser Plasma Wakefield",
            Self::QuantumResonance => "Quantum Vacuum Resonance",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Self::LinearLinac => "RF Linac",
            Self::SynchrotronHarmonic => "Synchrotron",
            Self::PlasmaWakefield => "Plasma Wake",
            Self::QuantumResonance => "Quantum Res",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::LinearLinac => "High phase stability (98%), ideal for light particles & calibration.",
            Self::SynchrotronHarmonic => "Balanced magnetic ring: high luminosity & steady collision rate.",
            Self::PlasmaWakefield => "2.4x peak TeV energy multiplier with higher phase instability.",
            Self::QuantumResonance => "Maximizes quantum tunneling & exotic anomaly drops (2.85x at high γ).",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::LinearLinac => Self::SynchrotronHarmonic,
            Self::SynchrotronHarmonic => Self::PlasmaWakefield,
            Self::PlasmaWakefield => Self::QuantumResonance,
            Self::QuantumResonance => Self::LinearLinac,
        }
    }

    pub fn all() -> [Self; 4] {
        [
            Self::LinearLinac,
            Self::SynchrotronHarmonic,
            Self::PlasmaWakefield,
            Self::QuantumResonance,
        ]
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "linac" | "linear" | "1" => Some(Self::LinearLinac),
            "synchrotron" | "ring" | "2" => Some(Self::SynchrotronHarmonic),
            "plasma" | "wakefield" | "3" => Some(Self::PlasmaWakefield),
            "quantum" | "resonance" | "4" => Some(Self::QuantumResonance),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineCommand {
    GetSnapshot,
    SetThreads(usize),
    SetIntensity(u8),
    SetMode(AccelerationMode),
    CycleMode,
    AdjustMagnetFocus(f64),
    SetMagnetFocus(f64),
    AdjustRfHarmonic(f64),
    SetRfHarmonic(f64),
    LockOptimalHarmonic,
    TriggerPulseBurst,
    ResetState,
    Shutdown,
    Batch(Vec<EngineCommand>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryRecord {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub tier: u8,
    pub category: String,
    pub mass_gev: f64,
    pub best_sigma: f64,
    pub last_probability: f64,
    pub count: u64,
    pub first_discovered_at: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowerTrack {
    pub angle_rad: f64,
    pub curvature: f64,
    pub speed: f64,
    pub energy_ratio: f64,
    pub kind: TrackKind,
    /// Optional secondary decay vertex fraction (0.3..0.7 along track) for boson decay forks
    pub decay_fork_at: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackKind {
    HadronJet,
    MuonCurve,
    PhotonRay,
    QuantumAnomaly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollisionShower {
    pub seq_id: u64,
    pub energy_tev: f64,
    pub is_quantum_anomaly: bool,
    pub label: String,
    pub tracks: Vec<ShowerTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    pub uptime_secs: u64,
    // Hardware / CPU Compute
    pub active_threads: usize,
    pub max_threads: usize,
    pub intensity_pct: u8,
    pub system_cpu_pct: f32,
    pub memory_used_mb: u64,
    pub hashes_per_sec: f64,
    pub gflops: f64,
    pub total_compute_cycles: u64,

    // Accelerator Config ("How it accelerates")
    pub mode: AccelerationMode,
    pub magnet_focus: f64,
    pub rf_harmonic: f64,
    pub optimal_harmonic_hint: f64,
    pub harmonic_alignment_pct: f64,

    // Relativistic Kinematics & Collider Stats
    pub beta: f64,
    pub nines_of_c: f64,
    pub lorentz_gamma: f64,
    pub beam_energy_tev: f64,
    pub peak_energy_tev: f64,
    pub luminosity_fb_inv: f64,
    pub collisions_per_sec: f64,
    pub total_collisions: u64,
    pub beam_stability_pct: f64,
    pub capacitor_charge_pct: f64,
    pub burst_ticks_remaining: u32,

    // Quantum Discovery Progress
    pub quantum_proximity_pct: f64,
    pub quantum_drop_chance_per_sec: f64,
    pub next_target_name: String,
    pub next_target_symbol: String,
    pub next_target_min_tev: f64,
    pub next_target_sigma_progress: f64,
    pub discoveries: Vec<DiscoveryRecord>,
    pub recent_logs: Vec<String>,
    pub latest_shower: Option<CollisionShower>,
}
