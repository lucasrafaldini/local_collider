use crate::benchmark::CpuComputePool;
use crate::physics::{AcceleratorPhysicsState, QUANTUM_CATALOG};
use crate::protocol::{AccelerationMode, EngineCommand, TelemetrySnapshot};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseButton,
        MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::io::{self, BufRead, BufReader, IsTerminal, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
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
    pub headless: bool,
    pub duration_secs: Option<u64>,
}

struct SharedEngineContext {
    physics: AcceleratorPhysicsState,
    latest_snapshot: TelemetrySnapshot,
    max_threads: usize,
}

#[derive(Clone)]
enum DeckAction {
    Command(EngineCommand),
    Quit,
}

#[derive(Clone)]
struct HitBox {
    rect: Rect,
    action: DeckAction,
}

pub fn run_engine_daemon(cfg: EngineConfig) -> io::Result<()> {
    let bind_addr = format!("{}:{}", cfg.host, cfg.port);
    let listener = TcpListener::bind(&bind_addr)?;

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
        max_threads,
    }));

    let running = Arc::new(AtomicBool::new(true));

    // Dedicated instant TCP IPC Acceptor thread (0ms accept latency for collider-viz and CLI commands)
    {
        let shared_ipc = Arc::clone(&shared);
        let running_ipc = Arc::clone(&running);
        thread::Builder::new()
            .name("collider-ipc-listener".into())
            .spawn(move || {
                for stream_res in listener.incoming() {
                    if !running_ipc.load(Ordering::Relaxed) {
                        break;
                    }
                    if let Ok(stream) = stream_res {
                        let ctx = Arc::clone(&shared_ipc);
                        thread::spawn(move || {
                            let _ = handle_client_connection(stream, ctx);
                        });
                    }
                }
            })
            .expect("Failed to start IPC TCP listener thread");
    }

    // Decide whether to launch the interactive Clickable Control Deck TUI or Headless Console mode
    let use_interactive_tui =
        !cfg.headless && !cfg.quiet && cfg.duration_secs.is_none() && io::stdout().is_terminal();

    if use_interactive_tui {
        run_interactive_engine_deck(&cfg, &bind_addr, &mut compute_pool, &shared, &running)
    } else {
        run_headless_engine_loop(&cfg, &bind_addr, &mut compute_pool, &shared, &running)
    }
}

fn step_engine_physics(
    dt: f64,
    start_instant: Instant,
    sys: &mut System,
    compute_pool: &mut CpuComputePool,
    shared: &Arc<Mutex<SharedEngineContext>>,
) -> TelemetrySnapshot {
    sys.refresh_cpu_all();
    sys.refresh_memory();
    let system_cpu_pct = sys.global_cpu_usage();
    let memory_used_mb = sys.used_memory() / (1024 * 1024);

    let (hashes_per_sec, gflops, total_hashes) = compute_pool.sample_rates();
    let entropy = compute_pool.entropy_seed();
    let uptime_secs = start_instant.elapsed().as_secs();
    let max_threads = compute_pool.max_threads();

    let mut guard = shared.lock().unwrap();
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
}

