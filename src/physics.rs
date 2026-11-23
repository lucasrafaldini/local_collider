use crate::protocol::{
    AccelerationMode, CollisionShower, DiscoveryRecord, ShowerTrack, TelemetrySnapshot, TrackKind,
};
use chrono::Local;
use rand::prelude::*;
use rand::rngs::StdRng;
use rand_distr::{Normal, Poisson};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const PROTON_MASS_TEV: f64 = 0.000_938_272;
pub const STATE_FILE: &str = "collider_state.json";

#[derive(Debug, Clone)]
pub struct QuantumTargetSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub symbol: &'static str,
    pub tier: u8,
    pub category: &'static str,
    pub mass_gev: f64,
    pub min_energy_tev: f64,
    pub base_drop_prob_per_collision: f64,
    pub preferred_mode: AccelerationMode,
    pub resonant_harmonic: f64,
    pub description: &'static str,
}

pub const QUANTUM_CATALOG: &[QuantumTargetSpec] = &[
    QuantumTargetSpec {
        id: "muon_pair",
        name: "Relativistic Muon Pair",
        symbol: "μ⁺μ⁻",
        tier: 1,
        category: "Standard Model (Leptons)",
        mass_gev: 0.211,
        min_energy_tev: 0.12,
        base_drop_prob_per_collision: 1.2e-4,
        preferred_mode: AccelerationMode::LinearLinac,
        resonant_harmonic: 1.0,
        description: "Second-generation heavy leptons exhibiting measurable relativistic time dilation.",
    },
    QuantumTargetSpec {
        id: "jpsi_meson",
        name: "J/ψ Charmonium Meson",
        symbol: "J/ψ",
        tier: 1,
        category: "Standard Model (Hadrons)",
        mass_gev: 3.0969,
        min_energy_tev: 0.45,
        base_drop_prob_per_collision: 5.5e-5,
        preferred_mode: AccelerationMode::LinearLinac,
        resonant_harmonic: 1.3,
        description: "Bound charm-anticharm quark state that confirmed the quark model of matter.",
    },
    QuantumTargetSpec {
        id: "z_boson",
        name: "Z⁰ Electroweak Vector Boson",
        symbol: "Z⁰",
        tier: 2,
        category: "Electroweak Gauge Boson",
        mass_gev: 91.1876,
        min_energy_tev: 1.25,
        base_drop_prob_per_collision: 2.2e-5,
        preferred_mode: AccelerationMode::SynchrotronHarmonic,
        resonant_harmonic: 1.7,
        description: "Neutral carrier of the weak nuclear force observed in high-luminosity collisions.",
    },
    QuantumTargetSpec {
        id: "higgs_boson",
        name: "Scalar Higgs Boson",
        symbol: "H⁰",
        tier: 2,
        category: "Spontaneous Symmetry Breaking",
        mass_gev: 125.25,
        min_energy_tev: 2.90,
        base_drop_prob_per_collision: 9.0e-6,
        preferred_mode: AccelerationMode::SynchrotronHarmonic,
        resonant_harmonic: 2.1,
        description: "Quantum excitation of the Higgs field responsible for elementary particle mass.",
    },
    QuantumTargetSpec {
        id: "tetraquark_tcc",
        name: "Double-Charm Tetraquark",
        symbol: "Tcc⁺",
        tier: 3,
        category: "Exotic Multiquark Hadron",
        mass_gev: 3.875,
        min_energy_tev: 6.20,
        base_drop_prob_per_collision: 3.8e-6,
        preferred_mode: AccelerationMode::SynchrotronHarmonic,
        resonant_harmonic: 2.4,
        description: "Long-lived exotic four-quark bound quantum state (ccūd̄).",
    },
    QuantumTargetSpec {
        id: "pentaquark_pc",
        name: "Charmonium Pentaquark",
        symbol: "Pc(4457)⁺",
        tier: 3,
        category: "Exotic Multiquark Hadron",
        mass_gev: 4.457,
        min_energy_tev: 11.5,
        base_drop_prob_per_collision: 1.6e-6,
        preferred_mode: AccelerationMode::PlasmaWakefield,
        resonant_harmonic: 2.8,
        description: "Exotic baryon composed of four quarks and one antiquark in strong QCD coupling.",
    },
    QuantumTargetSpec {
        id: "susy_neutralino",
        name: "Supersymmetric Neutralino",
        symbol: "χ̃₁⁰",
        tier: 4,
        category: "Supersymmetry (Dark Matter)",
        mass_gev: 480.0,
        min_energy_tev: 21.0,
        base_drop_prob_per_collision: 7.2e-7,
        preferred_mode: AccelerationMode::PlasmaWakefield,
        resonant_harmonic: 3.2,
        description: "Lightest stable supersymmetric fermion and prime cold Dark Matter candidate.",
    },
    QuantumTargetSpec {
        id: "leptoquark_lq3",
        name: "3rd-Gen Vector Leptoquark",
        symbol: "LQ₃",
        tier: 4,
        category: "Grand Unification (Beyond SM)",
        mass_gev: 1450.0,
        min_energy_tev: 38.0,
        base_drop_prob_per_collision: 3.1e-7,
        preferred_mode: AccelerationMode::PlasmaWakefield,
        resonant_harmonic: 3.6,
        description: "Hypothetical gauge boson coupling quarks directly to leptons across generations.",
    },
    QuantumTargetSpec {
        id: "kk_graviton",
        name: "Kaluza-Klein Graviton",
        symbol: "G_KK",
        tier: 5,
        category: "Quantum Gravity / Extra Dimensions",
        mass_gev: 2900.0,
        min_energy_tev: 68.0,
        base_drop_prob_per_collision: 1.4e-7,
        preferred_mode: AccelerationMode::QuantumResonance,
        resonant_harmonic: 4.0,
        description: "Excited graviton resonance propagating through compactified extra spatial dimensions.",
    },
    QuantumTargetSpec {
        id: "dirac_monopole",
        name: "Dirac Magnetic Monopole",
        symbol: "g_D",
        tier: 5,
        category: "Quantum Gauge Topology",
        mass_gev: 4600.0,
        min_energy_tev: 115.0,
        base_drop_prob_per_collision: 6.0e-8,
        preferred_mode: AccelerationMode::QuantumResonance,
        resonant_harmonic: 4.3,
        description: "Topological gauge soliton carrying isolated magnetic charge that quantizes electric charge.",
    },
    QuantumTargetSpec {
        id: "quantum_foam_condensate",
        name: "Quantum Foam Condensate",
        symbol: "Ψ_foam",
        tier: 6,
        category: "Planck-Scale Spacetime Anomaly",
        mass_gev: 8800.0,
        min_energy_tev: 185.0,
        base_drop_prob_per_collision: 2.5e-8,
        preferred_mode: AccelerationMode::QuantumResonance,
        resonant_harmonic: 4.7,
        description: "Coherent spacetime metric fluctuation undergoing macroscopic quantum tunneling.",
    },
    QuantumTargetSpec {
        id: "brane_singularity",
        name: "Brane Tunneling Singularity",
        symbol: "Ω_brane",
        tier: 6,
        category: "Deep Multiverse Singularity",
        mass_gev: 14000.0,
        min_energy_tev: 300.0,
        base_drop_prob_per_collision: 1.0e-8,
        preferred_mode: AccelerationMode::QuantumResonance,
        resonant_harmonic: 4.9,
        description: "Localized quantum vacuum rupture exchanging momentum across the hyperdimensional bulk.",
    },
];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistentColliderSave {
    pub total_collisions: u64,
    pub luminosity_fb_inv: f64,
    pub accumulated_ring_momentum_tev: f64,
    pub peak_energy_tev: f64,
    pub discoveries: Vec<DiscoveryRecord>,
    pub target_sigma_accum: Vec<(String, f64)>,
}

