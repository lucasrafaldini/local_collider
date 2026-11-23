use clap::{Parser, Subcommand};
use local_collider::engine::{
    build_offline_snapshot, run_engine_daemon, send_command_once, EngineConfig,
};
use local_collider::physics::QUANTUM_CATALOG;
use local_collider::protocol::{AccelerationMode, EngineCommand, DEFAULT_HOST, DEFAULT_PORT};
use local_collider::viz::{run_tui_visualizer, VizConfig};

#[derive(Parser)]
#[command(
    name = "local-collider",
    version,
    about = "CLI particle accelerator simulator that trades real CPU processing for relativistic beam velocity and quantum discoveries."
)]
struct Cli {
    /// IPC server host for the engine daemon
    #[arg(long, global = true, default_value = DEFAULT_HOST)]
    host: String,

    /// IPC server TCP port for the engine daemon
    #[arg(short, long, global = true, default_value_t = DEFAULT_PORT)]
    port: u16,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch the particle accelerator engine & clickable control deck (Window 1)
    Engine {
        /// Initial number of CPU threads dedicated to RF cavities
        #[arg(short, long, default_value_t = 2)]
        threads: usize,

        /// CPU compute intensity per thread (5 to 100%)
        #[arg(short, long, default_value_t = 70)]
        intensity: u8,

        /// Initial acceleration mode (linac, synchrotron, plasma, quantum)
        #[arg(short, long, default_value = "synchrotron")]
        mode: String,

        /// Run in plain text log mode instead of the interactive clickable TUI control deck
        #[arg(long)]
        headless: bool,

        /// Suppress periodic console output in headless mode
        #[arg(short, long)]
        quiet: bool,

        /// Optional max runtime in seconds (automatically enables headless mode)
        #[arg(long)]
        duration_secs: Option<u64>,
    },

    /// Launch the separate TUI particle collider animation viewer (Window 2)
    Viz {
        /// Animation refresh rate in frames per second
        #[arg(short, long, default_value_t = 25)]
        fps: u64,
    },

    /// Query live telemetry from the running engine (or saved disk state if offline)
    Status,

    /// Tune accelerator parameters in real time over IPC ("how it accelerates")
    Tune {
        /// Set active CPU thread count
        #[arg(short, long)]
        threads: Option<usize>,

        /// Set CPU intensity percentage (5..=100%)
        #[arg(short, long)]
        intensity: Option<u8>,

        /// Switch acceleration mode (linac, synchrotron, plasma, quantum)
        #[arg(short, long)]
        mode: Option<String>,

        /// Adjust RF harmonic in GHz by a relative delta (e.g. 0.2 or -0.1)
        #[arg(long, allow_hyphen_values = true)]
        harmonic_delta: Option<f64>,

        /// Lock RF harmonic directly to the next target's resonant frequency
        #[arg(long)]
        auto_lock_rf: bool,

        /// Adjust quadrupole magnet focus in T/m by a relative delta (e.g. 0.15 or -0.1)
        #[arg(long, allow_hyphen_values = true)]
        focus_delta: Option<f64>,

        /// Fire a supercritical pulse burst from the capacitor
        #[arg(short, long)]
        burst: bool,
    },