fn apply_command_locked(guard: &mut SharedEngineContext, cmd: EngineCommand) {
    let max_threads = guard.max_threads;
    match cmd {
        EngineCommand::GetSnapshot => {}
        EngineCommand::SetThreads(n) => {
            let clamped = n.clamp(1, max_threads);
            guard.latest_snapshot.active_threads = clamped;
            guard.physics.push_log(format!(
                "RF cavity CPU threads adjusted to {}/{}.",
                clamped, max_threads
            ));
        }
        EngineCommand::SetIntensity(pct) => {
            let clamped = pct.clamp(5, 100);
            guard.latest_snapshot.intensity_pct = clamped;
            guard
                .physics
                .push_log(format!("CPU compute intensity set to {}%.", clamped));
        }
        EngineCommand::SetMode(mode) => {
            guard.physics.mode = mode;
            guard.latest_snapshot.mode = mode;
            guard
                .physics
                .push_log(format!("Acceleration mode switched to: {}.", mode.name()));
        }
        EngineCommand::CycleMode => {
            let next = guard.physics.mode.next();
            guard.physics.mode = next;
            guard.latest_snapshot.mode = next;
            guard
                .physics
                .push_log(format!("Acceleration mode cycled to: {}.", next.name()));
        }
        EngineCommand::AdjustMagnetFocus(delta) => {
            let new_focus = (guard.physics.magnet_focus + delta).clamp(0.2, 2.5);
            guard.physics.magnet_focus = new_focus;
            guard.latest_snapshot.magnet_focus = new_focus;
            guard.physics.push_log(format!(
                "Quadrupole magnet focus tuned to {:.2} T/m.",
                new_focus
            ));
        }
        EngineCommand::SetMagnetFocus(val) => {
            let new_focus = val.clamp(0.2, 2.5);
            guard.physics.magnet_focus = new_focus;
            guard.latest_snapshot.magnet_focus = new_focus;
            guard.physics.push_log(format!(
                "Quadrupole magnet focus locked to {:.2} T/m.",
                new_focus
            ));
        }
        EngineCommand::AdjustRfHarmonic(delta) => {
            let new_harmonic = (guard.physics.rf_harmonic + delta).clamp(0.5, 5.0);
            guard.physics.rf_harmonic = new_harmonic;
            guard.latest_snapshot.rf_harmonic = new_harmonic;
            guard.physics.push_log(format!(
                "RF cavity harmonic tuned to {:.2} GHz.",
                new_harmonic
            ));
        }
        EngineCommand::SetRfHarmonic(val) => {
            let new_harmonic = val.clamp(0.5, 5.0);
            guard.physics.rf_harmonic = new_harmonic;
            guard.latest_snapshot.rf_harmonic = new_harmonic;
            guard.physics.push_log(format!(
                "RF cavity harmonic locked to {:.2} GHz.",
                new_harmonic
            ));
        }
        EngineCommand::LockOptimalHarmonic => {
            guard.physics.lock_optimal_harmonic();
            guard.latest_snapshot.rf_harmonic = guard.physics.rf_harmonic;
        }
        EngineCommand::TriggerPulseBurst => {
            guard.physics.trigger_burst();
            guard.latest_snapshot.capacitor_charge_pct = guard.physics.capacitor_charge_pct;
            guard.latest_snapshot.burst_ticks_remaining = guard.physics.burst_ticks_remaining;
        }
        EngineCommand::ResetState => {
            guard.physics.reset();
        }
        EngineCommand::Batch(cmds) => {
            for sub in cmds {
                apply_command_locked(guard, sub);
            }
        }
    }
}

