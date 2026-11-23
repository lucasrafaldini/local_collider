use clap::Parser;
use local_collider::protocol::{DEFAULT_HOST, DEFAULT_PORT};
use local_collider::viz::{run_tui_visualizer, VizConfig};

#[derive(Parser)]
#[command(
    name = "collider-viz",
    version,
    about = "Processo Visualizador TUI à parte do Local Collider: conecta ao collider-engine via TCP e renderiza o anel síncrotron e colisões em tempo real."
)]
struct Args {
    #[arg(long, default_value = DEFAULT_HOST)]
    host: String,

    #[arg(short, long, default_value_t = DEFAULT_PORT)]
    port: u16,

    /// Taxa de atualização da animação em quadros por segundo (FPS)
    #[arg(short, long, default_value_t = 25)]
    fps: u64,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    run_tui_visualizer(VizConfig {
        host: args.host,
        port: args.port,
        fps: args.fps,
    })
}
