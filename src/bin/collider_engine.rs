use clap::Parser;
use local_collider::engine::{run_engine_daemon, EngineConfig};
use local_collider::protocol::{AccelerationMode, DEFAULT_HOST, DEFAULT_PORT};

#[derive(Parser)]
#[command(
    name = "collider-engine",
    version,
    about = "Processo Daemon/Motor do Local Collider: converte processamento real de CPU em aceleração de partículas e descobertas quânticas."
)]
struct Args {
    #[arg(long, default_value = DEFAULT_HOST)]
    host: String,

    #[arg(short, long, default_value_t = DEFAULT_PORT)]
    port: u16,

    /// Número de threads de CPU dedicadas à aceleração
    #[arg(short, long, default_value_t = 2)]
    threads: usize,

    /// Intensidade de processamento de CPU por thread (5 a 100%)
    #[arg(short, long, default_value_t = 70)]
    intensity: u8,

    /// Modo de aceleração inicial (linac, synchrotron, plasma, quantum)
    #[arg(short, long, default_value = "synchrotron")]
    mode: String,

    /// Oculta saída periódica no console
    #[arg(short, long)]
    quiet: bool,

    /// Executa apenas por N segundos e encerra
    #[arg(long)]
    duration_secs: Option<u64>,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let initial_mode =
        AccelerationMode::from_slug(&args.mode).unwrap_or(AccelerationMode::SynchrotronHarmonic);

    run_engine_daemon(EngineConfig {
        host: args.host,
        port: args.port,
        initial_threads: args.threads,
        initial_intensity: args.intensity,
        initial_mode,
        quiet: args.quiet,
        duration_secs: args.duration_secs,
    })
}
