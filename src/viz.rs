use crate::engine::send_command_once;
use crate::physics::QUANTUM_CATALOG;
use crate::protocol::{
    AccelerationMode, CollisionShower, EngineCommand, TelemetrySnapshot, TrackKind,
};
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
    symbols,
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Circle, Line as CanvasLine, Points},
        Block, Borders, Gauge, List, ListItem, Paragraph,
    },
    Frame, Terminal,
};
use std::io;
use std::time::{Duration, Instant};

pub struct VizConfig {
    pub host: String,
    pub port: u16,
    pub fps: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CameraMode {
    FullRingAndDetector,
    VertexCalorimeterZoom,
    ResonanceOscilloscope,
}

impl CameraMode {
    fn next(self) -> Self {
        match self {
            Self::FullRingAndDetector => Self::VertexCalorimeterZoom,
            Self::VertexCalorimeterZoom => Self::ResonanceOscilloscope,
            Self::ResonanceOscilloscope => Self::FullRingAndDetector,
        }
    }
}

#[derive(Clone)]
enum VizClickAction {
    SetCamera(CameraMode),
    SendCmd(EngineCommand),
}

#[derive(Clone)]
struct VizHitBox {
    rect: Rect,
    action: VizClickAction,
}

struct VizAnimationState {
    phase_angle: f64,
    rf_wave_phase: f64,
    shower_age: f64,
    last_shower_seq: u64,
    flash_ticks: u32,
    collision_flash: f64,
    last_discovery_count: usize,
    camera: CameraMode,
    calorimeter_bins: [f64; 36],
    pending_cmd: Option<EngineCommand>,
    last_error: Option<String>,
}

impl VizAnimationState {
    fn new() -> Self {
        Self {
            phase_angle: 0.0,
            rf_wave_phase: 0.0,
            shower_age: 0.0,
            last_shower_seq: 0,
            flash_ticks: 0,
            collision_flash: 0.0,
            last_discovery_count: 0,
            camera: CameraMode::FullRingAndDetector,
            calorimeter_bins: [0.0; 36],
            pending_cmd: None,
            last_error: None,
        }
    }

    fn absorb_shower_into_calorimeter(&mut self, shower: &CollisionShower) {
        for track in &shower.tracks {
            let norm_angle = track.angle_rad.rem_euclid(std::f64::consts::TAU);
            let bin = ((norm_angle / std::f64::consts::TAU) * 36.0).floor() as usize % 36;
            let boost = if shower.is_quantum_anomaly {
                1.0
            } else {
                0.65
            };
            self.calorimeter_bins[bin] =
                (self.calorimeter_bins[bin] + track.energy_ratio * boost).min(1.4);
        }
    }
}

pub fn run_tui_visualizer(cfg: VizConfig) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = viz_loop(&mut terminal, &cfg);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    res
}

