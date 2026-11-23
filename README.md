# ⚛️ Local Collider (`local_collider`)

A **CLI particle accelerator simulator** written in **Rust** that trades **real CPU compute** (SHA-256 cryptographic hashing + $4\times 4$ relativistic Lorentz matrix FMA operations / GFLOPS) for relativistic particle velocity ($\beta = v/c$, Lorentz factor $\gamma$, beam energy in $\text{TeV}$) and **real statistical discovery probabilities** (Poisson collision rates, Gaussian cavity phase jitter, Breit-Wigner resonance curves, and $5\sigma$ significance thresholds) to uncover quantum particles and anomalies.

Designed around a **two-process architecture** connected over instant TCP IPC (`127.0.0.1:7878`):
1. **Window 1 — `collider-engine` *(Mouse-Clickable Engine Control Deck & Daemon)***: Runs the real multi-threaded CPU benchmark pool, simulates relativistic kinematics and $5\sigma$ quantum physics, saves discoveries to disk, and features a **mouse-clickable TUI Control Deck** where you can click buttons to allocate CPU threads/intensity, switch acceleration modes, auto-lock RF cavity harmonics, and fire supercritical capacitor bursts.
2. **Window 2 — `collider-viz` *(Separate High-FPS CLI Animation Visualizer)***: Connects to the running engine from a second terminal and renders a live **Braille TUI animation** featuring multi-bunch counter-rotating proton beams, glowing 36-sector calorimeter towers, curved solenoid particle tracks with secondary boson decay forks ($Z^0 \to \mu^+\mu^-$), and 3 clickable camera views (including a live **Breit-Wigner RF Resonance Oscilloscope**).

---

## 🏗️ Two-Process Architecture

```text
┌─────────────────────────────────────────────┐       Instant TCP IPC        ┌─────────────────────────────────────────────┐
│   Window 1: collider-engine (Control Deck)  │ ◄──────────────────────────► │    Window 2: collider-viz (TUI Animation)   │
│  • Mouse-Clickable TUI Control Buttons      │        127.0.0.1:7878        │  • Multi-Bunch Synchrotron & Calorimeter    │
│  • Real CPU Compute Pool (SHA-256 + GFLOPS) │                              │  • Curved Helical Tracks & Boson Decay Forks│
│  • Relativistic Kinematics (β, γ, TeV)      │ ◄──────────────────────────► │  • 3 Camera Modes (Ring / Vertex / RF Scope)│
│  • Poisson & Breit-Wigner 5σ Discovery Engine│  Window 3: CLI `status`,    │  • Clickable Canvas & Remote IPC Controls   │
└─────────────────────────────────────────────┘       `tune`, and `codex`    └─────────────────────────────────────────────┘
```

---

## 🚀 Quick Start

### 1. Window 1 — Start the Clickable Engine Control Deck

```bash
cargo run --release --bin collider-engine -- --threads 4 --intensity 80
```

- **Fully Mouse-Clickable**: Click any button in Window 1 (`[-1 THREAD]`, `[+1 THREAD]`, `[MAX CORES]`, `[CPU 25%..100%]`, the 4 Acceleration Mode cards, `[AUTO-LOCK RF]`, `[FIRE SUPERCRITICAL PULSE BURST]`, or any row in the **Quantum Codex**) to tune the accelerator in real time!
- *(Optional headless mode for scripts/servers: pass `--headless`)*

### 2. Window 2 — Launch the Separate TUI Collider Animation

In a **second terminal window**, run:

```bash
cargo run --release --bin collider-viz
```

- **3 Camera Views** (click the top tabs or press `1`, `2`, `3`, or `V`):
  - **`[1] Full Ring & Detector`**: Full cryogenic synchrotron ring, 12 pulsing RF cavities, counter-rotating proton bunches with relativistic Doppler color shift, 36-sector angular calorimeter towers, and curved collision tracks.
  - **`[2] Vertex & Calorimeter Zoom`**: High-magnification view of the inner Silicon Vertex Tracker showing opposing beam pinch, secondary boson decay forks, and calorimeter energy deposition.
  - **`[3] RF Breit-Wigner Scope`**: Dual-trace live oscilloscope plotting the animated RF cavity standing wave against the target particle's **Breit-Wigner resonance peak**.

### 3. Window 3 (Optional) — CLI Telemetry, Remote Tuning & Codex

Query or tune the accelerator from any terminal (works both while the engine is online and offline via saved disk state):

