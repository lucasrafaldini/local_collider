use clap::Parser;
use local_collider::engine::{run_engine_daemon, EngineConfig};
use local_collider::protocol::{AccelerationMode, DEFAULT_HOST, DEFAULT_PORT};

#[derive(Parser)]
#[command(
    name = "collider-engine",
    version,
    about = "Window 1 Engine Daemon & Clickable Control Deck for Local Collider: converts real CPU compute into particle acceleration and quantum discoveries."
)]
struct Args {
    #[arg(long, default_value = DEFAULT_HOST)]
    host: String,

    #[arg(short, long, default_value_t = DEFAULT_PORT)]
    port: u16,

    /// Initial number of CPU threads dedicated to acceleration
    #[arg(short, long, default_value_t = 2)]
    threads: usize,

    /// CPU processing intensity per thread (5 to 100%)
    #[arg(short, long, default_value_t = 70)]
    intensity: u8,

    /// Initial acceleration mode (linac, synchrotron, plasma, quantum)
    #[arg(short, long, default_value = "synchrotron")]
    mode: String,

    /// Run in plain text stdout mode instead of the interactive mouse-clickable TUI deck
    #[arg(long)]
    headless: bool,

    /// Suppress periodic stdout logs in headless mode
    #[arg(short, long)]
    quiet: bool,

    /// Run for N seconds and exit (automatically uses headless mode)
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
        headless: args.headless,
        duration_secs: args.duration_secs,
    })
}