fn viz_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    cfg: &VizConfig,
) -> io::Result<()> {
    let frame_duration = Duration::from_millis(1000 / cfg.fps.clamp(10, 60));
    let mut anim = VizAnimationState::new();
    let mut latest_snap: Option<TelemetrySnapshot> = None;
    let mut last_poll = Instant::now();
    let mut last_frame = Instant::now();
    let mut hitboxes: Vec<VizHitBox> = Vec::new();

    loop {
        let dt = last_frame.elapsed().as_secs_f64().clamp(0.01, 0.20);
        last_frame = Instant::now();

        if anim.pending_cmd.is_some() || last_poll.elapsed() >= Duration::from_millis(75) {
            last_poll = Instant::now();
            let cmd = anim.pending_cmd.take().unwrap_or(EngineCommand::GetSnapshot);
            match send_command_once(&cfg.host, cfg.port, &cmd) {
                Ok(snap) => {
                    if snap.discoveries.len() > anim.last_discovery_count {
                        anim.flash_ticks = 30;
                        anim.last_discovery_count = snap.discoveries.len();
                    }
                    if let Some(ref sh) = snap.latest_shower {
                        if sh.seq_id != anim.last_shower_seq {
                            anim.last_shower_seq = sh.seq_id;
                            anim.shower_age = 0.0;
                            anim.collision_flash = 1.0;
                            anim.absorb_shower_into_calorimeter(sh);
                        }
                    }
                    anim.last_error = None;
                    latest_snap = Some(snap);
                }
                Err(e) => {
                    anim.last_error = Some(format!(
                        "Engine offline at {}:{} ({})",
                        cfg.host, cfg.port, e
                    ));
                }
            }
        }

        // Advance relativistic animation kinematics
        if let Some(ref snap) = latest_snap {
            let speed_factor = 2.6 + (snap.beam_energy_tev.ln_1p() * 1.55);
            let burst_boost = if snap.burst_ticks_remaining > 0 {
                2.35
            } else {
                1.0
            };
            anim.phase_angle =
                (anim.phase_angle + dt * speed_factor * burst_boost) % std::f64::consts::TAU;
            anim.rf_wave_phase =
                (anim.rf_wave_phase + dt * snap.rf_harmonic * 7.5) % std::f64::consts::TAU;
            anim.shower_age = (anim.shower_age + dt * 1.65).min(1.65);
            anim.collision_flash = (anim.collision_flash - dt * 3.2).max(0.0);
            for bin in &mut anim.calorimeter_bins {
                *bin = (*bin - dt * 0.55).max(0.0);
            }
            if anim.flash_ticks > 0 {
                anim.flash_ticks -= 1;
            }
        } else {
            anim.phase_angle = (anim.phase_angle + dt * 1.2) % std::f64::consts::TAU;
            anim.rf_wave_phase = (anim.rf_wave_phase + dt * 3.0) % std::f64::consts::TAU;
        }

        hitboxes.clear();
        terminal.draw(|f| {
            if let Some(ref snap) = latest_snap {
                draw_connected_ui(f, snap, &anim, cfg, &mut hitboxes);
            } else {
                draw_waiting_ui(f, &anim, cfg);
            }
        })?;

        if event::poll(frame_duration)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => break,
                    KeyCode::Char('v') | KeyCode::Char('V') | KeyCode::Tab => {
                        anim.camera = anim.camera.next();
                    }
                    KeyCode::Char('1') => anim.camera = CameraMode::FullRingAndDetector,
                    KeyCode::Char('2') => anim.camera = CameraMode::VertexCalorimeterZoom,
                    KeyCode::Char('3') => anim.camera = CameraMode::ResonanceOscilloscope,
                    KeyCode::Char('m') | KeyCode::Char('M') => {
                        anim.pending_cmd = Some(EngineCommand::CycleMode);
                    }
                    KeyCode::Char('a') | KeyCode::Char('A') => {
                        anim.pending_cmd = Some(EngineCommand::LockOptimalHarmonic);
                    }
                    KeyCode::Char('+') | KeyCode::Char('=') => {
                        if let Some(ref s) = latest_snap {
                            anim.pending_cmd =
                                Some(EngineCommand::SetThreads(s.active_threads + 1));
                        }
                    }
                    KeyCode::Char('-') | KeyCode::Char('_') => {
                        if let Some(ref s) = latest_snap {
                            anim.pending_cmd = Some(EngineCommand::SetThreads(
                                s.active_threads.saturating_sub(1).max(1),
                            ));
                        }
                    }
                    KeyCode::Char(']') => {
                        if let Some(ref s) = latest_snap {
                            anim.pending_cmd = Some(EngineCommand::SetIntensity(
                                s.intensity_pct.saturating_add(10).min(100),
                            ));
                        }
                    }
                    KeyCode::Char('[') => {
                        if let Some(ref s) = latest_snap {
                            anim.pending_cmd = Some(EngineCommand::SetIntensity(
                                s.intensity_pct.saturating_sub(10).max(10),
                            ));
                        }
                    }
                    KeyCode::Up => {
                        anim.pending_cmd = Some(EngineCommand::AdjustRfHarmonic(0.1));
                    }
                    KeyCode::Down => {
                        anim.pending_cmd = Some(EngineCommand::AdjustRfHarmonic(-0.1));
                    }
                    KeyCode::Right => {
                        anim.pending_cmd = Some(EngineCommand::AdjustMagnetFocus(0.1));
                    }
                    KeyCode::Left => {
                        anim.pending_cmd = Some(EngineCommand::AdjustMagnetFocus(-0.1));
                    }
                    KeyCode::Char(' ') | KeyCode::Char('b') | KeyCode::Char('B') => {
                        anim.pending_cmd = Some(EngineCommand::TriggerPulseBurst);
                    }
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        anim.pending_cmd = Some(EngineCommand::ResetState);
                    }
                    _ => {}
                },
                Event::Mouse(mouse) => {
                    if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                        let col = mouse.column;
                        let row = mouse.row;
                        for hb in &hitboxes {
                            if col >= hb.rect.x
                                && col < hb.rect.x.saturating_add(hb.rect.width)
                                && row >= hb.rect.y
                                && row < hb.rect.y.saturating_add(hb.rect.height)
                            {
                                match &hb.action {
                                    VizClickAction::SetCamera(cam) => anim.camera = *cam,
                                    VizClickAction::SendCmd(cmd) => {
                                        anim.pending_cmd = Some(cmd.clone());
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
}

fn draw_waiting_ui(f: &mut Frame, anim: &VizAnimationState, cfg: &VizConfig) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(7)])
        .split(area);

    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" LOCAL COLLIDER :: LIVE TUI VISUALIZER (WAITING FOR ENGINE DAEMON) "),
        )
        .marker(symbols::Marker::Braille)
        .x_bounds([-100.0, 100.0])
        .y_bounds([-100.0, 100.0])
        .paint(|ctx| {
            ctx.draw(&Circle {
                x: 0.0,
                y: 0.0,
                radius: 65.0,
                color: Color::DarkGray,
            });
            let px = 65.0 * anim.phase_angle.cos();
            let py = 65.0 * anim.phase_angle.sin();
            ctx.draw(&Points {
                coords: &[(px, py)],
                color: Color::Yellow,
            });
        });
    f.render_widget(canvas, chunks[0]);

    let err_msg = anim
        .last_error
        .as_deref()
        .unwrap_or("Connecting to collider engine...");
    let info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                " STATUS: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(err_msg),
        ]),
        Line::from(" Start the clickable accelerator engine in Window 1 with:"),
        Line::from(Span::styled(
            format!(
                "   cargo run --release --bin collider-engine -- --port {}",
                cfg.port
            ),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(" Press [Q] or [ESC] to exit."),
    ])
    .block(Block::default().borders(Borders::ALL).title(" Instructions "));
    f.render_widget(info, chunks[1]);
}

