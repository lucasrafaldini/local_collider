use clap::{Parser, Subcommand};
use local_collider::engine::{run_engine_daemon, send_command_once, EngineConfig};
use local_collider::physics::QUANTUM_CATALOG;
use local_collider::protocol::{AccelerationMode, EngineCommand, DEFAULT_HOST, DEFAULT_PORT};
use local_collider::viz::{run_tui_visualizer, VizConfig};

#[derive(Parser)]
#[command(
    name = "local-collider",
    version,
    about = "Simulador de acelerador de partículas em CLI que troca processamento real de CPU por velocidade relativística e descobertas quânticas."
)]
struct Cli {
    /// Host do servidor IPC do motor
    #[arg(long, global = true, default_value = DEFAULT_HOST)]
    host: String,

    /// Porta TCP do servidor IPC do motor
    #[arg(short, long, global = true, default_value_t = DEFAULT_PORT)]
    port: u16,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Inicia o motor/daemon de aceleração (processo 1: consome CPU e calcula colisões quânticas)
    Engine {
        /// Número inicial de threads de CPU dedicadas às cavidades RF
        #[arg(short, long, default_value_t = 2)]
        threads: usize,

        /// Intensidade de uso de CPU por thread (5 a 100%)
        #[arg(short, long, default_value_t = 70)]
        intensity: u8,

        /// Modo de aceleração inicial (linac, synchrotron, plasma, quantum)
        #[arg(short, long, default_value = "synchrotron")]
        mode: String,

        /// Suprime logs periódicos no terminal do motor
        #[arg(short, long)]
        quiet: bool,

        /// Tempo máximo opcional de execução em segundos (útil para benchmarks/testes)
        #[arg(long)]
        duration_secs: Option<u64>,
    },

    /// Inicia o visualizador de animação TUI à parte (processo 2: conecta ao motor via TCP)
    Viz {
        /// Taxa de quadros por segundo da animação em terminal
        #[arg(short, long, default_value_t = 25)]
        fps: u64,
    },

    /// Consulta o status instantâneo do motor em execução via CLI
    Status,

    /// Ajusta parâmetros do motor em tempo real via linha de comando ("como acelera")
    Tune {
        /// Define o número de threads ativas
        #[arg(short, long)]
        threads: Option<usize>,

        /// Define a intensidade de CPU (5..=100%)
        #[arg(short, long)]
        intensity: Option<u8>,

        /// Altera o modo de aceleração (linac, synchrotron, plasma, quantum)
        #[arg(short, long)]
        mode: Option<String>,

        /// Ajusta o harmônico RF em GHz (delta relativo, ex: 0.2 ou -0.1)
        #[arg(long, allow_hyphen_values = true)]
        harmonic_delta: Option<f64>,

        /// Ajusta o foco magnético quadrupolar em T/m (delta relativo, ex: 0.15 ou -0.1)
        #[arg(long, allow_hyphen_values = true)]
        focus_delta: Option<f64>,

        /// Dispara um pulso supercrítico do capacitor
        #[arg(short, long)]
        burst: bool,
    },

