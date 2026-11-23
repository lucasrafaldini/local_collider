use crate::engine::send_command_once;
use crate::physics::QUANTUM_CATALOG;
use crate::protocol::{AccelerationMode, EngineCommand, TelemetrySnapshot, TrackKind};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
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

struct VizAnimationState {
    phase_angle: f64,
    shower_age: f64,
    last_shower_seq: u64,
    flash_ticks: u32,
    last_discovery_count: usize,
    pending_cmd: Option<EngineCommand>,
    last_error: Option<String>,
}

impl VizAnimationState {
    fn new() -> Self {
        Self {
            phase_angle: 0.0,
            shower_age: 0.0,
            last_shower_seq: 0,
            flash_ticks: 0,
            last_discovery_count: 0,
            pending_cmd: None,
            last_error: None,
        }
    }
}

pub fn run_tui_visualizer(cfg: VizConfig) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = viz_loop(&mut terminal, &cfg);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    res
}

fn viz_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    cfg: &VizConfig,
) -> io::Result<()> {
    let frame_duration = Duration::from_millis(1000 / cfg.fps.clamp(5, 60));
    let mut anim = VizAnimationState::new();
    let mut latest_snap: Option<TelemetrySnapshot> = None;
    let mut last_poll = Instant::now();
    let mut last_frame = Instant::now();

    loop {
        let dt = last_frame.elapsed().as_secs_f64().clamp(0.01, 0.25);
        last_frame = Instant::now();

        // Consulta o daemon via IPC TCP a ~12 Hz ou imediatamente quando há comando do teclado
        if anim.pending_cmd.is_some() || last_poll.elapsed() >= Duration::from_millis(85) {
            last_poll = Instant::now();
            let cmd = anim.pending_cmd.take().unwrap_or(EngineCommand::GetSnapshot);
            match send_command_once(&cfg.host, cfg.port, &cmd) {
                Ok(snap) => {
                    if snap.discoveries.len() > anim.last_discovery_count {
                        anim.flash_ticks = 25;
                        anim.last_discovery_count = snap.discoveries.len();
                    }
                    if let Some(ref sh) = snap.latest_shower {
                        if sh.seq_id != anim.last_shower_seq {
                            anim.last_shower_seq = sh.seq_id;
                            anim.shower_age = 0.0;
                        }
                    }
                    anim.last_error = None;
                    latest_snap = Some(snap);
                }
                Err(e) => {
                    anim.last_error = Some(format!(
                        "Motor offline em {}:{} ({})",
                        cfg.host, cfg.port, e
                    ));
                }
            }
        }

        // Atualiza cinemática visual da animação a cada frame
        if let Some(ref snap) = latest_snap {
            let speed_factor = 2.2 + (snap.beam_energy_tev.ln_1p() * 1.45);
            let burst_boost = if snap.burst_ticks_remaining > 0 {
                2.2
            } else {
                1.0
            };
            anim.phase_angle =
                (anim.phase_angle + dt * speed_factor * burst_boost) % std::f64::consts::TAU;
            anim.shower_age = (anim.shower_age + dt * 1.85).min(1.5);
            if anim.flash_ticks > 0 {
                anim.flash_ticks -= 1;
            }
        } else {
            anim.phase_angle = (anim.phase_angle + dt * 1.2) % std::f64::consts::TAU;
        }

        terminal.draw(|f| {
            if let Some(ref snap) = latest_snap {
                draw_connected_ui(f, snap, &anim, cfg);
            } else {
                draw_waiting_ui(f, &anim, cfg);
            }
        })?;

        if event::poll(frame_duration)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => break,
                        KeyCode::Char('m') | KeyCode::Char('M') => {
                            anim.pending_cmd = Some(EngineCommand::CycleMode);
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
                    }
                }
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
                .title(" LOCAL COLLIDER :: VISUALIZADOR TUI (AGUARDANDO DAEMON) "),
        )
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
        .unwrap_or("Conectando ao motor...");
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
        Line::from(format!(
            " Para iniciar o motor do acelerador em outro terminal, execute:"
        )),
        Line::from(Span::styled(
            format!(
                "   cargo run --release --bin collider-engine -- --port {}",
                cfg.port
            ),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(" Pressione [Q] ou [ESC] para sair."),
    ])
    .block(Block::default().borders(Borders::ALL).title(" Instruções "));
    f.render_widget(info, chunks[1]);
}