fn draw_connected_ui(
    f: &mut Frame,
    snap: &TelemetrySnapshot,
    anim: &VizAnimationState,
    cfg: &VizConfig,
    hitboxes: &mut Vec<VizHitBox>,
) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Top header bar
            Constraint::Min(16),   // Main area (Left: Animated Canvas + Camera Tabs, Right: Telemetry & Codex)
            Constraint::Length(7), // Event logs & Hotkeys
        ])
        .split(f.area());

    draw_header(f, root[0], snap, anim, cfg);

    let main_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(root[1]);

    // Left column: Clickable camera mode bar + Main Braille Animation Canvas
    let left_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(12)])
        .split(main_cols[0]);

    draw_camera_selector_bar(f, left_rows[0], anim, hitboxes);
    draw_collider_animation_canvas(f, left_rows[1], snap, anim, hitboxes);

    // Right column: Telemetry + RF Oscilloscope/Tuning + Quantum Codex
    let right_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // CPU -> Relativistic Kinematics
            Constraint::Length(8), // How It Accelerates (RF Tuning + Capacitor)
            Constraint::Min(7),    // Quantum Proximity & 5σ Codex
        ])
        .split(main_cols[1]);

    draw_cpu_and_kinematics(f, right_rows[0], snap);
    draw_tuning_and_capacitor(f, right_rows[1], snap, hitboxes);
    draw_quantum_codex(f, right_rows[2], snap, hitboxes);

    draw_footer_logs_and_controls(f, root[2], snap);
}