/// Interactive Clickable TUI Control Deck for Window 1 (`collider-engine`)
fn run_interactive_engine_deck(
    _cfg: &EngineConfig,
    bind_addr: &str,
    compute_pool: &mut CpuComputePool,
    shared: &Arc<Mutex<SharedEngineContext>>,
    running: &Arc<AtomicBool>,
) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let start_instant = Instant::now();
    let mut last_tick = Instant::now();
    let mut last_save = Instant::now();
    let mut sys = System::new();
    let mut hitboxes: Vec<HitBox> = Vec::new();
    let mut snap = { shared.lock().unwrap().latest_snapshot.clone() };

    let res: io::Result<()> = (|| {
        while running.load(Ordering::Relaxed) {
            if last_tick.elapsed() >= Duration::from_millis(100) {
                let now = Instant::now();
                let dt = now.duration_since(last_tick).as_secs_f64().max(0.02);
                last_tick = now;
                snap = step_engine_physics(dt, start_instant, &mut sys, compute_pool, shared);
            }

            if last_save.elapsed() >= Duration::from_secs(10) {
                last_save = Instant::now();
                shared.lock().unwrap().physics.save_to_disk();
            }

            hitboxes.clear();
            terminal.draw(|f| {
                draw_clickable_engine_deck(f, &snap, bind_addr, &mut hitboxes);
            })?;

            if event::poll(Duration::from_millis(35))? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                            running.store(false, Ordering::Relaxed);
                            break;
                        }
                        KeyCode::Char('m') | KeyCode::Char('M') => {
                            let mut g = shared.lock().unwrap();
                            apply_command_locked(&mut g, EngineCommand::CycleMode);
                        }
                        KeyCode::Char('+') | KeyCode::Char('=') => {
                            let mut g = shared.lock().unwrap();
                            let n = g.latest_snapshot.active_threads + 1;
                            apply_command_locked(&mut g, EngineCommand::SetThreads(n));
                        }
                        KeyCode::Char('-') | KeyCode::Char('_') => {
                            let mut g = shared.lock().unwrap();
                            let n = g.latest_snapshot.active_threads.saturating_sub(1).max(1);
                            apply_command_locked(&mut g, EngineCommand::SetThreads(n));
                        }
                        KeyCode::Char(']') => {
                            let mut g = shared.lock().unwrap();
                            let i = g.latest_snapshot.intensity_pct.saturating_add(10).min(100);
                            apply_command_locked(&mut g, EngineCommand::SetIntensity(i));
                        }
                        KeyCode::Char('[') => {
                            let mut g = shared.lock().unwrap();
                            let i = g.latest_snapshot.intensity_pct.saturating_sub(10).max(10);
                            apply_command_locked(&mut g, EngineCommand::SetIntensity(i));
                        }
                        KeyCode::Up => {
                            let mut g = shared.lock().unwrap();
                            apply_command_locked(&mut g, EngineCommand::AdjustRfHarmonic(0.1));
                        }
                        KeyCode::Down => {
                            let mut g = shared.lock().unwrap();
                            apply_command_locked(&mut g, EngineCommand::AdjustRfHarmonic(-0.1));
                        }
                        KeyCode::Right => {
                            let mut g = shared.lock().unwrap();
                            apply_command_locked(&mut g, EngineCommand::AdjustMagnetFocus(0.1));
                        }
                        KeyCode::Left => {
                            let mut g = shared.lock().unwrap();
                            apply_command_locked(&mut g, EngineCommand::AdjustMagnetFocus(-0.1));
                        }
                        KeyCode::Char('a') | KeyCode::Char('A') => {
                            let mut g = shared.lock().unwrap();
                            apply_command_locked(&mut g, EngineCommand::LockOptimalHarmonic);
                        }
                        KeyCode::Char(' ') | KeyCode::Char('b') | KeyCode::Char('B') => {
                            let mut g = shared.lock().unwrap();
                            apply_command_locked(&mut g, EngineCommand::TriggerPulseBurst);
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            let mut g = shared.lock().unwrap();
                            apply_command_locked(&mut g, EngineCommand::ResetState);
                        }
                        _ => {}
                    },
                    Event::Mouse(mouse) => {
                        if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                            let col = mouse.column;
                            let row = mouse.row;
                            for hb in &hitboxes {
                                if rect_contains(hb.rect, col, row) {
                                    match &hb.action {
                                        DeckAction::Command(cmd) => {
                                            let mut g = shared.lock().unwrap();
                                            apply_command_locked(&mut g, cmd.clone());
                                            snap = g.latest_snapshot.clone();
                                        }
                                        DeckAction::Quit => {
                                            running.store(false, Ordering::Relaxed);
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    })();

    shared.lock().unwrap().physics.save_to_disk();
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    res
}

fn rect_contains(r: Rect, col: u16, row: u16) -> bool {
    col >= r.x && col < r.x.saturating_add(r.width) && row >= r.y && row < r.y.saturating_add(r.height)
}

fn render_button(
    f: &mut Frame,
    area: Rect,
    label: &str,
    fg: Color,
    bg: Color,
    bold: bool,
    action: DeckAction,
    hitboxes: &mut Vec<HitBox>,
) {
    let mut style = Style::default().fg(fg).bg(bg);
    if bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(fg));
    let p = Paragraph::new(Line::from(Span::styled(label, style)))
        .alignment(Alignment::Center)
        .block(block);
    f.render_widget(p, area);
    hitboxes.push(HitBox { rect: area, action });
}

fn draw_clickable_engine_deck(
    f: &mut Frame,
    snap: &TelemetrySnapshot,
    bind_addr: &str,
    hitboxes: &mut Vec<HitBox>,
) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Top header + Quit button
            Constraint::Length(5), // Telemetry summary banner
            Constraint::Length(3), // Clickable Row 1: CPU Threads & Intensity buttons
            Constraint::Length(3), // Clickable Row 2: 4 Acceleration Mode buttons
            Constraint::Length(3), // Clickable Row 3: RF Harmonic & Magnet Focus buttons
            Constraint::Length(3), // Clickable Row 4: Supercritical Burst & Auto-Lock buttons
            Constraint::Min(7),    // Clickable Quantum Codex + Live Event Logs
        ])
        .split(f.area());

    // 1. Header bar with clickable [ QUIT ] and [ RESET ] buttons
    let header_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(30),
            Constraint::Length(14),
            Constraint::Length(12),
        ])
        .split(root[0]);

    let title_line = Line::from(vec![
        Span::styled(
            " ⚛ LOCAL COLLIDER :: ENGINE CONTROL DECK (MOUSE CLICKABLE) ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("| IPC {} ", bind_addr),
            Style::default().fg(Color::Green),
        ),
        Span::raw(format!("| Uptime: {}s", snap.uptime_secs)),
    ]);
    f.render_widget(
        Paragraph::new(title_line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        ),
        header_cols[0],
    );

    render_button(
        f,
        header_cols[1],
        "↺ RESET [R]",
        Color::Yellow,
        Color::Black,
        true,
        DeckAction::Command(EngineCommand::ResetState),
        hitboxes,
    );
    render_button(
        f,
        header_cols[2],
        "✕ QUIT [Q]",
        Color::LightRed,
        Color::Black,
        true,
        DeckAction::Quit,
        hitboxes,
    );

    // 2. Telemetry Summary Banner
    let nines_count = (snap.nines_of_c.floor() as usize).clamp(2, 12);
    let summary_text = vec![
        Line::from(vec![
            Span::styled(
                " CPU COMPUTE : ",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:.1} kH/s", snap.hashes_per_sec / 1000.0),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                " ({:.2} GFLOPS) | Active Cores: {}/{} @ {}% Intensity | Sys CPU: {:.1}% | RAM: {} MB",
                snap.gflops,
                snap.active_threads,
                snap.max_threads,
                snap.intensity_pct,
                snap.system_cpu_pct,
                snap.memory_used_mb
            )),
        ]),
        Line::from(vec![
            Span::styled(
                " KINEMATICS  : ",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("E = {:.3} TeV", snap.beam_energy_tev),
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                " (Peak: {:.3} TeV) | v = 0.{}c ({:.2} nines) | Lorentz γ = {:.1} | Stability: {:.1}%",
                snap.peak_energy_tev,
                "9".repeat(nines_count),
                snap.nines_of_c,
                snap.lorentz_gamma,
                snap.beam_stability_pct
            )),
        ]),
        Line::from(vec![
            Span::styled(
                " QUANTUM 5σ  : ",
                Style::default()
                    .fg(Color::LightMagenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "Proximity: {:.1}% | Next: {} ({}) [{:.2}σ / 5.00σ @ {:.2} TeV] | Collisions/s: {:.0} | Run `cargo run --bin collider-viz` in Window 2!",
                snap.quantum_proximity_pct,
                snap.next_target_name,
                snap.next_target_symbol,
                snap.next_target_sigma_progress,
                snap.next_target_min_tev,
                snap.collisions_per_sec
            )),
        ]),
    ];
    f.render_widget(
        Paragraph::new(summary_text).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Live Hardware & Relativistic Beam Telemetry "),
        ),
        root[1],
    );

    // 3. Clickable Row 1: CPU Threads & Intensity Buttons
    let row1_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(14),
            Constraint::Percentage(14),
            Constraint::Percentage(16),
            Constraint::Percentage(14),
            Constraint::Percentage(14),
            Constraint::Percentage(14),
            Constraint::Percentage(14),
        ])
        .split(root[2]);

    render_button(
        f,
        row1_cols[0],
        "[-1 THREAD]",
        Color::Cyan,
        Color::Black,
        true,
        DeckAction::Command(EngineCommand::SetThreads(
            snap.active_threads.saturating_sub(1).max(1),
        )),
        hitboxes,
    );
    render_button(
        f,
        row1_cols[1],
        "[+1 THREAD]",
        Color::Cyan,
        Color::Black,
        true,
        DeckAction::Command(EngineCommand::SetThreads(snap.active_threads + 1)),
        hitboxes,
    );
    render_button(
        f,
        row1_cols[2],
        &format!("[MAX {} CORES]", snap.max_threads),
        Color::LightCyan,
        Color::Black,
        true,
        DeckAction::Command(EngineCommand::SetThreads(snap.max_threads)),
        hitboxes,
    );

    for (i, pct) in [25u8, 50, 75, 100].iter().enumerate() {
        let active = (snap.intensity_pct as i16 - *pct as i16).abs() < 8;
        render_button(
            f,
            row1_cols[3 + i],
            &format!("[CPU {}%]", pct),
            if active { Color::Black } else { Color::Green },
            if active { Color::Green } else { Color::Black },
            true,
            DeckAction::Command(EngineCommand::SetIntensity(*pct)),
            hitboxes,
        );
    }

    // 4. Clickable Row 2: 4 Acceleration Modes ("How It Accelerates")
    let row2_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(root[3]);

    for (idx, mode) in AccelerationMode::all().iter().enumerate() {
        let is_selected = snap.mode == *mode;
        let label = format!(
            "{}Mode {}: {}",
            if is_selected { "★ " } else { "" },
            idx + 1,
            mode.name()
        );
        render_button(
            f,
            row2_cols[idx],
            &label,
            if is_selected {
                Color::Black
            } else {
                Color::Yellow
            },
            if is_selected {
                Color::Yellow
            } else {
                Color::Black
            },
            true,
            DeckAction::Command(EngineCommand::SetMode(*mode)),
            hitboxes,
        );
    }

    // 5. Clickable Row 3: RF Harmonic & Quadrupole Magnet Focus Controls
    let row3_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(14),
            Constraint::Percentage(18),
            Constraint::Percentage(14),
            Constraint::Percentage(14),
            Constraint::Percentage(26),
            Constraint::Percentage(14),
        ])
        .split(root[4]);

    render_button(
        f,
        row3_cols[0],
        "[RF -0.1 GHz]",
        Color::LightCyan,
        Color::Black,
        false,
        DeckAction::Command(EngineCommand::AdjustRfHarmonic(-0.1)),
        hitboxes,
    );
    render_button(
        f,
        row3_cols[1],
        &format!(
            "RF {:.2}/{:.2}GHz ({:.0}%)",
            snap.rf_harmonic, snap.optimal_harmonic_hint, snap.harmonic_alignment_pct
        ),
        Color::White,
        Color::Black,
        true,
        DeckAction::Command(EngineCommand::LockOptimalHarmonic),
        hitboxes,
    );
    render_button(
        f,
        row3_cols[2],
        "[RF +0.1 GHz]",
        Color::LightCyan,
        Color::Black,
        false,
        DeckAction::Command(EngineCommand::AdjustRfHarmonic(0.1)),
        hitboxes,
    );
    render_button(
        f,
        row3_cols[3],
        "[MAG -0.1 T/m]",
        Color::Magenta,
        Color::Black,
        false,
        DeckAction::Command(EngineCommand::AdjustMagnetFocus(-0.1)),
        hitboxes,
    );
    render_button(
        f,
        row3_cols[4],
        &format!("[MAG {:.2} T/m :: SET 1.45 OPTIMAL]", snap.magnet_focus),
        Color::LightMagenta,
        Color::Black,
        true,
        DeckAction::Command(EngineCommand::SetMagnetFocus(1.45)),
        hitboxes,
    );
    render_button(
        f,
        row3_cols[5],
        "[MAG +0.1 T/m]",
        Color::Magenta,
        Color::Black,
        false,
        DeckAction::Command(EngineCommand::AdjustMagnetFocus(0.1)),
        hitboxes,
    );

    // 6. Clickable Row 4: Auto-Lock Resonance & Supercritical Capacitor Pulse Burst
    let row4_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(36), Constraint::Percentage(64)])
        .split(root[5]);

    let aligned = snap.harmonic_alignment_pct >= 95.0;
    render_button(
        f,
        row4_cols[0],
        &format!(
            "[ 🎯 AUTO-LOCK RF TO {} ({:.1} GHz) ]",
            snap.next_target_symbol, snap.optimal_harmonic_hint
        ),
        if aligned { Color::LightGreen } else { Color::Yellow },
        Color::Black,
        true,
        DeckAction::Command(EngineCommand::LockOptimalHarmonic),
        hitboxes,
    );

    let burst_ready = snap.capacitor_charge_pct >= 25.0;
    let burst_label = if snap.burst_ticks_remaining > 0 {
        format!(
            "⚡ SUPERCRITICAL OVERDRIVE ACTIVE ({:.1}s LEFT :: 3.25x TeV & QUANTUM) ⚡",
            snap.burst_ticks_remaining as f64 * 0.1
        )
    } else if burst_ready {
        format!(
            "[ ⚡ CLICK TO FIRE SUPERCRITICAL PULSE BURST ({:.0}% CHARGED) ⚡ ]",
            snap.capacitor_charge_pct
        )
    } else {
        format!(
            "[ CHARGING PULSE CAPACITOR: {:.1}% / 25% MINIMUM ]",
            snap.capacitor_charge_pct
        )
    };
    render_button(
        f,
        row4_cols[1],
        &burst_label,
        if snap.burst_ticks_remaining > 0 {
            Color::Black
        } else if burst_ready {
            Color::LightGreen
        } else {
            Color::DarkGray
        },
        if snap.burst_ticks_remaining > 0 {
            Color::LightRed
        } else {
            Color::Black
        },
        true,
        DeckAction::Command(EngineCommand::TriggerPulseBurst),
        hitboxes,
    );

    // 7. Bottom Split: Clickable Quantum Codex (left) & Live Detector Logs (right)
    let bottom_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(root[6]);

    let codex_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::LightMagenta))
        .title(format!(
            " Clickable Quantum Codex ({}/{} Discovered — Click Any Row to Tune RF & Mode!) ",
            snap.discoveries.len(),
            QUANTUM_CATALOG.len()
        ));
    let codex_inner = codex_block.inner(bottom_cols[0]);
    f.render_widget(codex_block, bottom_cols[0]);

    let max_visible_rows = codex_inner.height as usize;
    for (idx, spec) in QUANTUM_CATALOG.iter().take(max_visible_rows).enumerate() {
        let row_rect = Rect {
            x: codex_inner.x,
            y: codex_inner.y + idx as u16,
            width: codex_inner.width,
            height: 1,
        };
        let is_tuned = (snap.rf_harmonic - spec.resonant_harmonic).abs() < 0.06;
        let line = if let Some(found) = snap.discoveries.iter().find(|d| d.id == spec.id) {
            Line::from(vec![
                Span::styled(
                    format!(
                        "{}★ [T{}] {:<7} ",
                        if is_tuned { "►" } else { " " },
                        spec.tier,
                        found.symbol
                    ),
                    Style::default()
                        .fg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:<27}", found.name),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!(
                        "| {:>6.2} TeV | {:.1}GHz | {:.2}σ (x{})",
                        spec.min_energy_tev, spec.resonant_harmonic, found.best_sigma, found.count
                    ),
                    Style::default().fg(Color::Cyan),
                ),
            ])
        } else {
            let color = if is_tuned {
                Color::LightYellow
            } else if snap.beam_energy_tev >= spec.min_energy_tev {
                Color::Yellow
            } else {
                Color::Gray
            };
            Line::from(vec![
                Span::styled(
                    format!(
                        "{}○ [T{}] {:<7} ",
                        if is_tuned { "►" } else { " " },
                        spec.tier,
                        spec.symbol
                    ),
                    Style::default().fg(color),
                ),
                Span::styled(
                    format!("{:<27}", spec.name),
                    Style::default().fg(color),
                ),
                Span::styled(
                    format!(
                        "| Req {:>6.2} TeV | Click -> {:.1} GHz ({})",
                        spec.min_energy_tev,
                        spec.resonant_harmonic,
                        spec.preferred_mode.short_name()
                    ),
                    Style::default().fg(color),
                ),
            ])
        };
        f.render_widget(Paragraph::new(line), row_rect);
        hitboxes.push(HitBox {
            rect: row_rect,
            action: DeckAction::Command(EngineCommand::Batch(vec![
                EngineCommand::SetRfHarmonic(spec.resonant_harmonic),
                EngineCommand::SetMode(spec.preferred_mode),
            ])),
        });
    }

    // Right side of bottom: 5σ progress gauge + detector event log
    let right_bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(4)])
        .split(bottom_cols[1]);

    let sigma_pct = ((snap.next_target_sigma_progress / 5.0) * 100.0)
        .round()
        .clamp(0.0, 100.0) as u16;
    let sigma_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Next 5σ Quantum Discovery Threshold "),
        )
        .gauge_style(Style::default().fg(Color::LightMagenta).bg(Color::Black))
        .percent(sigma_pct)
        .label(format!(
            "{} ({}): {:.2}σ / 5.00σ (Drop/s: {:.2}%)",
            snap.next_target_name,
            snap.next_target_symbol,
            snap.next_target_sigma_progress,
            (snap.quantum_drop_chance_per_sec * 100.0).clamp(0.0, 100.0)
        ));
    f.render_widget(sigma_gauge, right_bottom[0]);

    let log_items: Vec<ListItem> = snap
        .recent_logs
        .iter()
        .rev()
        .take(10)
        .map(|l| ListItem::new(Line::from(Span::raw(format!(" {}", l)))))
        .collect();
    f.render_widget(
        List::new(log_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Detector & RF Cavity Event Log "),
        ),
        right_bottom[1],
    );
}