    /// Display the complete 12-tier Quantum Discovery Codex (5σ)
    Codex,
}

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Engine {
        threads: 2,
        intensity: 70,
        mode: "synchrotron".into(),
        headless: false,
        quiet: false,
        duration_secs: None,
    }) {
        Commands::Engine {
            threads,
            intensity,
            mode,
            headless,
            quiet,
            duration_secs,
        } => {
            let parsed_mode = AccelerationMode::from_slug(&mode)
                .unwrap_or(AccelerationMode::SynchrotronHarmonic);
            run_engine_daemon(EngineConfig {
                host: cli.host,
                port: cli.port,
                initial_threads: threads,
                initial_intensity: intensity,
                initial_mode: parsed_mode,
                quiet,
                headless,
                duration_secs,
            })
        }
        Commands::Viz { fps } => run_tui_visualizer(VizConfig {
            host: cli.host,
            port: cli.port,
            fps,
        }),
        Commands::Status => {
            match send_command_once(&cli.host, cli.port, &EngineCommand::GetSnapshot) {
                Ok(snap) => print_status_report(&snap, true),
                Err(_) => {
                    let snap = build_offline_snapshot();
                    print_status_report(&snap, false);
                }
            }
            Ok(())
        }
        Commands::Tune {
            threads,
            intensity,
            mode,
            harmonic_delta,
            auto_lock_rf,
            focus_delta,
            burst,
        } => {
            let mut batch = Vec::new();
            if let Some(t) = threads {
                batch.push(EngineCommand::SetThreads(t));
            }
            if let Some(i) = intensity {
                batch.push(EngineCommand::SetIntensity(i));
            }
            if let Some(m) = mode {
                if let Some(parsed) = AccelerationMode::from_slug(&m) {
                    batch.push(EngineCommand::SetMode(parsed));
                }
            }
            if auto_lock_rf {
                batch.push(EngineCommand::LockOptimalHarmonic);
            }
            if let Some(dh) = harmonic_delta {
                batch.push(EngineCommand::AdjustRfHarmonic(dh));
            }
            if let Some(df) = focus_delta {
                batch.push(EngineCommand::AdjustMagnetFocus(df));
            }
            if burst {
                batch.push(EngineCommand::TriggerPulseBurst);
            }

            let cmd = if batch.is_empty() {
                EngineCommand::GetSnapshot
            } else {
                EngineCommand::Batch(batch)
            };

            match send_command_once(&cli.host, cli.port, &cmd) {
                Ok(snap) => {
                    println!("✓ Applied tuning commands to running engine at {}:{}.", cli.host, cli.port);
                    print_status_report(&snap, true);
                }
                Err(_) => {
                    println!(
                        "⚠ Engine daemon is not currently running on {}:{} (start it in Window 1 with `cargo run --release --bin collider-engine`).",
                        cli.host, cli.port
                    );
                    let snap = build_offline_snapshot();
                    print_status_report(&snap, false);
                }
            }
            Ok(())
        }
        Commands::Codex => {
            let (snap, online) =
                match send_command_once(&cli.host, cli.port, &EngineCommand::GetSnapshot) {
                    Ok(s) => (s, true),
                    Err(_) => (build_offline_snapshot(), false),
                };
            println!("╔══════════════════════════════════════════════════════════════════════════════╗");
            println!(
                "║  LOCAL COLLIDER :: 5σ QUANTUM DISCOVERY CODEX ({:<27}) ║",
                if online {
                    "LIVE ENGINE CONNECTION"
                } else {
                    "SAVED DISK STATE"
                }
            );
            println!("╚══════════════════════════════════════════════════════════════════════════════╝");
            for spec in QUANTUM_CATALOG {
                let found = snap.discoveries.iter().find(|d| d.id == spec.id);
                let status = if let Some(d) = found {
                    format!("★ DISCOVERED ({:.2}σ | x{})", d.best_sigma, d.count)
                } else {
                    "○ UNDETECTED".to_string()
                };
                println!(
                    "[Tier {}] {:<8} {:<34} | Req: {:>6.2} TeV | RF: {:.1} GHz | {}",
                    spec.tier,
                    spec.symbol,
                    spec.name,
                    spec.min_energy_tev,
                    spec.resonant_harmonic,
                    status
                );
                println!("         └─ {} ({})", spec.description, spec.category);
            }
            Ok(())
        }
    }
}

fn print_status_report(snap: &local_collider::protocol::TelemetrySnapshot, online: bool) {
    let badge = if online {
        format!("ONLINE :: Uptime {}s", snap.uptime_secs)
    } else {
        "OFFLINE :: Saved Disk Snapshot".to_string()
    };
    println!("╭──────────────────────────────────────────────────────────────────────────╮");
    println!(
        "│ LOCAL COLLIDER :: ENGINE TELEMETRY ({:<36}) │",
        badge
    );
    println!("├──────────────────────────────────────────────────────────────────────────┤");
    println!(
        "│ CPU Compute    : {:>8.1} kH/s | {:>6.2} GFLOPS | Threads: {}/{} @ {}%      │",
        snap.hashes_per_sec / 1000.0,
        snap.gflops,
        snap.active_threads,
        snap.max_threads,
        snap.intensity_pct
    );
    println!(
        "│ Accel. Mode    : {:<32} | Stability:   {:>5.1}% │",
        snap.mode.name(),
        snap.beam_stability_pct
    );
    println!(
        "│ RF & Magnets   : {:>5.2} GHz (Target {:>4.2} GHz | Align {:>5.1}%) | {:>4.2} T/m │",
        snap.rf_harmonic,
        snap.optimal_harmonic_hint,
        snap.harmonic_alignment_pct,
        snap.magnet_focus
    );
    println!(
        "│ Kinematics     : E = {:>8.3} TeV (Peak {:>8.3} TeV) | γ = {:>11.1}   │",
        snap.beam_energy_tev, snap.peak_energy_tev, snap.lorentz_gamma
    );
    println!(
        "│ Velocity (β)   : {:>5.2} nines of c | Collisions/s: {:>8.0} (Total: {}) │",
        snap.nines_of_c, snap.collisions_per_sec, snap.total_collisions
    );
    println!(
        "│ Quantum 5σ     : Proximity {:>5.1}% | Next: {:<8} ({:.2}σ / 5.00σ)       │",
        snap.quantum_proximity_pct, snap.next_target_symbol, snap.next_target_sigma_progress
    );
    println!(
        "│ Discoveries    : {:>2}/{} cataloged  | Pulse Capacitor: {:>5.1}%              │",
        snap.discoveries.len(),
        QUANTUM_CATALOG.len(),
        snap.capacitor_charge_pct
    );
    println!("╰──────────────────────────────────────────────────────────────────────────╯");
}