fn draw_camera_selector_bar(
    f: &mut Frame,
    area: Rect,
    anim: &VizAnimationState,
    hitboxes: &mut Vec<VizHitBox>,
) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(34),
            Constraint::Percentage(32),
        ])
        .split(area);

    let views = [
        (CameraMode::FullRingAndDetector, "[1] Full Ring & Detector"),
        (
            CameraMode::VertexCalorimeterZoom,
            "[2] Vertex & Calorimeter Zoom",
        ),
        (
            CameraMode::ResonanceOscilloscope,
            "[3] RF Breit-Wigner Scope",
        ),
    ];

    for (i, (mode, label)) in views.iter().enumerate() {
        let active = anim.camera == *mode;
        let fg = if active { Color::Black } else { Color::Cyan };
        let bg = if active { Color::Cyan } else { Color::Black };
        let p = Paragraph::new(Line::from(Span::styled(
            *label,
            Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );
        f.render_widget(p, cols[i]);
        hitboxes.push(VizHitBox {
            rect: cols[i],
            action: VizClickAction::SetCamera(*mode),
        });
    }
}

fn draw_header(
    f: &mut Frame,
    area: Rect,
    snap: &TelemetrySnapshot,
    anim: &VizAnimationState,
    cfg: &VizConfig,
) {
    let border_color = if anim.flash_ticks > 0 {
        Color::LightMagenta
    } else if snap.burst_ticks_remaining > 0 {
        Color::LightRed
    } else {
        Color::Cyan
    };

    let status_badge = if snap.burst_ticks_remaining > 0 {
        Span::styled(
            " [⚡ SUPERCRITICAL OVERDRIVE ⚡] ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        )
    } else if anim.flash_ticks > 0 {
        Span::styled(
            " [★ NEW 5σ QUANTUM DISCOVERY ★] ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightMagenta)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " [BEAM LOCKED :: ONLINE] ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
    };

    let line = Line::from(vec![
        Span::styled(
            " ⚛ LOCAL COLLIDER VIZ ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        status_badge,
        Span::raw(format!(
            " | IPC {}:{} | Mode: ",
            cfg.host, cfg.port
        )),
        Span::styled(
            snap.mode.name(),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            " | Beam: {:.2} TeV (Peak: {:.2} TeV)",
            snap.beam_energy_tev, snap.peak_energy_tev
        )),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    f.render_widget(Paragraph::new(line).block(block), area);
}

fn draw_collider_animation_canvas(
    f: &mut Frame,
    area: Rect,
    snap: &TelemetrySnapshot,
    anim: &VizAnimationState,
    hitboxes: &mut Vec<VizHitBox>,
) {
    // Clicking the main collider canvas triggers a supercritical pulse burst
    hitboxes.push(VizHitBox {
        rect: area,
        action: VizClickAction::SendCmd(EngineCommand::TriggerPulseBurst),
    });

    match anim.camera {
        CameraMode::FullRingAndDetector => draw_full_ring_canvas(f, area, snap, anim),
        CameraMode::VertexCalorimeterZoom => draw_vertex_zoom_canvas(f, area, snap, anim),
        CameraMode::ResonanceOscilloscope => draw_oscilloscope_canvas(f, area, snap, anim),
    }
}

fn draw_full_ring_canvas(
    f: &mut Frame,
    area: Rect,
    snap: &TelemetrySnapshot,
    anim: &VizAnimationState,
) {
    let ring_color = match snap.mode {
        AccelerationMode::LinearLinac => Color::Blue,
        AccelerationMode::SynchrotronHarmonic => Color::Cyan,
        AccelerationMode::PlasmaWakefield => Color::LightRed,
        AccelerationMode::QuantumResonance => Color::LightMagenta,
    };

    let title = format!(
        " Synchrotron & Calorimeter [v = 0.{}c | γ = {:.1}] (Click Canvas = Pulse Burst) ",
        "9".repeat((snap.nines_of_c.floor() as usize).clamp(2, 10)),
        snap.lorentz_gamma
    );

    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ring_color))
                .title(title),
        )
        .marker(symbols::Marker::Braille)
        .x_bounds([-115.0, 115.0])
        .y_bounds([-115.0, 115.0])
        .paint(|ctx| {
            // 1. Cryogenic outer magnet cryostat & inner beam pipe
            ctx.draw(&Circle {
                x: 0.0,
                y: 0.0,
                radius: 88.0,
                color: ring_color,
            });
            ctx.draw(&Circle {
                x: 0.0,
                y: 0.0,
                radius: 78.0,
                color: Color::DarkGray,
            });

            // 2. Superconducting RF Cavities & Quadrupole Magnets around the ring
            for i in 0..12 {
                let a = (i as f64) * (std::f64::consts::TAU / 12.0);
                let x1 = 74.0 * a.cos();
                let y1 = 74.0 * a.sin();
                let x2 = 93.0 * a.cos();
                let y2 = 93.0 * a.sin();
                let rf_active = ((anim.phase_angle - a).cos()) > 0.82;
                let magnet_color = if rf_active {
                    Color::LightYellow
                } else if i % 2 == 0 {
                    Color::Yellow
                } else {
                    Color::Gray
                };
                ctx.draw(&CanvasLine {
                    x1,
                    y1,
                    x2,
                    y2,
                    color: magnet_color,
                });
            }

            // 3. Multi-bunch counter-rotating proton beams (bunch count scales with active threads)
            let num_bunches = snap.active_threads.clamp(1, 6);
            let r_beam1 = 84.0;
            let r_beam2 = 81.5;

            // Relativistic Doppler color shift based on Lorentz γ
            let wake_color1 = if snap.lorentz_gamma > 50_000.0 {
                Color::LightMagenta
            } else if snap.lorentz_gamma > 5_000.0 {
                Color::LightCyan
            } else {
                Color::Cyan
            };

            for b in 0..num_bunches {
                let bunch_offset = (b as f64) * (std::f64::consts::TAU / num_bunches as f64);
                let mut trail1 = Vec::with_capacity(20);
                let mut trail2 = Vec::with_capacity(20);

                for k in 0..20 {
                    let lag = (k as f64) * 0.045;
                    let betatron = ((anim.rf_wave_phase + k as f64 * 0.6).sin()) * 1.3;
                    let a1 = anim.phase_angle + bunch_offset - lag;
                    let a2 = -anim.phase_angle - bunch_offset + std::f64::consts::PI + lag;
                    trail1.push(((r_beam1 + betatron) * a1.cos(), (r_beam1 + betatron) * a1.sin()));
                    trail2.push(((r_beam2 - betatron) * a2.cos(), (r_beam2 - betatron) * a2.sin()));
                }

                ctx.draw(&Points {
                    coords: &trail1[2..],
                    color: wake_color1,
                });
                ctx.draw(&Points {
                    coords: &trail2[2..],
                    color: Color::LightRed,
                });
                ctx.draw(&Points {
                    coords: &trail1[0..2],
                    color: Color::White,
                });
                ctx.draw(&Points {
                    coords: &trail2[0..2],
                    color: Color::LightYellow,
                });
            }

            // 4. Beam transfer lines feeding the central Interaction Point (0, 0)
            ctx.draw(&CanvasLine {
                x1: -81.0,
                y1: 0.0,
                x2: -10.0,
                y2: 0.0,
                color: Color::DarkGray,
            });
            ctx.draw(&CanvasLine {
                x1: 81.0,
                y1: 0.0,
                x2: 10.0,
                y2: 0.0,
                color: Color::DarkGray,
            });

            // 5. Central Solenoid Tracker & 36-Sector Calorimeter Ring
            ctx.draw(&Circle {
                x: 0.0,
                y: 0.0,
                radius: 48.0,
                color: Color::DarkGray,
            });
            ctx.draw(&Circle {
                x: 0.0,
                y: 0.0,
                radius: 12.0,
                color: if anim.collision_flash > 0.3 {
                    Color::White
                } else {
                    Color::Gray
                },
            });

            // Render glowing calorimeter energy towers around r = 48..65
            for (bin_idx, &energy) in anim.calorimeter_bins.iter().enumerate() {
                if energy > 0.04 {
                    let a = (bin_idx as f64) * (std::f64::consts::TAU / 36.0);
                    let r_in = 48.0;
                    let r_out = 48.0 + (energy * 18.0).min(21.0);
                    let tower_color = if energy > 0.85 {
                        Color::LightMagenta
                    } else if energy > 0.45 {
                        Color::LightYellow
                    } else {
                        Color::LightGreen
                    };
                    ctx.draw(&CanvasLine {
                        x1: r_in * a.cos(),
                        y1: r_in * a.sin(),
                        x2: r_out * a.cos(),
                        y2: r_out * a.sin(),
                        color: tower_color,
                    });
                }
            }

            // 6. Quantum vacuum wave ripples when in overdrive or Quantum Resonance mode
            if snap.burst_ticks_remaining > 0 || snap.mode == AccelerationMode::QuantumResonance {
                for w in 0..2 {
                    let wave_r = 16.0
                        + (((anim.phase_angle * 2.2 + w as f64 * 1.57).sin().abs()) * 46.0);
                    ctx.draw(&Circle {
                        x: 0.0,
                        y: 0.0,
                        radius: wave_r,
                        color: Color::Magenta,
                    });
                }
            }

            // 7. Curved Helical Particle Tracks + Secondary Boson Decay Forks
            if let Some(ref shower) = snap.latest_shower {
                paint_shower_tracks(ctx, shower, anim.shower_age, 50.0);
            }
        });

    f.render_widget(canvas, area);
}

fn draw_vertex_zoom_canvas(
    f: &mut Frame,
    area: Rect,
    snap: &TelemetrySnapshot,
    anim: &VizAnimationState,
) {
    let title = format!(
        " Silicon Vertex Tracker & Calorimeter Zoom [Event: {}] ",
        snap.latest_shower
            .as_ref()
            .map(|s| s.label.as_str())
            .unwrap_or("Waiting for collision")
    );

    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::LightMagenta))
                .title(title),
        )
        .marker(symbols::Marker::Braille)
        .x_bounds([-100.0, 100.0])
        .y_bounds([-100.0, 100.0])
        .paint(|ctx| {
            // Concentric silicon pixel layers & electromagnetic/hadronic calorimeters
            for &(r, col) in &[
                (18.0, Color::DarkGray),
                (38.0, Color::DarkGray),
                (62.0, Color::Gray),
                (82.0, Color::Cyan),
            ] {
                ctx.draw(&Circle {
                    x: 0.0,
                    y: 0.0,
                    radius: r,
                    color: col,
                });
            }

            // Incoming opposing proton beams colliding at (0, 0)
            let pinch = (1.0 - anim.shower_age.min(1.0)) * 80.0;
            ctx.draw(&CanvasLine {
                x1: -98.0,
                y1: 0.0,
                x2: -pinch.max(2.0),
                y2: 0.0,
                color: Color::Cyan,
            });
            ctx.draw(&CanvasLine {
                x1: 98.0,
                y1: 0.0,
                x2: pinch.max(2.0),
                y2: 0.0,
                color: Color::LightRed,
            });

            // High-resolution Calorimeter bars at r = 82..98
            for (bin_idx, &energy) in anim.calorimeter_bins.iter().enumerate() {
                if energy > 0.03 {
                    let a = (bin_idx as f64) * (std::f64::consts::TAU / 36.0);
                    let r_in = 82.0;
                    let r_out = 82.0 + (energy * 16.0).min(17.0);
                    ctx.draw(&CanvasLine {
                        x1: r_in * a.cos(),
                        y1: r_in * a.sin(),
                        x2: r_out * a.cos(),
                        y2: r_out * a.sin(),
                        color: if energy > 0.8 {
                            Color::LightMagenta
                        } else {
                            Color::LightYellow
                        },
                    });
                }
            }

            if let Some(ref shower) = snap.latest_shower {
                paint_shower_tracks(ctx, shower, anim.shower_age, 82.0);
            }
        });

    f.render_widget(canvas, area);
}

