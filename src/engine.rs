use crate::benchmark::CpuComputePool;
use crate::physics::{AcceleratorPhysicsState, QUANTUM_CATALOG};
use crate::protocol::{AccelerationMode, EngineCommand, TelemetrySnapshot};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::System;

pub struct EngineConfig {
    pub host: String,
    pub port: u16,
    pub initial_threads: usize,
    pub initial_intensity: u8,
    pub initial_mode: AccelerationMode,
    pub quiet: bool,
    pub duration_secs: Option<u64>,
}

struct SharedEngineContext {
    physics: AcceleratorPhysicsState,
    latest_snapshot: TelemetrySnapshot,
}

pub fn run_engine_daemon(cfg: EngineConfig) -> std::io::Result<()> {
    let bind_addr = format!("{}:{}", cfg.host, cfg.port);
    let listener = TcpListener::bind(&bind_addr)?;
    listener.set_nonblocking(true)?;

    let mut compute_pool = CpuComputePool::new(cfg.initial_threads, cfg.initial_intensity);
    let max_threads = compute_pool.max_threads();
    let mut physics = AcceleratorPhysicsState::load_or_new();
    physics.mode = cfg.initial_mode;

    let initial_snapshot = physics.step(
        0.1,
        10_000.0,
        0.5,
        compute_pool.entropy_seed(),
        0,
        compute_pool.active_threads(),
        max_threads,
        compute_pool.intensity_pct(),
        0.0,
        0,
        0,
    );

    let shared = Arc::new(Mutex::new(SharedEngineContext {
        physics,
        latest_snapshot: initial_snapshot,
    }));

    if !cfg.quiet {
        println!("╔══════════════════════════════════════════════════════════════════════════╗");
        println!("║  LOCAL COLLIDER :: MOTOR / DAEMON DE ACELERAÇÃO DE PARTÍCULAS (RUST)     ║");
        println!("╠══════════════════════════════════════════════════════════════════════════╣");
        println!(
            "║  IPC Server TCP : {:<54} ║",
            format!("{} (aguardando collider-viz / ctl)", bind_addr)
        );
        println!(
            "║  Threads Ativas : {}/{} núcleos | Intensidade CPU: {}%                   ║",
            compute_pool.active_threads(),
            max_threads,
            compute_pool.intensity_pct()
        );
        println!(
            "║  Dica           : Em outro terminal, rode `cargo run --bin collider-viz` ║"
        );
        println!("╚══════════════════════════════════════════════════════════════════════════╝");
    }

    let start_instant = Instant::now();
    let mut last_tick = Instant::now();
    let mut last_console_log = Instant::now();
    let mut last_save = Instant::now();
    let mut sys = System::new();
    let mut known_discoveries_count = {
        let guard = shared.lock().unwrap();
        guard.latest_snapshot.discoveries.len()
    };

    loop {
        if let Some(max_secs) = cfg.duration_secs {
            if start_instant.elapsed().as_secs() >= max_secs {
                if !cfg.quiet {
                    println!("\n[MOTOR] Tempo configurado ({max_secs}s) concluído. Salvando estado...");
                }
                let guard = shared.lock().unwrap();
                guard.physics.save_to_disk();
                break;
            }
        }

        // Aceita novos clientes TCP (collider-viz ou local-collider ctl)
        while let Ok((stream, _peer)) = listener.accept() {
            let shared_clone = Arc::clone(&shared);
            let pool_active = compute_pool.active_threads();
            let pool_max = compute_pool.max_threads();
            thread::spawn(move || {
                let _ = handle_client_connection(stream, shared_clone, pool_active, pool_max);
            });
        }

        thread::sleep(Duration::from_millis(100));

        let now = Instant::now();
        let dt = now.duration_since(last_tick).as_secs_f64().max(0.02);
        last_tick = now;

        sys.refresh_cpu_all();
        sys.refresh_memory();
        let system_cpu_pct = sys.global_cpu_usage();
        let memory_used_mb = sys.used_memory() / (1024 * 1024);

        let (hashes_per_sec, gflops, total_hashes) = compute_pool.sample_rates();
        let entropy = compute_pool.entropy_seed();
        let uptime_secs = start_instant.elapsed().as_secs();

        let snap = {
            let mut guard = shared.lock().unwrap();
            // Verifica se algum comando alterou threads/intensidade via pendência no snapshot
            let desired_threads = guard.latest_snapshot.active_threads;
            let desired_intensity = guard.latest_snapshot.intensity_pct;
            if desired_threads != compute_pool.active_threads() {
                compute_pool.set_active_threads(desired_threads);
            }
            if desired_intensity != compute_pool.intensity_pct() {
                compute_pool.set_intensity_pct(desired_intensity);
            }

            let snap = guard.physics.step(
                dt,
                hashes_per_sec,
                gflops,
                entropy,
                uptime_secs,
                compute_pool.active_threads(),
                max_threads,
                compute_pool.intensity_pct(),
                system_cpu_pct,
                memory_used_mb,
                total_hashes,
            );
            guard.latest_snapshot = snap.clone();
            snap
        };

        if !cfg.quiet && snap.discoveries.len() > known_discoveries_count {
            if let Some(newest) = snap.discoveries.last() {
                println!(
                    "\n★ [DESCOBERTA QUÂNTICA {:.2}σ] {} ({}) | Massa: {:.2} GeV | Energia: {:.2} TeV",
                    newest.best_sigma, newest.name, newest.symbol, newest.mass_gev, snap.beam_energy_tev
                );
                println!("  └─ {}", newest.description);
            }
            known_discoveries_count = snap.discoveries.len();
        }

        if !cfg.quiet && last_console_log.elapsed() >= Duration::from_secs(1) {
            last_console_log = Instant::now();
            println!(
                "[T+{:>4}s] CPU: {:>5.1}% ({}/{} thr @ {:>3}%) | {:>7.1} kH/s ({:>5.2} GFLOPS) | E: {:>8.2} TeV | v: 0.{}c (γ={:>9.1}) | Colisões/s: {:>8.0} | Quântico: {:>5.1}% ({}/{})",
                snap.uptime_secs,
                snap.system_cpu_pct,
                snap.active_threads,
                snap.max_threads,
                snap.intensity_pct,
                snap.hashes_per_sec / 1000.0,
                snap.gflops,
                snap.beam_energy_tev,
                format_nines_short(snap.nines_of_c),
                snap.lorentz_gamma,
                snap.collisions_per_sec,
                snap.quantum_proximity_pct,
                snap.discoveries.len(),
                QUANTUM_CATALOG.len()
            );
        }

        if last_save.elapsed() >= Duration::from_secs(10) {
            last_save = Instant::now();
            let guard = shared.lock().unwrap();
            guard.physics.save_to_disk();
        }
    }

    Ok(())
}