pub struct AcceleratorPhysicsState {
    pub mode: AccelerationMode,
    pub magnet_focus: f64,
    pub rf_harmonic: f64,
    pub accumulated_ring_momentum_tev: f64,
    pub beam_energy_tev: f64,
    pub peak_energy_tev: f64,
    pub luminosity_fb_inv: f64,
    pub total_collisions: u64,
    pub capacitor_charge_pct: f64,
    pub burst_ticks_remaining: u32,
    pub discoveries: Vec<DiscoveryRecord>,
    pub target_sigma_accum: Vec<f64>,
    pub recent_logs: Vec<String>,
    pub latest_shower: Option<CollisionShower>,
    pub shower_seq: u64,
    rng: StdRng,
}

impl AcceleratorPhysicsState {
    pub fn load_or_new() -> Self {
        let mut state = Self {
            mode: AccelerationMode::SynchrotronHarmonic,
            magnet_focus: 1.25,
            rf_harmonic: 1.70,
            accumulated_ring_momentum_tev: 0.05,
            beam_energy_tev: 0.10,
            peak_energy_tev: 0.10,
            luminosity_fb_inv: 0.0,
            total_collisions: 0,
            capacitor_charge_pct: 25.0,
            burst_ticks_remaining: 0,
            discoveries: Vec::new(),
            target_sigma_accum: vec![0.0; QUANTUM_CATALOG.len()],
            recent_logs: Vec::new(),
            latest_shower: None,
            shower_seq: 0,
            rng: StdRng::from_entropy(),
        };

        if Path::new(STATE_FILE).exists() {
            if let Ok(raw) = fs::read_to_string(STATE_FILE) {
                if let Ok(mut saved) = serde_json::from_str::<PersistentColliderSave>(&raw) {
                    // Normalize discovery names/descriptions to current English catalog
                    for rec in &mut saved.discoveries {
                        if let Some(spec) = QUANTUM_CATALOG.iter().find(|s| s.id == rec.id) {
                            rec.name = spec.name.to_string();
                            rec.category = spec.category.to_string();
                            rec.description = spec.description.to_string();
                        }
                    }
                    state.total_collisions = saved.total_collisions;
                    state.luminosity_fb_inv = saved.luminosity_fb_inv;
                    state.accumulated_ring_momentum_tev =
                        saved.accumulated_ring_momentum_tev.max(0.05);
                    state.peak_energy_tev = saved.peak_energy_tev;
                    state.discoveries = saved.discoveries;
                    for (id, sig) in saved.target_sigma_accum {
                        if let Some(idx) = QUANTUM_CATALOG.iter().position(|t| t.id == id) {
                            state.target_sigma_accum[idx] = sig;
                        }
                    }
                    state.push_log(format!(
                        "Restored collider state: {} quantum discoveries, {:.2} TeV peak energy.",
                        state.discoveries.len(),
                        state.peak_energy_tev
                    ));
                    return state;
                }
            }
        }

        state.push_log(
            "Collider ring initialized. Injecting proton bunches into superconducting RF cavities..."
                .into(),
        );
        state
    }

