# ⚛️ Local Collider (`local_collider`)

A **CLI particle accelerator simulator** written in **Rust** that trades **real CPU compute** (SHA-256 cryptographic hashing + $4\times 4$ relativistic Lorentz matrix FMA operations / GFLOPS) for relativistic particle velocity ($\beta = v/c$, Lorentz factor $\gamma$, beam energy in $\text{TeV}$) and **real statistical discovery probabilities** (Poisson collision rates, Gaussian cavity phase jitter, Breit-Wigner resonance curves, and $5\sigma$ significance thresholds) to uncover quantum particles and anomalies.

Designed around a **multi-process architecture** connected over instant TCP IPC (`127.0.0.1:7878`), plus a native **macOS Menu Bar Guardian & `.app` Bundle**:
1. **macOS Menu Bar Guardian & `Local Collider.app` *(Native AppKit Status Bar & Double-Click Launcher)***:
   - Displays a persistent icon in the top-right macOS Menu Bar showing live resource consumption right next to the button (`⚛ 26% CPU · 18.4 TeV` when running, or `⚛ OFF · 0% CPU` when stopped) so there is zero risk of leaving CPU threads running unintentionally in the background.
   - Double-clicking **`Local Collider.app`** in Finder installs the Menu Bar Guardian and automatically opens all **3 Terminal windows**.
   - Includes one-click menu links to open all 3 terminals (or each terminal individually), throttle CPU usage (Eco / Balanced / Overdrive), or **Stop All Acceleration & Close Terminals (0% CPU)**.
2. **Window 1 — `collider-engine` *(Mouse-Clickable Engine Control Deck & Daemon)***: Runs the real multi-threaded CPU benchmark pool, simulates relativistic kinematics and $5\sigma$ quantum physics, saves discoveries to disk, and features a **mouse-clickable TUI Control Deck** where you can click buttons to allocate CPU threads/intensity, switch acceleration modes, auto-lock RF cavity harmonics, and fire supercritical capacitor bursts.
3. **Window 2 — `collider-viz` *(Separate High-FPS CLI Animation Visualizer)***: Connects to the running engine from a second terminal and renders a live **Braille TUI animation** featuring multi-bunch counter-rotating proton beams, glowing 36-sector calorimeter towers, curved solenoid particle tracks with secondary boson decay forks ($Z^0 \to \mu^+\mu^-$), and 3 clickable camera views (including a live **Breit-Wigner RF Resonance Oscilloscope**).

---

## 🏗️ Architecture

```text
                  ┌────────────────────────────────────────────────────────┐
                  │  macOS Menu Bar Guardian: "⚛ 26% CPU · 18.4 TeV"       │
                  │  • Live CPU %, kH/s, GFLOPS, RAM & Beam TeV in Menu Bar│
                  │  • One-click: Open All 3 Terminals (or 1 / 2 / 3)      │
                  │  • One-click: Stop All Acceleration & Close Terminals  │
                  └───────────────────────────▲────────────────────────────┘
                                              │ TCP JSON IPC (127.0.0.1:7878)
┌─────────────────────────────────────────────┴┐                            ┌─────────────────────────────────────────────┐
│   Window 1: collider-engine (Control Deck)   │ ◄────────────────────────► │    Window 2: collider-viz (TUI Animation)   │
│  • Mouse-Clickable TUI Control Buttons       │       127.0.0.1:7878       │  • Multi-Bunch Synchrotron & Calorimeter    │
│  • Real CPU Compute Pool (SHA-256 + GFLOPS)  │                            │  • Curved Helical Tracks & Boson Decay Forks│
│  • Relativistic Kinematics (β, γ, TeV)       │ ◄────────────────────────► │  • 3 Camera Modes (Ring / Vertex / RF Scope)│
│  • Poisson & Breit-Wigner 5σ Discovery Engine│  Window 3: CLI `status`,   │  • Clickable Canvas & Remote IPC Controls   │
└──────────────────────────────────────────────┘  `tune`, `codex`, `stop`   └─────────────────────────────────────────────┘
```

---

## 🍎 macOS App Bundle (`Local Collider.app`) & Menu Bar Guardian

Even though Local Collider is a CLI/TUI simulator, macOS users can build and double-click **`Local Collider.app`** in Finder (or move it to `/Applications`):

```bash
# Build the self-contained macOS application bundle ("Local Collider.app")
./scripts/build_macos_app.sh
```

- **Double-Click Launch**: Double-clicking `Local Collider.app` starts the Menu Bar Guardian (`⚛ 26% CPU · 18.4 TeV`) and opens all 3 Terminal windows automatically.
- **Automatic Safety Guardian**: Even if you start `collider-engine` directly from a terminal, it automatically spawns the Menu Bar Guardian in `--status-only` mode so you can always see live CPU consumption in your macOS menu bar and stop the engine with one click (`🛑 Stop All Acceleration & Close Terminals (0% CPU)`).

---

## 🚀 Manual CLI Quick Start

### 1. Window 1 — Start the Clickable Engine Control Deck

```bash
cargo run --release --bin collider-engine -- --threads 4 --intensity 80
```

- **Fully Mouse-Clickable**: Click any button in Window 1 (`[-1 THREAD]`, `[+1 THREAD]`, `[MAX CORES]`, `[CPU 25%..100%]`, the 4 Acceleration Mode cards, `[AUTO-LOCK RF]`, `[FIRE SUPERCRITICAL PULSE BURST]`, or any row in the **Quantum Codex**) to tune the accelerator in real time!

### 2. Window 2 — Launch the Separate TUI Collider Animation

In a **second terminal window**, run:

```bash
cargo run --release --bin collider-viz
```

- **3 Camera Views** (click the top tabs or press `1`, `2`, `3`, or `V`):
  - **`[1] Full Ring & Detector`**: Full cryogenic synchrotron ring, 12 pulsing RF cavities, counter-rotating proton bunches with relativistic Doppler color shift, 36-sector angular calorimeter towers, and curved collision tracks.
  - **`[2] Vertex & Calorimeter Zoom`**: High-magnification view of the inner Silicon Vertex Tracker showing opposing beam pinch, secondary boson decay forks, and calorimeter energy deposition.
  - **`[3] RF Breit-Wigner Scope`**: Dual-trace live oscilloscope plotting the animated RF cavity standing wave against the target particle's **Breit-Wigner resonance peak**.

### 3. Window 3 (Optional) — CLI Telemetry, Remote Tuning, Codex & Emergency Stop

```bash
# View live engine telemetry (or saved disk snapshot if offline)
cargo run --release --bin local-collider -- status

# Remotely switch acceleration mode, auto-lock RF harmonic resonance, and fire a burst
cargo run --release --bin local-collider -- tune --mode quantum --auto-lock-rf --burst

# Inspect the 12-tier 5σ Quantum Discovery Codex
cargo run --release --bin local-collider -- codex

# Immediately save state and stop all CPU acceleration (0% CPU)
cargo run --release --bin local-collider -- stop
```

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