fn format_nines_short(nines: f64) -> String {
    let count = (nines.floor() as usize).clamp(2, 15);
    if count <= 6 {
        "9".repeat(count)
    } else {
        format!("999..[{} noves]", count)
    }
}

fn handle_client_connection(
    stream: TcpStream,
    shared: Arc<Mutex<SharedEngineContext>>,
    _initial_active: usize,
    max_threads: usize,
) -> std::io::Result<()> {
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    let mut writer = stream.try_clone()?;
    let reader = BufReader::new(stream);

    for line_res in reader.lines() {
        let line = match line_res {
            Ok(l) if !l.trim().is_empty() => l,
            Ok(_) => continue,
            Err(_) => break,
        };

        let cmd: EngineCommand = match serde_json::from_str(&line) {
            Ok(c) => c,
            Err(_) => EngineCommand::GetSnapshot,
        };

        let reply_snapshot = {
            let mut guard = shared.lock().unwrap();
            match cmd {
                EngineCommand::GetSnapshot => {}
                EngineCommand::SetThreads(n) => {
                    let clamped = n.clamp(1, max_threads);
                    guard.latest_snapshot.active_threads = clamped;
                    guard.physics.push_log(format!(
                        "Threads de cavidade RF ajustadas para {}/{}.",
                        clamped, max_threads
                    ));
                }
                EngineCommand::SetIntensity(pct) => {
                    let clamped = pct.clamp(5, 100);
                    guard.latest_snapshot.intensity_pct = clamped;
                    guard.physics.push_log(format!(
                        "Intensidade de processamento ajustada para {}%.",
                        clamped
                    ));
                }
                EngineCommand::SetMode(mode) => {
                    guard.physics.mode = mode;
                    guard.latest_snapshot.mode = mode;
                    guard.physics.push_log(format!(
                        "Modo de aceleração alterado para: {}.",
                        mode.name()
                    ));
                }
                EngineCommand::CycleMode => {
                    let next = guard.physics.mode.next();
                    guard.physics.mode = next;
                    guard.latest_snapshot.mode = next;
                    guard.physics.push_log(format!(
                        "Modo de aceleração alternado para: {}.",
                        next.name()
                    ));
                }
                EngineCommand::AdjustMagnetFocus(delta) => {
                    let new_focus = (guard.physics.magnet_focus + delta).clamp(0.2, 2.5);
                    guard.physics.magnet_focus = new_focus;
                    guard.latest_snapshot.magnet_focus = new_focus;
                    guard.physics.push_log(format!(
                        "Foco magnético quadrupolar ajustado para {:.2} T/m.",
                        new_focus
                    ));
                }
                EngineCommand::AdjustRfHarmonic(delta) => {
                    let new_harmonic = (guard.physics.rf_harmonic + delta).clamp(0.5, 5.0);
                    guard.physics.rf_harmonic = new_harmonic;
                    guard.latest_snapshot.rf_harmonic = new_harmonic;
                    guard.physics.push_log(format!(
                        "Harmônico RF sintonizado em {:.2} GHz.",
                        new_harmonic
                    ));
                }
                EngineCommand::TriggerPulseBurst => {
                    guard.physics.trigger_burst();
                    guard.latest_snapshot.capacitor_charge_pct =
                        guard.physics.capacitor_charge_pct;
                    guard.latest_snapshot.burst_ticks_remaining =
                        guard.physics.burst_ticks_remaining;
                }
                EngineCommand::ResetState => {
                    guard.physics.reset();
                }
            }
            guard.latest_snapshot.clone()
        };

        let mut payload = serde_json::to_string(&reply_snapshot).unwrap_or_default();
        payload.push('\n');
        writer.write_all(payload.as_bytes())?;
        writer.flush()?;
    }

    Ok(())
}

/// Envia um único comando ao motor em execução via TCP e retorna o TelemetrySnapshot atualizado.
pub fn send_command_once(
    host: &str,
    port: u16,
    cmd: &EngineCommand,
) -> std::io::Result<TelemetrySnapshot> {
    let addr = format!("{}:{}", host, port);
    let mut stream = TcpStream::connect_timeout(
        &addr.parse().map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("Endereço inválido: {e}"))
        })?,
        Duration::from_millis(1500),
    )?;
    stream.set_read_timeout(Some(Duration::from_millis(1500)))?;
    stream.set_write_timeout(Some(Duration::from_millis(1500)))?;

    let mut payload = serde_json::to_string(cmd)?;
    payload.push('\n');
    stream.write_all(payload.as_bytes())?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line)?;
    let snap: TelemetrySnapshot = serde_json::from_str(&response_line)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(snap)
}