fn paint_shower_tracks(
    ctx: &mut ratatui::widgets::canvas::Context<'_>,
    shower: &CollisionShower,
    shower_age: f64,
    max_radius: f64,
) {
    let reach = (shower_age * max_radius * 1.15).clamp(6.0, max_radius);

    for track in &shower.tracks {
        let color = match track.kind {
            TrackKind::HadronJet => Color::LightGreen,
            TrackKind::MuonCurve => Color::LightCyan,
            TrackKind::PhotonRay => Color::LightYellow,
            TrackKind::QuantumAnomaly => Color::LightMagenta,
        };

        let track_max_r = (reach * track.speed * track.energy_ratio.max(0.45)).min(max_radius);
        let steps = 10;
        let mut prev_x = 0.0;
        let mut prev_y = 0.0;
        let mut tip_x = 0.0;
        let mut tip_y = 0.0;

        for s in 1..=steps {
            let frac = (s as f64) / (steps as f64);
            let r = track_max_r * frac;
            let deflected_angle = track.angle_rad + track.curvature * frac * 0.95;
            let nx = r * deflected_angle.cos();
            let ny = r * deflected_angle.sin();
            ctx.draw(&CanvasLine {
                x1: prev_x,
                y1: prev_y,
                x2: nx,
                y2: ny,
                color,
            });

            // Secondary decay vertex fork (e.g., Z⁰ -> μ⁺μ⁻ or H⁰ -> γγ)
            if let Some(fork_frac) = track.decay_fork_at {
                if frac >= fork_frac && s == ((fork_frac * steps as f64).round() as usize).max(2) {
                    let branch_len = (track_max_r - r).max(4.0) * 0.85;
                    let a_plus = deflected_angle + 0.55;
                    let a_minus = deflected_angle - 0.55;
                    ctx.draw(&CanvasLine {
                        x1: nx,
                        y1: ny,
                        x2: nx + branch_len * a_plus.cos(),
                        y2: ny + branch_len * a_plus.sin(),
                        color: Color::White,
                    });
                    ctx.draw(&CanvasLine {
                        x1: nx,
                        y1: ny,
                        x2: nx + branch_len * a_minus.cos(),
                        y2: ny + branch_len * a_minus.sin(),
                        color: Color::LightMagenta,
                    });
                }
            }

            prev_x = nx;
            prev_y = ny;
            tip_x = nx;
            tip_y = ny;
        }

        // Bright particle wavefront head at the tip of the trajectory
        ctx.draw(&Points {
            coords: &[(tip_x, tip_y)],
            color: Color::White,
        });
    }

    if shower.is_quantum_anomaly {
        ctx.draw(&Circle {
            x: 0.0,
            y: 0.0,
            radius: (shower_age * max_radius * 0.78).clamp(5.0, max_radius * 0.92),
            color: Color::LightMagenta,
        });
    }
}