fn draw_connected_ui(
    f: &mut Frame,
    snap: &TelemetrySnapshot,
    anim: &VizAnimationState,
    cfg: &VizConfig,
) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Barra de título / status global
            Constraint::Min(16),   // Área principal (Anel animado + Telemetria & Códice)
            Constraint::Length(7), // Logs + Controles
        ])
        .split(f.area());

    draw_header(f, root[0], snap, anim, cfg);

    let main_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
        .split(root[1]);

    draw_collider_ring_canvas(f, main_cols[0], snap, anim);

    let right_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(9), // CPU -> Velocidade & Cinemática Relativística
            Constraint::Length(8), // Como Acelera (Harmônico RF, Foco, Capacitor)
            Constraint::Min(7),    // Proximidade Quântica & Códice 5σ
        ])
        .split(main_cols[1]);

    draw_cpu_and_kinematics(f, right_rows[0], snap);
    draw_tuning_and_capacitor(f, right_rows[1], snap);
    draw_quantum_codex(f, right_rows[2], snap);

    draw_footer_logs_and_controls(f, root[2], snap);
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
            " [PULSO SUPERCRÍTICO ATIVO!] ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        )
    } else if anim.flash_ticks > 0 {
        Span::styled(
            " [★ NOVA DESCOBERTA QUÂNTICA 5σ ★] ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightMagenta)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " [ONLINE :: FEIXE ESTÁVEL] ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
    };

    let line = Line::from(vec![
        Span::styled(
            " ⚛ LOCAL COLLIDER ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        status_badge,
        Span::raw(format!(
            " | IPC {}:{} | Uptime: {}s | Modo: ",
            cfg.host, cfg.port, snap.uptime_secs
        )),
        Span::styled(
            snap.mode.name(),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            " | Energia: {:.2} TeV (Pico: {:.2} TeV)",
            snap.beam_energy_tev, snap.peak_energy_tev
        )),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    f.render_widget(Paragraph::new(line).block(block), area);
}

fn draw_collider_ring_canvas(
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
        " Anel Síncrotron & Calorímetro [v = 0.{}c | γ = {:.1}] ",
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
        .x_bounds([-110.0, 110.0])
        .y_bounds([-110.0, 110.0])
        .paint(|ctx| {
            // Anel externo e tubo de vácuo interno do acelerador
            ctx.draw(&Circle {
                x: 0.0,
                y: 0.0,
                radius: 78.0,
                color: ring_color,
            });
            ctx.draw(&Circle {
                x: 0.0,
                y: 0.0,
                radius: 70.0,
                color: Color::DarkGray,
            });

            // Cavidades RF e ímãs quadrupolos ao redor do anel
            for i in 0..8 {
                let a = (i as f64) * std::f64::consts::FRAC_PI_4;
                let x1 = 66.0 * a.cos();
                let y1 = 66.0 * a.sin();
                let x2 = 82.0 * a.cos();
                let y2 = 82.0 * a.sin();
                let magnet_color = if i % 2 == 0 {
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

            // Ondas de sobrecarga quando o Pulso Supercrítico ou Ressonância Quântica está ativo
            if snap.burst_ticks_remaining > 0 || snap.mode == AccelerationMode::QuantumResonance {
                let pulse_r = 35.0 + ((anim.phase_angle * 3.0).sin().abs() * 25.0);
                ctx.draw(&Circle {
                    x: 0.0,
                    y: 0.0,
                    radius: pulse_r,
                    color: Color::Magenta,
                });
            }

            // Feixe 1 (Sentido anti-horário) + rastro Cherenkov
            let r_beam = 74.0;
            let mut beam1_trail = Vec::with_capacity(18);
            let mut beam2_trail = Vec::with_capacity(18);
            for k in 0..18 {
                let offset = (k as f64) * 0.07;
                let a1 = anim.phase_angle - offset;
                let a2 = -anim.phase_angle + std::f64::consts::PI + offset;
                beam1_trail.push((r_beam * a1.cos(), r_beam * a1.sin()));
                beam2_trail.push((r_beam * a2.cos(), r_beam * a2.sin()));
            }
            ctx.draw(&Points {
                coords: &beam1_trail[2..],
                color: Color::Cyan,
            });
            ctx.draw(&Points {
                coords: &beam2_trail[2..],
                color: Color::LightRed,
            });
            // Cabeças dos pacotes de prótons (bunches)
            ctx.draw(&Points {
                coords: &beam1_trail[0..2],
                color: Color::White,
            });
            ctx.draw(&Points {
                coords: &beam2_trail[0..2],
                color: Color::Yellow,
            });

            // Câmara central do Detector / Ponto de Interação (0, 0)
            ctx.draw(&Circle {
                x: 0.0,
                y: 0.0,
                radius: 14.0,
                color: Color::DarkGray,
            });

            // Linhas de injeção para o detector central
            ctx.draw(&CanvasLine {
                x1: 0.0,
                y1: 74.0,
                x2: 0.0,
                y2: 14.0,
                color: Color::DarkGray,
            });
            ctx.draw(&CanvasLine {
                x1: 0.0,
                y1: -74.0,
                x2: 0.0,
                y2: -14.0,
                color: Color::DarkGray,
            });

            // Renderiza os jatos e trajetórias curvas da última colisão no detector central
            if let Some(ref shower) = snap.latest_shower {
                let reach = (anim.shower_age * 52.0).clamp(6.0, 62.0);
                for track in &shower.tracks {
                    let color = match track.kind {
                        TrackKind::HadronJet => Color::LightGreen,
                        TrackKind::MuonCurve => Color::LightCyan,
                        TrackKind::PhotonRay => Color::LightYellow,
                        TrackKind::QuantumAnomaly => Color::LightMagenta,
                    };

                    // Desenha a trajetória curvada pelo campo magnético solenoide em 6 segmentos
                    let max_r = reach * track.speed * track.energy_ratio.max(0.4);
                    let steps = 6;
                    let mut prev_x = 0.0;
                    let mut prev_y = 0.0;
                    for s in 1..=steps {
                        let frac = (s as f64) / (steps as f64);
                        let r = max_r * frac;
                        let deflected_angle = track.angle_rad + track.curvature * frac * 0.85;
                        let nx = r * deflected_angle.cos();
                        let ny = r * deflected_angle.sin();
                        ctx.draw(&CanvasLine {
                            x1: prev_x,
                            y1: prev_y,
                            x2: nx,
                            y2: ny,
                            color,
                        });
                        prev_x = nx;
                        prev_y = ny;
                    }
                }

                if shower.is_quantum_anomaly {
                    ctx.draw(&Circle {
                        x: 0.0,
                        y: 0.0,
                        radius: (anim.shower_age * 42.0).clamp(5.0, 58.0),
                        color: Color::LightMagenta,
                    });
                }
            }
        });

    f.render_widget(canvas, area);
}

fn draw_cpu_and_kinematics(f: &mut Frame, area: Rect, snap: &TelemetrySnapshot) {
    let nines_count = snap.nines_of_c.floor() as usize;
    let beta_str = if nines_count <= 9 {
        format!("0.{}c ({:.2} noves)", "9".repeat(nines_count.max(2)), snap.nines_of_c)
    } else {
        format!("0.999999999...c ({:.2} noves de c!)", snap.nines_of_c)
    };

    let text = vec![
        Line::from(vec![
            Span::styled(
                " Processamento CPU : ",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:.1} kH/s", snap.hashes_per_sec / 1000.0),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                " | {:.2} GFLOPS | Total: {:.2}M hashes",
                snap.gflops,
                snap.total_compute_cycles as f64 / 1e6
            )),
        ]),
        Line::from(vec![
            Span::styled(" Recursos Alocados : ", Style::default().fg(Color::Cyan)),
            Span::raw(format!(
                "{}/{} Threads [+/-] | Intensidade: {}% [[/]] | CPU Sistema: {:.1}%",
                snap.active_threads, snap.max_threads, snap.intensity_pct, snap.system_cpu_pct
            )),
        ]),
        Line::from(vec![
            Span::styled(
                " Velocidade Feixe  : ",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                beta_str,
                Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Fator Lorentz (γ) : ", Style::default().fg(Color::Yellow)),
            Span::raw(format!(
                "γ = {:.2} | Energia: {:.3} TeV (Pico: {:.3} TeV)",
                snap.lorentz_gamma, snap.beam_energy_tev, snap.peak_energy_tev
            )),
        ]),
        Line::from(vec![
            Span::styled(" Taxa de Colisões  : ", Style::default().fg(Color::Green)),
            Span::raw(format!(
                "{:.0} colisões/s | Total: {} | Luminosidade: {:.3} fb⁻¹",
                snap.collisions_per_sec, snap.total_collisions, snap.luminosity_fb_inv
            )),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" CPU <-> Velocidade Relativística (Benchmark Real) ");
    f.render_widget(Paragraph::new(text).block(block), area);
}

fn draw_tuning_and_capacitor(f: &mut Frame, area: Rect, snap: &TelemetrySnapshot) {
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
        .title(" Como Acelera :: Sintonia de Cavidades RF & Quadrupolos ");
    f.render_widget(outer, area);

    let mode_line = Line::from(vec![
        Span::styled(" Modo [M]      : ", Style::default().fg(Color::Magenta)),
        Span::styled(
            snap.mode.name(),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" — {}", snap.mode.description())),
    ]);
    f.render_widget(Paragraph::new(mode_line), rows[0]);

    let focus_line = Line::from(vec![
        Span::styled(" Foco Ímã [←/→]: ", Style::default().fg(Color::Cyan)),
        Span::raw(format!(
            "{:.2} T/m | Estabilidade do Feixe: {:.1}%",
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
        Span::styled(" Harmônico [↑/↓]: ", Style::default().fg(Color::Cyan)),
        Span::styled(
            format!("{:.2} GHz", snap.rf_harmonic),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            " (Alvo Ressonante: {:.2} GHz) | Sintonia: ",
            snap.optimal_harmonic_hint
        )),
        Span::styled(
            format!("{:.1}%", snap.harmonic_alignment_pct),
            Style::default().fg(align_color).add_modifier(Modifier::BOLD),
        ),
    ]);
    f.render_widget(Paragraph::new(rf_line), rows[2]);

    let cap_label = if snap.burst_ticks_remaining > 0 {
        format!(
            "PULSO SUPERCRÍTICO EM DESCARGA! ({:.1}s restantes - 3.25x TeV & Quântico)",
            snap.burst_ticks_remaining as f64 * 0.1
        )
    } else {
        format!(
            "Capacitor de Pulso [ESPAÇO]: {:.1}% (Mínimo 25% para disparar sobrecarga)",
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
}

fn draw_quantum_codex(f: &mut Frame, area: Rect, snap: &TelemetrySnapshot) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2), // Barra de Proximidade Quântica / 5σ
            Constraint::Min(3),    // Lista de partículas descobertas e próximos alvos
        ])
        .split(area);

    let title = format!(
        " Proximidade Quântica: {:.1}% | Códice de Descobertas ({}/{}) ",
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
        "Próximo Alvo: {} ({}) [Limiar {:.2} TeV] -> Significância {:.2}σ / 5.00σ (Chance/s: {:.2}%)",
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

    let mut items = Vec::with_capacity(QUANTUM_CATALOG.len());
    for spec in QUANTUM_CATALOG {
        if let Some(found) = snap.discoveries.iter().find(|d| d.id == spec.id) {
            items.push(ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" ★ [T{}] {:<7} ", spec.tier, found.symbol),
                    Style::default()
                        .fg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:<32}", found.name),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!(
                        " | {:>7.2} GeV | {:.2}σ | x{}",
                        found.mass_gev, found.best_sigma, found.count
                    ),
                    Style::default().fg(Color::Cyan),
                ),
            ])));
        } else {
            let unlocked_energy = snap.beam_energy_tev >= spec.min_energy_tev;
            let color = if unlocked_energy {
                Color::Yellow
            } else {
                Color::DarkGray
            };
            items.push(ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" ○ [T{}] {:<7} ", spec.tier, spec.symbol),
                    Style::default().fg(color),
                ),
                Span::styled(
                    format!("{:<32}", spec.name),
                    Style::default().fg(color),
                ),
                Span::styled(
                    format!(
                        " | Requer {:>6.2} TeV | Harmônico {:.1} GHz",
                        spec.min_energy_tev, spec.resonant_harmonic
                    ),
                    Style::default().fg(color),
                ),
            ])));
        }
    }

    f.render_widget(List::new(items), chunks[1]);
}

fn draw_footer_logs_and_controls(f: &mut Frame, area: Rect, snap: &TelemetrySnapshot) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
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
            .title(" Telemetria & Eventos do Detector "),
    );
    f.render_widget(logs_widget, cols[0]);

    let controls = Paragraph::new(vec![
        Line::from(" [M] Alternar Modo   | [ESPAÇO] Pulso Supercrítico"),
        Line::from(" [+/-] Threads CPU   | [[ / ]] Intensidade CPU %"),
        Line::from(" [↑/↓] Harmônico RF  | [← / →] Foco Magnético"),
        Line::from(" [R] Reiniciar Salvo | [Q / ESC] Fechar Animação"),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Controles Remotos (IPC -> Motor) "),
    );
    f.render_widget(controls, cols[1]);
}