    pub fn save_to_disk(&self) {
        let save = PersistentColliderSave {
            total_collisions: self.total_collisions,
            luminosity_fb_inv: self.luminosity_fb_inv,
            accumulated_ring_momentum_tev: self.accumulated_ring_momentum_tev,
            peak_energy_tev: self.peak_energy_tev,
            discoveries: self.discoveries.clone(),
            target_sigma_accum: QUANTUM_CATALOG
                .iter()
                .enumerate()
                .map(|(i, t)| (t.id.to_string(), self.target_sigma_accum[i]))
                .collect(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&save) {
            let _ = fs::write(STATE_FILE, json);
        }
    }

    pub fn reset(&mut self) {
        self.accumulated_ring_momentum_tev = 0.05;
        self.beam_energy_tev = 0.10;
        self.peak_energy_tev = 0.10;
        self.luminosity_fb_inv = 0.0;
        self.total_collisions = 0;
        self.capacitor_charge_pct = 0.0;
        self.burst_ticks_remaining = 0;
        self.discoveries.clear();
        self.target_sigma_accum = vec![0.0; QUANTUM_CATALOG.len()];
        self.recent_logs.clear();
        self.push_log("Collider state reset to zero calibration.".into());
        self.save_to_disk();
    }

    pub fn push_log(&mut self, msg: String) {
        let stamp = Local::now().format("%H:%M:%S");
        self.recent_logs.push(format!("[{}] {}", stamp, msg));
        if self.recent_logs.len() > 40 {
            self.recent_logs.remove(0);
        }
    }

    pub fn lock_optimal_harmonic(&mut self) {
        let idx = self.next_target_index();
        let target_harm = QUANTUM_CATALOG[idx].resonant_harmonic;
        self.rf_harmonic = target_harm;
        self.push_log(format!(
            "Auto-locked RF cavity harmonic to {:.2} GHz (resonant with {}).",
            target_harm, QUANTUM_CATALOG[idx].symbol
        ));
    }

    pub fn trigger_burst(&mut self) -> bool {
        if self.capacitor_charge_pct >= 25.0 {
            let bonus_ticks = ((self.capacitor_charge_pct / 100.0) * 90.0).round() as u32;
            self.burst_ticks_remaining += bonus_ticks.max(25);
            self.push_log(format!(
                "SUPERCRITICAL PULSE FIRED! Discharged {:.0}% capacitor (+{:.1}s quantum overdrive).",
                self.capacitor_charge_pct,
                bonus_ticks as f64 * 0.1
            ));
            self.capacitor_charge_pct = 0.0;
            true
        } else {
            self.push_log(format!(
                "Capacitor at {:.1}% (minimum 25% required for supercritical pulse).",
                self.capacitor_charge_pct
            ));
            false
        }
    }

    pub fn next_target_index(&self) -> usize {
        for (idx, spec) in QUANTUM_CATALOG.iter().enumerate() {
            if !self.discoveries.iter().any(|d| d.id == spec.id) {
                return idx;
            }
        }
        QUANTUM_CATALOG.len() - 1
    }

    /// Advances one physics simulation step driven by real CPU benchmark metrics.
    pub fn step(
        &mut self,
        dt_secs: f64,
        hashes_per_sec: f64,
        gflops: f64,
        cpu_entropy: u64,
        uptime_secs: u64,
        active_threads: usize,
        max_threads: usize,
        intensity_pct: u8,
        system_cpu_pct: f32,
        memory_used_mb: u64,
        total_compute_cycles: u64,
    ) -> TelemetrySnapshot {
        let mut seed_bytes = [0u8; 32];
        for (i, chunk) in seed_bytes.chunks_exact_mut(8).enumerate() {
            let mixed = cpu_entropy
                .wrapping_add((i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
                ^ self.total_collisions.rotate_left((i * 13) as u32)
                ^ hashes_per_sec.to_bits();
            chunk.copy_from_slice(&mixed.to_le_bytes());
        }
        self.rng = StdRng::from_seed(seed_bytes);

        let target_idx = self.next_target_index();
        let target_spec = &QUANTUM_CATALOG[target_idx];
        let optimal_harmonic = target_spec.resonant_harmonic;

        let harmonic_diff = (self.rf_harmonic - optimal_harmonic).abs();
        let harmonic_alignment_pct =
            ((-harmonic_diff * harmonic_diff / 0.45).exp() * 100.0).clamp(5.0, 100.0);
        let harmonic_resonance_boost = 0.75 + 1.45 * (harmonic_alignment_pct / 100.0);

        let (mode_energy_mult, mode_luminosity_mult, mode_quantum_mult, base_stability) =
            match self.mode {
                AccelerationMode::LinearLinac => (1.00, 1.05, 1.00, 98.0),
                AccelerationMode::SynchrotronHarmonic => (1.45, 1.65, 1.35, 92.0),
                AccelerationMode::PlasmaWakefield => (2.40, 1.20, 1.90, 78.0),
                AccelerationMode::QuantumResonance => (1.80, 1.30, 2.85, 84.0),
            };

        let focus_penalty = ((self.magnet_focus - 1.2).max(0.0) * 14.0)
            * (1.0 - (harmonic_alignment_pct / 130.0));
        let beam_stability_pct =
            (base_stability - focus_penalty + (harmonic_alignment_pct * 0.08)).clamp(25.0, 99.9);

        let burst_mult = if self.burst_ticks_remaining > 0 {
            self.burst_ticks_remaining -= 1;
            3.25
        } else {
            1.0
        };

        let raw_compute_score = (hashes_per_sec / 120_000.0) + (gflops * 3.5);
        let compute_power = raw_compute_score.max(0.08).powf(0.72);

        let momentum_gain = compute_power
            * mode_energy_mult
            * harmonic_resonance_boost
            * (beam_stability_pct / 100.0)
            * dt_secs
            * 0.085;
        self.accumulated_ring_momentum_tev =
            (self.accumulated_ring_momentum_tev + momentum_gain).min(95_000.0);

        let instant_tev = (compute_power * 0.24 + self.accumulated_ring_momentum_tev)
            * mode_energy_mult
            * harmonic_resonance_boost
            * burst_mult;

        let jitter = Normal::new(1.0, 0.015 * (100.0 - beam_stability_pct) / 100.0)
            .ok()
            .map(|n| n.sample(&mut self.rng).clamp(0.92, 1.12))
            .unwrap_or(1.0);

        self.beam_energy_tev =
            (self.beam_energy_tev * 0.65 + instant_tev * 0.35 * jitter).max(0.05);
        if self.beam_energy_tev > self.peak_energy_tev {
            self.peak_energy_tev = self.beam_energy_tev;
        }

        let lorentz_gamma = 1.0 + (self.beam_energy_tev / PROTON_MASS_TEV);
        let inv_gamma_sq = 1.0 / (lorentz_gamma * lorentz_gamma);
        let beta = (1.0 - inv_gamma_sq).sqrt();
        let nines_of_c = (2.0 * lorentz_gamma * lorentz_gamma).log10().max(1.0);

        let charge_add = (compute_power * 0.38 * dt_secs).clamp(0.05, 8.0);
        self.capacitor_charge_pct = (self.capacitor_charge_pct + charge_add).min(100.0);

        let mean_collisions_per_sec = compute_power
            * 185.0
            * mode_luminosity_mult
            * self.magnet_focus.powf(1.35)
            * (beam_stability_pct / 100.0)
            * burst_mult;

        let lambda = (mean_collisions_per_sec * dt_secs).max(1.0);
        let step_collisions = if lambda < 1e6 {
            Poisson::new(lambda)
                .ok()
                .map(|p| p.sample(&mut self.rng) as u64)
                .unwrap_or(lambda as u64)
        } else {
            lambda as u64
        };

        self.total_collisions = self.total_collisions.saturating_add(step_collisions);
        self.luminosity_fb_inv += (step_collisions as f64) * 1.2e-5;

        let mut best_quantum_drop_chance_per_sec = 0.0;
        let mut any_new_discovery = false;

        for (idx, spec) in QUANTUM_CATALOG.iter().enumerate() {
            let energy_ratio = self.beam_energy_tev / spec.min_energy_tev;
            let energy_factor = if energy_ratio >= 1.0 {
                1.0 + (energy_ratio - 1.0).ln_1p() * 0.85
            } else {
                let tunnel_alpha = if self.mode == AccelerationMode::QuantumResonance {
                    3.2
                } else {
                    6.0
                };
                (-tunnel_alpha * (1.0 - energy_ratio)).exp() * 0.25
            };

            if energy_factor < 1e-6 {
                continue;
            }

            let d_harm = self.rf_harmonic - spec.resonant_harmonic;
            let breit_wigner = 0.35 / (d_harm * d_harm + 0.175);

            let preferred_mode_bonus = if self.mode == spec.preferred_mode {
                1.45
            } else {
                1.0
            };

            let p_single = (spec.base_drop_prob_per_collision
                * energy_factor
                * breit_wigner
                * mode_quantum_mult
                * preferred_mode_bonus
                * burst_mult)
                .clamp(0.0, 0.25);

            let expected_events = (step_collisions as f64) * p_single;
            let step_drop_prob = 1.0 - (-expected_events).exp();
            let drop_prob_per_sec = 1.0 - (-expected_events / dt_secs.max(0.01)).exp();

            if idx == target_idx {
                best_quantum_drop_chance_per_sec = drop_prob_per_sec;
            }

            let sigma_gain = (expected_events * 0.42).min(0.85);
            self.target_sigma_accum[idx] = (self.target_sigma_accum[idx] + sigma_gain).min(9.99);

            let roll: f64 = self.rng.gen();
            let triggered_by_roll = roll < step_drop_prob;
            let triggered_by_5sigma = self.target_sigma_accum[idx] >= 5.0;

            if triggered_by_roll || triggered_by_5sigma {
                let measured_sigma = if triggered_by_5sigma {
                    self.target_sigma_accum[idx].max(5.0)
                } else {
                    (5.0 + (-roll.max(1e-12).ln() * 0.18)).clamp(5.0, 8.9)
                };
                self.target_sigma_accum[idx] = 0.4;

                let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                let is_first_time = !self.discoveries.iter().any(|d| d.id == spec.id);

                if let Some(existing) = self.discoveries.iter_mut().find(|d| d.id == spec.id) {
                    existing.count += 1;
                    if measured_sigma > existing.best_sigma {
                        existing.best_sigma = measured_sigma;
                    }
                    existing.last_probability = step_drop_prob;
                } else {
                    self.discoveries.push(DiscoveryRecord {
                        id: spec.id.to_string(),
                        name: spec.name.to_string(),
                        symbol: spec.symbol.to_string(),
                        tier: spec.tier,
                        category: spec.category.to_string(),
                        mass_gev: spec.mass_gev,
                        best_sigma: measured_sigma,
                        last_probability: step_drop_prob,
                        count: 1,
                        first_discovered_at: now_str,
                        description: spec.description.to_string(),
                    });
                }

                if is_first_time {
                    any_new_discovery = true;
                    self.push_log(format!(
                        "★ QUANTUM DISCOVERY [{:.2}σ]! {} ({}) detected at {:.2} TeV (p={:.2e})!",
                        measured_sigma, spec.name, spec.symbol, self.beam_energy_tev, step_drop_prob
                    ));
                } else if spec.tier >= 3 || self.rng.gen_bool(0.25) {
                    self.push_log(format!(
                        "Rare resonance: {} ({}) observed again ({:.1}σ @ {:.2} TeV).",
                        spec.name, spec.symbol, measured_sigma, self.beam_energy_tev
                    ));
                }

                self.latest_shower = Some(self.generate_shower(
                    spec.tier >= 3,
                    format!("{} ({}) [{:.1}σ]", spec.name, spec.symbol, measured_sigma),
                ));
            }
        }

        if self.latest_shower.is_none() || self.rng.gen_bool(0.35) {
            self.latest_shower = Some(self.generate_shower(
                false,
                format!("pp Collision @ {:.2} TeV", self.beam_energy_tev),
            ));
        }

        if any_new_discovery {
            self.save_to_disk();
        }

        let new_target_idx = self.next_target_index();
        let next_spec = &QUANTUM_CATALOG[new_target_idx];
        let next_sigma = self.target_sigma_accum[new_target_idx];

        let catalog_progress =
            (self.discoveries.len() as f64 / QUANTUM_CATALOG.len() as f64) * 70.0;
        let target_sigma_contrib = (next_sigma / 5.0).clamp(0.0, 1.0) * 20.0;
        let energy_contrib =
            (self.beam_energy_tev / next_spec.min_energy_tev).clamp(0.0, 1.0) * 10.0;
        let quantum_proximity_pct =
            (catalog_progress + target_sigma_contrib + energy_contrib).clamp(0.1, 100.0);

        TelemetrySnapshot {
            uptime_secs,
            active_threads,
            max_threads,
            intensity_pct,
            system_cpu_pct,
            memory_used_mb,
            hashes_per_sec,
            gflops,
            total_compute_cycles,
            mode: self.mode,
            magnet_focus: self.magnet_focus,
            rf_harmonic: self.rf_harmonic,
            optimal_harmonic_hint: next_spec.resonant_harmonic,
            harmonic_alignment_pct,
            beta,
            nines_of_c,
            lorentz_gamma,
            beam_energy_tev: self.beam_energy_tev,
            peak_energy_tev: self.peak_energy_tev,
            luminosity_fb_inv: self.luminosity_fb_inv,
            collisions_per_sec: step_collisions as f64 / dt_secs.max(0.01),
            total_collisions: self.total_collisions,
            beam_stability_pct,
            capacitor_charge_pct: self.capacitor_charge_pct,
            burst_ticks_remaining: self.burst_ticks_remaining,
            quantum_proximity_pct,
            quantum_drop_chance_per_sec: best_quantum_drop_chance_per_sec,
            next_target_name: next_spec.name.to_string(),
            next_target_symbol: next_spec.symbol.to_string(),
            next_target_min_tev: next_spec.min_energy_tev,
            next_target_sigma_progress: next_sigma,
            discoveries: self.discoveries.clone(),
            recent_logs: self.recent_logs.clone(),
            latest_shower: self.latest_shower.clone(),
        }
    }

    fn generate_shower(&mut self, is_quantum_anomaly: bool, label: String) -> CollisionShower {
        self.shower_seq = self.shower_seq.wrapping_add(1);
        let num_tracks = if is_quantum_anomaly {
            self.rng.gen_range(16..=28)
        } else {
            self.rng.gen_range(8..=16)
        };

        let mut tracks = Vec::with_capacity(num_tracks);
        for _ in 0..num_tracks {
            let angle_rad = self.rng.gen_range(0.0..std::f64::consts::TAU);
            let curvature = self.rng.gen_range(-0.75..0.75);
            let speed = self.rng.gen_range(0.55..1.40);
            let energy_ratio = self.rng.gen_range(0.30..1.0);
            let kind = if is_quantum_anomaly && self.rng.gen_bool(0.45) {
                TrackKind::QuantumAnomaly
            } else {
                match self.rng.gen_range(0..3) {
                    0 => TrackKind::HadronJet,
                    1 => TrackKind::MuonCurve,
                    _ => TrackKind::PhotonRay,
                }
            };
            let decay_fork_at = if self.rng.gen_bool(if is_quantum_anomaly { 0.45 } else { 0.22 }) {
                Some(self.rng.gen_range(0.35..0.72))
            } else {
                None
            };
            tracks.push(ShowerTrack {
                angle_rad,
                curvature,
                speed,
                energy_ratio,
                kind,
                decay_fork_at,
            });
        }

        CollisionShower {
            seq_id: self.shower_seq,
            energy_tev: self.beam_energy_tev,
            is_quantum_anomaly,
            label,
            tracks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relativistic_kinematics_and_progression() {
        let mut phys = AcceleratorPhysicsState::load_or_new();
        phys.reset();
        let snap = phys.step(
            0.2,
            450_000.0,
            5.2,
            0xABCD_1234_5678_9DEF,
            10,
            4,
            8,
            80,
            65.0,
            512,
            90_000,
        );
        assert!(snap.beam_energy_tev > 0.1);
        assert!(snap.beta > 0.99 && snap.beta < 1.0);
        assert!(snap.lorentz_gamma > 100.0);
        assert!(snap.collisions_per_sec > 0.0);
        assert!(snap.quantum_proximity_pct > 0.0);
    }

    #[test]
    fn test_mode_and_harmonic_resonance_boost() {
        let mut phys_off = AcceleratorPhysicsState::load_or_new();
        phys_off.reset();
        phys_off.rf_harmonic = 4.9;
        let snap_off = phys_off.step(0.5, 500_000.0, 6.0, 42, 1, 4, 8, 80, 50.0, 200, 1000);

        let mut phys_on = AcceleratorPhysicsState::load_or_new();
        phys_on.reset();
        phys_on.rf_harmonic = 1.0;
        let snap_on = phys_on.step(0.5, 500_000.0, 6.0, 42, 1, 4, 8, 80, 50.0, 200, 1000);

        assert!(snap_on.harmonic_alignment_pct > snap_off.harmonic_alignment_pct);
        assert!(snap_on.beam_energy_tev > snap_off.beam_energy_tev);
    }
}