fn draw_oscilloscope_canvas(
    f: &mut Frame,
    area: Rect,
    snap: &TelemetrySnapshot,
    anim: &VizAnimationState,
) {
    let title = format!(
        " RF Standing Wave & Breit-Wigner Quantum Resonance [Tuned: {:.2} GHz | Target: {:.2} GHz ({:.1}%)] ",
        snap.rf_harmonic, snap.optimal_harmonic_hint, snap.harmonic_alignment_pct
    );

    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(title),
        )
        .marker(symbols::Marker::Braille)
        .x_bounds([0.5, 5.0])
        .y_bounds([-1.2, 2.2])
        .paint(|ctx| {
            // Baseline axis
            ctx.draw(&CanvasLine {
                x1: 0.5,
                y1: 0.0,
                x2: 5.0,
                y2: 0.0,
                color: Color::DarkGray,
            });

            // 1. Breit-Wigner Resonance curve centered at optimal_harmonic_hint
            let mut prev_x = 0.5;
            let mut prev_y = 0.0;
            for i in 0..=90 {
                let freq = 0.5 + (i as f64 / 90.0) * 4.5;
                let d = freq - snap.optimal_harmonic_hint;
                let bw = 0.35 / (d * d + 0.175); // Peak = 2.0
                if i > 0 {
                    ctx.draw(&CanvasLine {
                        x1: prev_x,
                        y1: prev_y,
                        x2: freq,
                        y2: bw,
                        color: Color::LightMagenta,
                    });
                }
                prev_x = freq;
                prev_y = bw;
            }

            // 2. Live animated RF cavity standing wave
            let mut w_prev_x = 0.5;
            let mut w_prev_y = 0.0;
            for i in 0..=90 {
                let x = 0.5 + (i as f64 / 90.0) * 4.5;
                let y = ((x * snap.rf_harmonic * 3.2 - anim.rf_wave_phase).sin()) * 0.65 - 0.35;
                if i > 0 {
                    ctx.draw(&CanvasLine {
                        x1: w_prev_x,
                        y1: w_prev_y,
                        x2: x,
                        y2: y,
                        color: Color::Cyan,
                    });
                }
                w_prev_x = x;
                w_prev_y = y;
            }

            // 3. Vertical markers for Target Resonance (Green) and Current RF Harmonic (Yellow)
            ctx.draw(&CanvasLine {
                x1: snap.optimal_harmonic_hint,
                y1: -1.1,
                x2: snap.optimal_harmonic_hint,
                y2: 2.05,
                color: Color::LightGreen,
            });
            ctx.draw(&CanvasLine {
                x1: snap.rf_harmonic,
                y1: -1.1,
                x2: snap.rf_harmonic,
                y2: 2.05,
                color: Color::LightYellow,
            });
        });

    f.render_widget(canvas, area);
}