fn run_headless_engine_loop(
    cfg: &EngineConfig,
    bind_addr: &str,
    compute_pool: &mut CpuComputePool,
    shared: &Arc<Mutex<SharedEngineContext>>,
    running: &Arc<AtomicBool>,
) -> io::Result<()> {
    let max_threads = compute_pool.max_threads();
    if !cfg.quiet {
        println!("╔══════════════════════════════════════════════════════════════════════════╗");
        println!("║  LOCAL COLLIDER :: PARTICLE ACCELERATOR ENGINE DAEMON (RUST)             ║");
        println!("╠══════════════════════════════════════════════════════════════════════════╣");
        println!(
            "║  IPC TCP Server : {:<54} ║",
            format!("{} (ready for collider-viz / CLI)", bind_addr)
        );
        println!(
            "║  Active Threads : {}/{} cores | CPU Intensity: {}%                       ║",
            compute_pool.active_threads(),
            max_threads,
            compute_pool.intensity_pct()
        );
        println!(
            "║  Visualizer Tip : In Window 2, run `cargo run --bin collider-viz`        ║"
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

    while running.load(Ordering::Relaxed) {
        if let Some(max_secs) = cfg.duration_secs {
            if start_instant.elapsed().as_secs() >= max_secs {
                if !cfg.quiet {
                    println!(
                        "\n[ENGINE] Configured duration ({max_secs}s) reached. Saving state..."
                    );
                }
                let guard = shared.lock().unwrap();
                guard.physics.save_to_disk();
                running.store(false, Ordering::Relaxed);
                break;
            }
        }

        thread::sleep(Duration::from_millis(100));

        let now = Instant::now();
        let dt = now.duration_since(last_tick).as_secs_f64().max(0.02);
        last_tick = now;

        let snap = step_engine_physics(dt, start_instant, &mut sys, compute_pool, shared);

        if !cfg.quiet && snap.discoveries.len() > known_discoveries_count {
            if let Some(newest) = snap.discoveries.last() {
                println!(
                    "\n★ [QUANTUM DISCOVERY {:.2}σ] {} ({}) | Mass: {:.2} GeV | Beam: {:.2} TeV",
                    newest.best_sigma,
                    newest.name,
                    newest.symbol,
                    newest.mass_gev,
                    snap.beam_energy_tev
                );
                println!("  └─ {}", newest.description);
            }
            known_discoveries_count = snap.discoveries.len();
        }

        if !cfg.quiet && last_console_log.elapsed() >= Duration::from_secs(1) {
            last_console_log = Instant::now();
            println!(
                "[T+{:>4}s] CPU: {:>5.1}% ({}/{} thr @ {:>3}%) | {:>7.1} kH/s ({:>5.2} GFLOPS) | E: {:>8.2} TeV | v: 0.{}c (γ={:>9.1}) | Collisions/s: {:>7.0} | Quantum: {:>5.1}% ({}/{})",
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
        format!("999..[{} nines]", count)
    }
}

fn handle_client_connection(
    stream: TcpStream,
    shared: Arc<Mutex<SharedEngineContext>>,
) -> io::Result<()> {
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
            apply_command_locked(&mut guard, cmd);
            guard.latest_snapshot.clone()
        };

        let mut payload = serde_json::to_string(&reply_snapshot).unwrap_or_default();
        payload.push('\n');
        writer.write_all(payload.as_bytes())?;
        writer.flush()?;
    }

    Ok(())
}

/// Sends a single command to the running engine over TCP and returns the updated TelemetrySnapshot.
pub fn send_command_once(
    host: &str,
    port: u16,
    cmd: &EngineCommand,
) -> io::Result<TelemetrySnapshot> {
    let addr = format!("{}:{}", host, port);
    let mut stream = TcpStream::connect_timeout(
        &addr.parse().map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid IPC address: {e}"),
            )
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
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(snap)
}

/// Builds an offline snapshot from `collider_state.json` when the daemon is not currently running.
pub fn build_offline_snapshot() -> TelemetrySnapshot {
    let mut phys = AcceleratorPhysicsState::load_or_new();
    let max_threads = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .max(1);
    phys.step(0.01, 0.0, 0.0, 0x1234, 0, 0, max_threads, 0, 0.0, 0, 0)
}
