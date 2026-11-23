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
const STATE_FILE: &str = "collider_state.json";

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
        name: "Par de Múons Relativísticos",
        symbol: "μ⁺μ⁻",
        tier: 1,
        category: "Modelo Padrão (Léptons)",
        mass_gev: 0.211,
        min_energy_tev: 0.12,
        base_drop_prob_per_collision: 1.2e-4,
        preferred_mode: AccelerationMode::LinearLinac,
        resonant_harmonic: 1.0,
        description: "Léptons pesados de segunda geração com dilatação temporal observável.",
    },
    QuantumTargetSpec {
        id: "jpsi_meson",
        name: "Méson J/ψ (Charmônio)",
        symbol: "J/ψ",
        tier: 1,
        category: "Modelo Padrão (Hádrons)",
        mass_gev: 3.0969,
        min_energy_tev: 0.45,
        base_drop_prob_per_collision: 5.5e-5,
        preferred_mode: AccelerationMode::LinearLinac,
        resonant_harmonic: 1.3,
        description: "Estado ligado quark charm-anticharm que confirmou o modelo de quarks.",
    },
    QuantumTargetSpec {
        id: "z_boson",
        name: "Bóson Vetorial Z⁰",
        symbol: "Z⁰",
        tier: 2,
        category: "Bóson de Calibre Eletrofraco",
        mass_gev: 91.1876,
        min_energy_tev: 1.25,
        base_drop_prob_per_collision: 2.2e-5,
        preferred_mode: AccelerationMode::SynchrotronHarmonic,
        resonant_harmonic: 1.7,
        description: "Mediador neutro da força nuclear fraca em colisões de alta luminosidade.",
    },
    QuantumTargetSpec {
        id: "higgs_boson",
        name: "Bóson de Higgs Escalar",
        symbol: "H⁰",
        tier: 2,
        category: "Quebra Espontânea de Simetria",
        mass_gev: 125.25,
        min_energy_tev: 2.90,
        base_drop_prob_per_collision: 9.0e-6,
        preferred_mode: AccelerationMode::SynchrotronHarmonic,
        resonant_harmonic: 2.1,
        description: "Excitação quântica do campo de Higgs responsável pela massa das partículas elementares.",
    },
    QuantumTargetSpec {
        id: "tetraquark_tcc",
        name: "Tetraquark Duplo-Charmoso",
        symbol: "Tcc⁺",
        tier: 3,
        category: "Hádron Exótico Multiquark",
        mass_gev: 3.875,
        min_energy_tev: 6.20,
        base_drop_prob_per_collision: 3.8e-6,
        preferred_mode: AccelerationMode::SynchrotronHarmonic,
        resonant_harmonic: 2.4,
        description: "Configuração quântica exótica de quatro quarks (ccūd̄) de vida longa.",
    },
    QuantumTargetSpec {
        id: "pentaquark_pc",
        name: "Pentaquark de Valência Charm",
        symbol: "Pc(4457)⁺",
        tier: 3,
        category: "Hádron Exótico Multiquark",
        mass_gev: 4.457,
        min_energy_tev: 11.5,
        base_drop_prob_per_collision: 1.6e-6,
        preferred_mode: AccelerationMode::PlasmaWakefield,
        resonant_harmonic: 2.8,
        description: "Bárion exótico composto por quatro quarks e um antiquark em forte acoplamento.",
    },
    QuantumTargetSpec {
        id: "susy_neutralino",
        name: "Neutralino Supersimétrico",
        symbol: "χ̃₁⁰",
        tier: 4,
        category: "Supersimetria (Matéria Escura)",
        mass_gev: 480.0,
        min_energy_tev: 21.0,
        base_drop_prob_per_collision: 7.2e-7,
        preferred_mode: AccelerationMode::PlasmaWakefield,
        resonant_harmonic: 3.2,
        description: "Superparceiro fermiônico estável e principal candidato quântico à Matéria Escura.",
    },
    QuantumTargetSpec {
        id: "leptoquark_lq3",
        name: "Leptoquark Vetorial de 3ª Geração",
        symbol: "LQ₃",
        tier: 4,
        category: "Grande Unificação (Além do MP)",
        mass_gev: 1450.0,
        min_energy_tev: 38.0,
        base_drop_prob_per_collision: 3.1e-7,
        preferred_mode: AccelerationMode::PlasmaWakefield,
        resonant_harmonic: 3.6,
        description: "Bóson hipotético que acopla quarks diretamente a léptons violando a universalidade.",
    },
    QuantumTargetSpec {
        id: "kk_graviton",
        name: "Gráviton de Kaluza-Klein",
        symbol: "G_KK",
        tier: 5,
        category: "Gravidade Quântica / Dimensões Extras",
        mass_gev: 2900.0,
        min_energy_tev: 68.0,
        base_drop_prob_per_collision: 1.4e-7,
        preferred_mode: AccelerationMode::QuantumResonance,
        resonant_harmonic: 4.0,
        description: "Modo excitado do gráviton propagando-se em dimensões espaciais extras compactadas.",
    },
    QuantumTargetSpec {
        id: "dirac_monopole",
        name: "Monopolo Magnético de Dirac",
        symbol: "g_D",
        tier: 5,
        category: "Topologia Quântica de Gauge",
        mass_gev: 4600.0,
        min_energy_tev: 115.0,
        base_drop_prob_per_collision: 6.0e-8,
        preferred_mode: AccelerationMode::QuantumResonance,
        resonant_harmonic: 4.3,
        description: "Soliton topológico com carga magnética isolada que quantiza a carga elétrica.",
    },
    QuantumTargetSpec {
        id: "quantum_foam_condensate",
        name: "Condensado de Espuma Quântica",
        symbol: "Ψ_foam",
        tier: 6,
        category: "Anomalia da Escala de Planck",
        mass_gev: 8800.0,
        min_energy_tev: 185.0,
        base_drop_prob_per_collision: 2.5e-8,
        preferred_mode: AccelerationMode::QuantumResonance,
        resonant_harmonic: 4.7,
        description: "Micro-flutuação espaço-temporal onde a métrica quântica sofre tunelamento coerente.",
    },
    QuantumTargetSpec {
        id: "brane_singularity",
        name: "Singularidade de Tunelamento de Brana",
        symbol: "Ω_brane",
        tier: 6,
        category: "Singularidade Quântica Profunda",
        mass_gev: 14000.0,
        min_energy_tev: 300.0,
        base_drop_prob_per_collision: 1.0e-8,
        preferred_mode: AccelerationMode::QuantumResonance,
        resonant_harmonic: 4.9,
        description: "Ruptura local de vácuo quântico com troca de momento através do bulk hiperdimensional.",
    },
];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PersistentColliderSave {
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
                if let Ok(saved) = serde_json::from_str::<PersistentColliderSave>(&raw) {
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
                        "Estado restaurado: {} descobertas quânticas, pico de {:.2} TeV.",
                        state.discoveries.len(),
                        state.peak_energy_tev
                    ));
                    return state;
                }
            }
        }

        state.push_log(
            "Anel acelerador inicializado. Injetando feixe de prótons nas cavidades RF...".into(),
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
        self.push_log("Estado do colisor reiniciado para calibração zero.".into());
        self.save_to_disk();
    }

    pub fn push_log(&mut self, msg: String) {
        let stamp = Local::now().format("%H:%M:%S");
        self.recent_logs.push(format!("[{}] {}", stamp, msg));
        if self.recent_logs.len() > 40 {
            self.recent_logs.remove(0);
        }
    }

    pub fn trigger_burst(&mut self) -> bool {
        if self.capacitor_charge_pct >= 25.0 {
            let bonus_ticks = ((self.capacitor_charge_pct / 100.0) * 90.0).round() as u32;
            self.burst_ticks_remaining += bonus_ticks.max(25);
            self.push_log(format!(
                "PULSO SUPERCRÍTICO ATIVADO! Descarga de {:.0}% do capacitor (+{:.1}s de sobrecarga quântica).",
                self.capacitor_charge_pct,
                bonus_ticks as f64 * 0.1
            ));
            self.capacitor_charge_pct = 0.0;
            true
        } else {
            self.push_log(format!(
                "Capacitor em {:.1}% (mínimo 25% necessário para pulso supercrítico).",
                self.capacitor_charge_pct
            ));
            false
        }
    }

    /// Determina o próximo alvo quântico ainda não descoberto (ou o último se todos já foram).
    pub fn next_target_index(&self) -> usize {
        for (idx, spec) in QUANTUM_CATALOG.iter().enumerate() {
            if !self.discoveries.iter().any(|d| d.id == spec.id) {
                return idx;
            }
        }
        QUANTUM_CATALOG.len() - 1
    }

    /// Avança um passo de simulação física alimentado pelas métricas reais de CPU.
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
        // Mistura a entropia real calculada pelo SHA-256 da CPU no gerador estatístico
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

        // Alinhamento harmônico ("Como acelera": sintonia fina da cavidade RF com a ressonância alvo)
        let harmonic_diff = (self.rf_harmonic - optimal_harmonic).abs();
        let harmonic_alignment_pct = ((-harmonic_diff * harmonic_diff / 0.45).exp() * 100.0)
            .clamp(5.0, 100.0);
        let harmonic_resonance_boost = 0.75 + 1.45 * (harmonic_alignment_pct / 100.0);

        // Multiplicadores do modo de aceleração ("Como acelera")
        let (mode_energy_mult, mode_luminosity_mult, mode_quantum_mult, base_stability) =
            match self.mode {
                AccelerationMode::LinearLinac => (1.00, 1.05, 1.00, 98.0),
                AccelerationMode::SynchrotronHarmonic => (1.45, 1.65, 1.35, 92.0),
                AccelerationMode::PlasmaWakefield => (2.40, 1.20, 1.90, 78.0),
                AccelerationMode::QuantumResonance => (1.80, 1.30, 2.85, 84.0),
            };

        // Foco magnético dos quadrupolos: aumenta colisões mas exige alinhamento harmônico para manter estabilidade
        let focus_penalty = ((self.magnet_focus - 1.2).max(0.0) * 14.0)
            * (1.0 - (harmonic_alignment_pct / 130.0));
        let beam_stability_pct = (base_stability - focus_penalty
            + (harmonic_alignment_pct * 0.08))
            .clamp(25.0, 99.9);

        // Sobrecarga de pulso supercrítico do capacitor
        let burst_mult = if self.burst_ticks_remaining > 0 {
            self.burst_ticks_remaining -= 1;
            3.25
        } else {
            1.0
        };

        // Conversão direta de Benchmark de CPU (Hashes/s + GFLOPS) em aceleração do anel
        // Escala sublinear calibrada para builds debug e --release: mais threads + sintonia RF + tempo = mais TeV
        let raw_compute_score = (hashes_per_sec / 120_000.0) + (gflops * 3.5);
        let compute_power = raw_compute_score.max(0.08).powf(0.72);

        // Acúmulo contínuo de momento no anel (quanto mais tempo e mais CPU você troca, mais o feixe acelera)
        let momentum_gain = compute_power
            * mode_energy_mult
            * harmonic_resonance_boost
            * (beam_stability_pct / 100.0)
            * dt_secs
            * 0.085;
        self.accumulated_ring_momentum_tev =
            (self.accumulated_ring_momentum_tev + momentum_gain).min(95_000.0);

        // Energia instantânea do feixe em TeV = Potência instantânea de CPU + Momento relativístico acumulado
        let instant_tev =
            (compute_power * 0.24 + self.accumulated_ring_momentum_tev)
                * mode_energy_mult
                * harmonic_resonance_boost
                * burst_mult;

        // Suavização exponencial + flutuação gaussiana térmica real das cavidades RF
        let jitter = Normal::new(1.0, 0.015 * (100.0 - beam_stability_pct) / 100.0)
            .ok()
            .map(|n| n.sample(&mut self.rng).clamp(0.92, 1.12))
            .unwrap_or(1.0);

        self.beam_energy_tev =
            (self.beam_energy_tev * 0.65 + instant_tev * 0.35 * jitter).max(0.05);
        if self.beam_energy_tev > self.peak_energy_tev {
            self.peak_energy_tev = self.beam_energy_tev;
        }

        // Cinemática relativística real: γ = 1 + E / (m_p c²) e β = v/c = sqrt(1 - 1/γ²)
        let lorentz_gamma = 1.0 + (self.beam_energy_tev / PROTON_MASS_TEV);
        let inv_gamma_sq = 1.0 / (lorentz_gamma * lorentz_gamma);
        let beta = (1.0 - inv_gamma_sq).sqrt();
        // Número de noves de c: -log10(1 - β) ≈ log10(2 * γ²) para alta precisão numérica
        let nines_of_c = (2.0 * lorentz_gamma * lorentz_gamma).log10().max(1.0);

        // Carregamento do capacitor com os hashes da CPU
        let charge_add = (compute_power * 0.38 * dt_secs).clamp(0.05, 8.0);
        self.capacitor_charge_pct = (self.capacitor_charge_pct + charge_add).min(100.0);

        // Taxa de colisões por segundo governada por Distribuição de Poisson real
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
        // Luminosidade integrada em femtobarns inversos (fb⁻¹)
        self.luminosity_fb_inv += (step_collisions as f64) * 1.2e-5;

        // Cálculo de proximidade quântica e probabilidade estatística de descoberta para cada partícula/anomalia
        let mut best_quantum_drop_chance_per_sec = 0.0;
        let mut any_new_discovery = false;

        for (idx, spec) in QUANTUM_CATALOG.iter().enumerate() {
            // Fator de ativação energética (curva relativística suave + barreira de tunelamento abaixo do limiar)
            let energy_ratio = self.beam_energy_tev / spec.min_energy_tev;
            let energy_factor = if energy_ratio >= 1.0 {
                1.0 + (energy_ratio - 1.0).ln_1p() * 0.85
            } else {
                // Tunelamento quântico sub-limiar: decai exponencialmente se a energia for baixa,
                // mas o modo Ressonância Quântica permite tunelamento próximo ao limiar!
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

            // Ressonância Breit-Wigner com o harmônico RF configurado pelo usuário
            let d_harm = self.rf_harmonic - spec.resonant_harmonic;
            let breit_wigner = 0.35 / (d_harm * d_harm + 0.175); // Pico = 2.0 quando sintonizado

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

            // Probabilidade binomial exata de >= 1 evento raro em `step_collisions` colisões:
            // P(X >= 1) = 1 - (1 - p)^N ≈ 1 - exp(-N * p)
            let expected_events = (step_collisions as f64) * p_single;
            let step_drop_prob = 1.0 - (-expected_events).exp();
            let drop_prob_per_sec = 1.0 - (-expected_events / dt_secs.max(0.01)).exp();

            if idx == target_idx {
                best_quantum_drop_chance_per_sec = drop_prob_per_sec;
            }

            // Acumula significância estatística (σ) proporcional à razão Sinal / sqrt(Fundo)
            let sigma_gain = (expected_events * 0.42).min(0.85);
            self.target_sigma_accum[idx] = (self.target_sigma_accum[idx] + sigma_gain).min(9.99);

            // Rolagem estatística real alimentada pela entropia da CPU OU atingimento do limiar 5σ padrão do CERN!
            let roll: f64 = self.rng.gen();
            let triggered_by_roll = roll < step_drop_prob;
            let triggered_by_5sigma = self.target_sigma_accum[idx] >= 5.0;

            if triggered_by_roll || triggered_by_5sigma {
                let measured_sigma = if triggered_by_5sigma {
                    self.target_sigma_accum[idx].max(5.0)
                } else {
                    (5.0 + (-roll.max(1e-12).ln() * 0.18)).clamp(5.0, 8.9)
                };
                // Reinicia acumulador de σ para a próxima observação
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
                        "★ DESCOBERTA QUÂNTICA [{:.2}σ]! {} ({}) detectado a {:.2} TeV (p={:.2e})!",
                        measured_sigma, spec.name, spec.symbol, self.beam_energy_tev, step_drop_prob
                    ));
                } else if spec.tier >= 3 || self.rng.gen_bool(0.25) {
                    self.push_log(format!(
                        "Evento raro: {} ({}) observado novamente ({:.1}σ @ {:.2} TeV).",
                        spec.name, spec.symbol, measured_sigma, self.beam_energy_tev
                    ));
                }

                self.latest_shower = Some(self.generate_shower(
                    spec.tier >= 4,
                    format!("{} ({}) [{:.1}σ]", spec.name, spec.symbol, measured_sigma),
                ));
            }
        }

        // Caso não tenha havido chuveiro de descoberta neste frame, gera chuveiros de colisão de fundo
        if self.latest_shower.is_none() || self.rng.gen_bool(0.35) {
            self.latest_shower = Some(self.generate_shower(
                false,
                format!("Colisão pp @ {:.2} TeV", self.beam_energy_tev),
            ));
        }

        if any_new_discovery {
            self.save_to_disk();
        }

        let new_target_idx = self.next_target_index();
        let next_spec = &QUANTUM_CATALOG[new_target_idx];
        let next_sigma = self.target_sigma_accum[new_target_idx];

        // Proximidade Quântica Global (0% -> 100%): combina fração do catálogo descoberto,
        // progresso 5σ do próximo alvo e regime relativístico de Lorentz
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
            self.rng.gen_range(14..=24)
        } else {
            self.rng.gen_range(6..=14)
        };

        let mut tracks = Vec::with_capacity(num_tracks);
        for _ in 0..num_tracks {
            let angle_rad = self.rng.gen_range(0.0..std::f64::consts::TAU);
            let curvature = self.rng.gen_range(-0.65..0.65);
            let speed = self.rng.gen_range(0.55..1.35);
            let energy_ratio = self.rng.gen_range(0.25..1.0);
            let kind = if is_quantum_anomaly && self.rng.gen_bool(0.45) {
                TrackKind::QuantumAnomaly
            } else {
                match self.rng.gen_range(0..3) {
                    0 => TrackKind::HadronJet,
                    1 => TrackKind::MuonCurve,
                    _ => TrackKind::PhotonRay,
                }
            };
            tracks.push(ShowerTrack {
                angle_rad,
                curvature,
                speed,
                energy_ratio,
                kind,
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
        phys_off.rf_harmonic = 4.9; // Longe da ressonância inicial (1.0)
        let snap_off = phys_off.step(0.5, 500_000.0, 6.0, 42, 1, 4, 8, 80, 50.0, 200, 1000);

        let mut phys_on = AcceleratorPhysicsState::load_or_new();
        phys_on.reset();
        phys_on.rf_harmonic = 1.0; // Exatamente na ressonância do primeiro alvo
        let snap_on = phys_on.step(0.5, 500_000.0, 6.0, 42, 1, 4, 8, 80, 50.0, 200, 1000);

        assert!(snap_on.harmonic_alignment_pct > snap_off.harmonic_alignment_pct);
        assert!(snap_on.beam_energy_tev > snap_off.beam_energy_tev);
    }
}