fn draw_cpu_and_kinematics(f: &mut Frame, area: Rect, snap: &TelemetrySnapshot) {
    let nines_count = snap.nines_of_c.floor() as usize;
    let beta_str = if nines_count <= 9 {
        format!(
            "0.{}c ({:.2} nines of c)",
            "9".repeat(nines_count.max(2)),
            snap.nines_of_c
        )
    } else {
        format!("0.999999999...c ({:.2} nines of c!)", snap.nines_of_c)
    };

    let text = vec![
        Line::from(vec![
            Span::styled(
                " CPU Benchmark    : ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:.1} kH/s", snap.hashes_per_sec / 1000.0),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                " | {:.2} GFLOPS | Total: {:.2}M hashes",
                snap.gflops,
                snap.total_compute_cycles as f64 / 1e6
            )),
        ]),
        Line::from(vec![
            Span::styled(" Compute Allocation: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!(
                "{}/{} Threads [+/-] | Intensity: {}% [[/]] | Sys CPU: {:.1}%",
                snap.active_threads, snap.max_threads, snap.intensity_pct, snap.system_cpu_pct
            )),
        ]),
        Line::from(vec![
            Span::styled(
                " Beam Velocity (β): ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                beta_str,
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Lorentz Factor γ : ", Style::default().fg(Color::Yellow)),
            Span::raw(format!(
                "γ = {:.2} | Beam Energy: {:.3} TeV (Peak: {:.3} TeV)",
                snap.lorentz_gamma, snap.beam_energy_tev, snap.peak_energy_tev
            )),
        ]),
        Line::from(vec![
            Span::styled(" Collision Rate   : ", Style::default().fg(Color::Green)),
            Span::raw(format!(
                "{:.0} collisions/s | Total: {} | Luminosity: {:.3} fb⁻¹",
                snap.collisions_per_sec, snap.total_collisions, snap.luminosity_fb_inv
            )),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" CPU Compute <-> Relativistic Kinematics ");
    f.render_widget(Paragraph::new(text).block(block), area);
}

fn draw_tuning_and_capacitor(
    f: &mut Frame,
    area: Rect,
    snap: &TelemetrySnapshot,
    hitboxes: &mut Vec<VizHitBox>,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(2),
        ])
        .split(area);

    let outer = Block::default()
        .borders(Borders::ALL)
        .title(" How It Accelerates :: RF Cavity & Quadrupole Tuning (Clickable) ");
    f.render_widget(outer, area);

    let mode_line = Line::from(vec![
        Span::styled(
            " Mode [M / Click] : ",
            Style::default().fg(Color::Magenta),
        ),
        Span::styled(
            snap.mode.name(),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" — {}", snap.mode.description())),
    ]);
    f.render_widget(Paragraph::new(mode_line), rows[0]);
    hitboxes.push(VizHitBox {
        rect: rows[0],
        action: VizClickAction::SendCmd(EngineCommand::CycleMode),
    });

    let focus_line = Line::from(vec![
        Span::styled(" Magnet Focus [←/→]: ", Style::default().fg(Color::Cyan)),
        Span::raw(format!(
            "{:.2} T/m | Beam Phase Stability: {:.1}%",
            snap.magnet_focus, snap.beam_stability_pct
        )),
    ]);
    f.render_widget(Paragraph::new(focus_line), rows[1]);

    let align_color = if snap.harmonic_alignment_pct >= 85.0 {
        Color::LightGreen
    } else if snap.harmonic_alignment_pct >= 55.0 {
        Color::Yellow
    } else {
        Color::LightRed
    };
    let rf_line = Line::from(vec![
        Span::styled(" RF Harmonic [↑/↓]: ", Style::default().fg(Color::Cyan)),
        Span::styled(
            format!("{:.2} GHz", snap.rf_harmonic),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            " (Target: {:.2} GHz) | Resonance: ",
            snap.optimal_harmonic_hint
        )),
        Span::styled(
            format!("{:.1}% [Click/A = Auto-Lock]", snap.harmonic_alignment_pct),
            Style::default()
                .fg(align_color)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    f.render_widget(Paragraph::new(rf_line), rows[2]);
    hitboxes.push(VizHitBox {
        rect: rows[2],
        action: VizClickAction::SendCmd(EngineCommand::LockOptimalHarmonic),
    });

    let cap_label = if snap.burst_ticks_remaining > 0 {
        format!(
            "⚡ SUPERCRITICAL OVERDRIVE ACTIVE! ({:.1}s left — 3.25x TeV & Quantum) ⚡",
            snap.burst_ticks_remaining as f64 * 0.1
        )
    } else {
        format!(
            "Pulse Capacitor [SPACE / Click]: {:.1}% (Min 25% to fire overdrive)",
            snap.capacitor_charge_pct
        )
    };
    let cap_gauge = Gauge::default()
        .gauge_style(
            Style::default()
                .fg(if snap.burst_ticks_remaining > 0 {
                    Color::LightRed
                } else {
                    Color::Cyan
                })
                .bg(Color::Black),
        )
        .percent((snap.capacitor_charge_pct.round() as u16).clamp(0, 100))
        .label(cap_label);
    f.render_widget(cap_gauge, rows[4]);
    hitboxes.push(VizHitBox {
        rect: rows[4],
        action: VizClickAction::SendCmd(EngineCommand::TriggerPulseBurst),
    });
}

fn draw_quantum_codex(
    f: &mut Frame,
    area: Rect,
    snap: &TelemetrySnapshot,
    hitboxes: &mut Vec<VizHitBox>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(2), Constraint::Min(3)])
        .split(area);

    let title = format!(
        " Quantum Proximity: {:.1}% | 5σ Discovery Codex ({}/{}) ",
        snap.quantum_proximity_pct,
        snap.discoveries.len(),
        QUANTUM_CATALOG.len()
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::LightMagenta))
        .title(title);
    f.render_widget(block, area);

    let sigma_pct = ((snap.next_target_sigma_progress / 5.0) * 100.0)
        .round()
        .clamp(0.0, 100.0) as u16;
    let sigma_label = format!(
        "Next Target: {} ({}) [Req {:.2} TeV] -> {:.2}σ / 5.00σ (Drop/s: {:.2}%)",
        snap.next_target_name,
        snap.next_target_symbol,
        snap.next_target_min_tev,
        snap.next_target_sigma_progress,
        (snap.quantum_drop_chance_per_sec * 100.0).clamp(0.0, 100.0)
    );
    let sigma_gauge = Gauge::default()
        .gauge_style(Style::default().fg(Color::LightMagenta).bg(Color::Black))
        .percent(sigma_pct)
        .label(sigma_label);
    f.render_widget(sigma_gauge, chunks[0]);

    let max_rows = chunks[1].height as usize;
    for (idx, spec) in QUANTUM_CATALOG.iter().take(max_rows).enumerate() {
        let row_rect = Rect {
            x: chunks[1].x,
            y: chunks[1].y + idx as u16,
            width: chunks[1].width,
            height: 1,
        };
        let line = if let Some(found) = snap.discoveries.iter().find(|d| d.id == spec.id) {
            Line::from(vec![
                Span::styled(
                    format!(" ★ [T{}] {:<7} ", spec.tier, found.symbol),
                    Style::default()
                        .fg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:<28}", found.name),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!(
                        " | {:>7.2} GeV | {:.2}σ | x{}",
                        found.mass_gev, found.best_sigma, found.count
                    ),
                    Style::default().fg(Color::Cyan),
                ),
            ])
        } else {
            let unlocked_energy = snap.beam_energy_tev >= spec.min_energy_tev;
            let color = if unlocked_energy {
                Color::Yellow
            } else {
                Color::DarkGray
            };
            Line::from(vec![
                Span::styled(
                    format!(" ○ [T{}] {:<7} ", spec.tier, spec.symbol),
                    Style::default().fg(color),
                ),
                Span::styled(
                    format!("{:<28}", spec.name),
                    Style::default().fg(color),
                ),
                Span::styled(
                    format!(
                        " | Req {:>6.2} TeV | RF {:.1} GHz",
                        spec.min_energy_tev, spec.resonant_harmonic
                    ),
                    Style::default().fg(color),
                ),
            ])
        };
        f.render_widget(Paragraph::new(line), row_rect);
        hitboxes.push(VizHitBox {
            rect: row_rect,
            action: VizClickAction::SendCmd(EngineCommand::Batch(vec![
                EngineCommand::SetRfHarmonic(spec.resonant_harmonic),
                EngineCommand::SetMode(spec.preferred_mode),
            ])),
        });
    }
}

fn draw_footer_logs_and_controls(f: &mut Frame, area: Rect, snap: &TelemetrySnapshot) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(area);

    let log_items: Vec<ListItem> = snap
        .recent_logs
        .iter()
        .rev()
        .take(4)
        .map(|l| ListItem::new(Line::from(Span::raw(format!(" {}", l)))))
        .collect();
    let logs_widget = List::new(log_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Live Detector & RF Telemetry Log "),
    );
    f.render_widget(logs_widget, cols[0]);

    let controls = Paragraph::new(vec![
        Line::from(" [1/2/3/V] Camera View | [SPACE] Supercritical Burst"),
        Line::from(" [M] Acceleration Mode | [A] Auto-Lock RF Resonance"),
        Line::from(" [+/-] CPU Threads     | [[ / ]] CPU Intensity %"),
        Line::from(" [↑/↓] RF Harmonic GHz | [← / →] Magnet Focus | [Q] Quit"),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Keyboard & Mouse Controls (IPC -> Engine) "),
    );
    f.render_widget(controls, cols[1]);
}