```bash
# View live engine telemetry (or saved disk snapshot if offline)
cargo run --release --bin local-collider -- status

# Remotely switch acceleration mode, auto-lock RF harmonic resonance, and fire a burst
cargo run --release --bin local-collider -- tune --mode quantum --auto-lock-rf --burst

# Inspect the 12-tier 5σ Quantum Discovery Codex
cargo run --release --bin local-collider -- codex
```

---

## ⚙️ Core Mechanics: "How Much & How You Accelerate"

1. **How Much You Accelerate (Real CPU Compute $\to$ Relativistic Velocity)**:
   - Active worker threads execute real **SHA-256** rounds coupled with **4D Lorentz matrix FMA transformations**.
   - Measured **kH/s** and **GFLOPS** pump relativistic momentum into the storage ring, increasing beam energy ($\text{TeV}$), Lorentz factor ($\gamma = 1 + E / m_p c^2$), and pushing velocity $\beta = v/c = \sqrt{1 - 1/\gamma^2}$ closer to the speed of light (measured in **"nines of $c$"**, e.g. $0.999999999c$).
   - Hardware SHA-256 digests continuously re-seed the statistical collision RNG with real CPU entropy.

2. **How You Accelerate (4 Modes + RF Harmonic Resonance + Quadrupole Focus + Capacitor)**:
   - **`Stable RF Linac` (`linac`)**: $98\%$ phase stability, ideal for light leptons/mesons and initial calibration.
   - **`Harmonic Synchrotron Ring` (`synchrotron`)**: Balanced magnetic storage ring with $1.65\times$ collision luminosity.
   - **`Laser Plasma Wakefield` (`plasma`)**: $2.40\times$ peak $\text{TeV}$ energy multiplier with higher phase entropy.
   - **`Quantum Vacuum Resonance` (`quantum`)**: Modulates vacuum harmonics to boost sub-threshold quantum tunneling and exotic drop rates by $2.85\times$.
   - **Breit-Wigner RF Harmonic Tuning ($\text{GHz}$)**: Each quantum target has a resonant frequency. Matching the RF cavity harmonic to the target's resonance peak (or clicking any row in the Quantum Codex) multiplies cross-section probability and energy transfer.

---

## 🌌 5σ Quantum Discovery Codex (12 Tiers)

| Tier | Symbol | Particle / Quantum Phenomenon | Threshold ($\text{TeV}$) | RF Harmonic | Category |
|:---:|:---:|---|---:|---:|---|
| 1 | `μ⁺μ⁻` | Relativistic Muon Pair | `0.12 TeV` | `1.0 GHz` | Standard Model (Leptons) |
| 1 | `J/ψ` | J/ψ Charmonium Meson | `0.45 TeV` | `1.3 GHz` | Standard Model (Hadrons) |
| 2 | `Z⁰` | Z⁰ Electroweak Vector Boson | `1.25 TeV` | `1.7 GHz` | Electroweak Gauge Boson |
| 2 | `H⁰` | Scalar Higgs Boson | `2.90 TeV` | `2.1 GHz` | Spontaneous Symmetry Breaking |
| 3 | `Tcc⁺` | Double-Charm Tetraquark | `6.20 TeV` | `2.4 GHz` | Exotic Multiquark Hadron |
| 3 | `Pc(4457)⁺` | Charmonium Pentaquark | `11.50 TeV` | `2.8 GHz` | Exotic Multiquark Hadron |
| 4 | `χ̃₁⁰` | Supersymmetric Neutralino | `21.00 TeV` | `3.2 GHz` | Supersymmetry (Dark Matter) |
| 4 | `LQ₃` | 3rd-Gen Vector Leptoquark | `38.00 TeV` | `3.6 GHz` | Grand Unification (Beyond SM) |
| 5 | `G_KK` | Kaluza-Klein Graviton | `68.00 TeV` | `4.0 GHz` | Quantum Gravity / Extra Dimensions |
| 5 | `g_D` | Dirac Magnetic Monopole | `115.00 TeV` | `4.3 GHz` | Quantum Gauge Topology |
| 6 | `Ψ_foam` | Quantum Foam Condensate | `185.00 TeV` | `4.7 GHz` | Planck-Scale Spacetime Anomaly |
| 6 | `Ω_brane` | Brane Tunneling Singularity | `300.00 TeV` | `4.9 GHz` | Deep Multiverse Singularity |

---

## 📄 License

Distributed under the **MIT License**. See [LICENSE](LICENSE) for details.