    /// Lista o Códice Quântico completo (12 partículas e anomalias) e seu progresso
    Codex,
}

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Engine {
        threads: 2,
        intensity: 70,
        mode: "synchrotron".into(),
        quiet: false,
        duration_secs: None,
    }) {
        Commands::Engine {
            threads,
            intensity,
            mode,
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
                duration_secs,
            })
        }
        Commands::Viz { fps } => run_tui_visualizer(VizConfig {
            host: cli.host,
            port: cli.port,
            fps,
        }),
        Commands::Status => {
            let snap = send_command_once(&cli.host, cli.port, &EngineCommand::GetSnapshot)?;
            print_status_report(&snap);
            Ok(())
        }
        Commands::Tune {
            threads,
            intensity,
            mode,
            harmonic_delta,
            focus_delta,
            burst,
        } => {
            let mut latest = None;
            if let Some(t) = threads {
                latest = Some(send_command_once(
                    &cli.host,
                    cli.port,
                    &EngineCommand::SetThreads(t),
                )?);
            }
            if let Some(i) = intensity {
                latest = Some(send_command_once(
                    &cli.host,
                    cli.port,
                    &EngineCommand::SetIntensity(i),
                )?);
            }
            if let Some(m) = mode {
                if let Some(parsed) = AccelerationMode::from_slug(&m) {
                    latest = Some(send_command_once(
                        &cli.host,
                        cli.port,
                        &EngineCommand::SetMode(parsed),
                    )?);
                }
            }
            if let Some(dh) = harmonic_delta {
                latest = Some(send_command_once(
                    &cli.host,
                    cli.port,
                    &EngineCommand::AdjustRfHarmonic(dh),
                )?);
            }
            if let Some(df) = focus_delta {
                latest = Some(send_command_once(
                    &cli.host,
                    cli.port,
                    &EngineCommand::AdjustMagnetFocus(df),
                )?);
            }
            if burst {
                latest = Some(send_command_once(
                    &cli.host,
                    cli.port,
                    &EngineCommand::TriggerPulseBurst,
                )?);
            }
            let snap = match latest {
                Some(s) => s,
                None => send_command_once(&cli.host, cli.port, &EngineCommand::GetSnapshot)?,
            };
            print_status_report(&snap);
            Ok(())
        }
        Commands::Codex => {
            let snap_opt = send_command_once(&cli.host, cli.port, &EngineCommand::GetSnapshot).ok();
            println!("╔══════════════════════════════════════════════════════════════════════════════╗");
            println!("║  LOCAL COLLIDER :: CÓDICE DE PARTÍCULAS E ANOMALIAS QUÂNTICAS (5σ)           ║");
            println!("╚══════════════════════════════════════════════════════════════════════════════╝");
            for spec in QUANTUM_CATALOG {
                let found = snap_opt
                    .as_ref()
                    .and_then(|s| s.discoveries.iter().find(|d| d.id == spec.id));
                let status = if let Some(d) = found {
                    format!("★ DESCOBERTO ({:.2}σ | x{})", d.best_sigma, d.count)
                } else {
                    "○ NÃO DETECTADO".to_string()
                };
                println!(
                    "[Tier {}] {:<8} {:<34} | Limiar: {:>6.2} TeV | Harmônico: {:.1} GHz | {}",
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

fn print_status_report(snap: &local_collider::protocol::TelemetrySnapshot) {
    println!("╭──────────────────────────────────────────────────────────────────────────╮");
    println!(
        "│ LOCAL COLLIDER :: TELEMETRIA DO MOTOR (Uptime: {:>5}s)                   │",
        snap.uptime_secs
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
        "│ Como Acelera   : {:<32} | Estabilidade: {:>5.1}% │",
        snap.mode.name(),
        snap.beam_stability_pct
    );
    println!(
        "│ Sintonia RF    : {:>5.2} GHz (Alvo: {:>4.2} GHz | Sintonia: {:>5.1}%) | Ímã: {:.2}T/m│",
        snap.rf_harmonic,
        snap.optimal_harmonic_hint,
        snap.harmonic_alignment_pct,
        snap.magnet_focus
    );
    println!(
        "│ Cinemática     : E = {:>8.3} TeV (Pico {:>8.3} TeV) | γ = {:>11.1}   │",
        snap.beam_energy_tev, snap.peak_energy_tev, snap.lorentz_gamma
    );
    println!(
        "│ Velocidade (β) : {:>5.2} noves de c | Colisões/s: {:>9.0} (Total: {}) │",
        snap.nines_of_c, snap.collisions_per_sec, snap.total_collisions
    );
    println!(
        "│ Quântico       : Proximidade {:>5.1}% | Alvo: {} ({:.2}σ / 5.00σ)   │",
        snap.quantum_proximity_pct, snap.next_target_symbol, snap.next_target_sigma_progress
    );
    println!(
        "│ Descobertas    : {}/{} catalogadas | Capacitor de Pulso: {:>5.1}%           │",
        snap.discoveries.len(),
        QUANTUM_CATALOG.len(),
        snap.capacitor_charge_pct
    );
    println!("╰──────────────────────────────────────────────────────────────────────────╯");
}
