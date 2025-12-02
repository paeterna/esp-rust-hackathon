mod protocol;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use num_complex::Complex;
use protocol::{CsiDataPacket, ParsedFrame, ProtocolHandler};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        Axis, Block, Borders, Chart, Dataset, GraphType, List, ListItem, Paragraph, Wrap,
    },
    Frame, Terminal,
};
use serialport::SerialPort;
use std::fs::File;
use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(name = "esp-csi-tui-rs")]
#[command(about = "ESP32 CSI Data Visualization TUI", long_about = None)]
struct Cli {
    /// Serial port path
    #[arg(short, long, default_value = "/dev/ttyUSB0")]
    port: String,

    /// Baud rate
    #[arg(short, long, default_value = "115200")]
    baud: u32,

    /// Mock data mode (no serial connection needed)
    #[arg(short, long)]
    mock: bool,

    /// Output CSV file
    #[arg(short, long)]
    csv: Option<String>,

    /// Output RRD file (Rerun format)
    #[arg(short, long)]
    rrd: Option<String>,

    /// Enable Rerun live streaming
    #[arg(long)]
    rerun_live: bool,
}

#[derive(Debug, Clone)]
struct CsiSample {
    timestamp: u64,
    rssi: i8,
    subcarriers: Vec<Complex<f32>>,
}

impl CsiSample {
    fn from_packet(packet: CsiDataPacket) -> Self {
        Self {
            timestamp: packet.timestamp as u64,
            rssi: packet.rssi,
            subcarriers: packet.subcarriers,
        }
    }

    fn amplitude(&self) -> Vec<f32> {
        self.subcarriers.iter().map(|c| c.norm()).collect()
    }

    fn phase(&self) -> Vec<f32> {
        self.subcarriers.iter().map(|c| c.arg()).collect()
    }

    fn to_csv_row(&self) -> String {
        let mut row = format!("{},{}", self.timestamp, self.rssi);
        for sc in &self.subcarriers {
            row.push_str(&format!(",{},{}", sc.re, sc.im));
        }
        row
    }
}

enum ViewMode {
    Amplitude,
    Phase,
    Heatmap,
    Status,
}

struct App {
    running: bool,
    view_mode: ViewMode,
    csi_samples: Vec<CsiSample>,
    max_samples: usize,
    port_name: String,
    connected: bool,
    last_update: Instant,
    messages: Vec<String>,
    mock_mode: bool,
    mock_counter: u64,
    csv_file: Option<File>,
    samples_saved: usize,
    rec: Option<rerun::RecordingStream>,
    rrd_path: Option<String>,
}

impl App {
    fn new(port_name: String, mock_mode: bool, csv_path: Option<String>, rrd_path: Option<String>, rerun_live: bool) -> Result<Self> {
        let csv_file = if let Some(path) = csv_path {
            let mut file = File::create(&path)?;
            // Write CSV header
            let mut header = "timestamp,rssi".to_string();
            for i in 0..64 {
                header.push_str(&format!(",sc{}_re,sc{}_im", i, i));
            }
            writeln!(file, "{}", header)?;
            Some(file)
        } else {
            None
        };

        // Initialize Rerun recording stream
        let rec = if rrd_path.is_some() || rerun_live {
            let mut builder = rerun::RecordingStreamBuilder::new("esp-csi-tui-rs");
            
            // If saving to file, use save mode; otherwise spawn viewer
            let rec = if let Some(ref path) = rrd_path {
                builder.save(path).ok()
            } else if rerun_live {
                builder.spawn().ok()
            } else {
                None
            };
            rec
        } else {
            None
        };

        Ok(Self {
            running: true,
            view_mode: ViewMode::Amplitude,
            csi_samples: Vec::new(),
            max_samples: 100,
            port_name,
            connected: !mock_mode,
            last_update: Instant::now(),
            messages: vec![
                "ESP32 CSI Data Collector".to_string(),
                "======================".to_string(),
                if mock_mode {
                    "Running in MOCK mode - generating simulated data".to_string()
                } else {
                    "Waiting for data...".to_string()
                },
            ],
            mock_mode,
            mock_counter: 0,
            csv_file,
            samples_saved: 0,
            rec,
            rrd_path,
        })
    }

    fn add_sample(&mut self, sample: CsiSample) {
        // Save to CSV if enabled
        if let Some(ref mut file) = self.csv_file {
            if let Err(e) = writeln!(file, "{}", sample.to_csv_row()) {
                self.add_message(format!("CSV write error: {}", e));
            } else {
                self.samples_saved += 1;
            }
        }

        // Log to Rerun if enabled
        if let Some(ref rec) = self.rec {
            // Set timeline
            rec.set_time_sequence("frame", sample.timestamp as i64);

            // Log amplitude data for all subcarriers
            let amplitudes: Vec<f64> = sample.amplitude().iter().map(|&a| a as f64).collect();
            let _ = rec.log(
                "csi/amplitude_scalar",
                &rerun::Scalar::new(amplitudes[0])
            );
            
            // Log phase data
            let phases: Vec<f64> = sample.phase().iter().map(|&p| p as f64).collect();
            let _ = rec.log(
                "csi/phase_scalar",
                &rerun::Scalar::new(phases[0])
            );

            // Log RSSI
            let _ = rec.log(
                "csi/rssi",
                &rerun::Scalar::new(sample.rssi as f64)
            );

            // Log subcarrier amplitudes as a line series
            let _points: Vec<(f64, f64)> = amplitudes.iter().enumerate()
                .map(|(i, &amp)| (i as f64, amp))
                .collect();
            
            let _ = rec.log(
                "csi/amplitude_spectrum",
                &rerun::SeriesLine::new()
                    .with_name("Amplitude")
            );
        }

        self.csi_samples.push(sample);
        if self.csi_samples.len() > self.max_samples {
            self.csi_samples.remove(0);
        }
    }

    fn add_message(&mut self, msg: String) {
        self.messages.push(msg);
        if self.messages.len() > 20 {
            self.messages.remove(0);
        }
    }

    fn save_rrd(&mut self) -> Result<()> {
        if let (Some(ref rec), Some(ref path)) = (&self.rec, &self.rrd_path) {
            rec.save(path)?;
            self.add_message(format!("RRD saved to: {}", path));
        }
        Ok(())
    }

    fn generate_mock_sample(&mut self) -> CsiSample {
        use std::f32::consts::PI;
        
        self.mock_counter += 1;
        let t = self.mock_counter as f32 * 0.1;
        
        // Generate 64 subcarriers with realistic WiFi CSI patterns
        let mut subcarriers = Vec::new();
        for i in 0..64 {
            let freq = i as f32 / 64.0;
            // Simulate multipath fading with multiple sine waves
            let amplitude = 10.0 + 5.0 * (t * 0.5 + freq * 2.0 * PI).sin() 
                          + 3.0 * (t * 0.3 + freq * 4.0 * PI).cos()
                          + 2.0 * (t * 0.7 + freq * PI).sin();
            let phase = freq * PI + t * 0.2;
            
            subcarriers.push(Complex::new(
                amplitude * phase.cos(),
                amplitude * phase.sin(),
            ));
        }

        CsiSample {
            timestamp: self.mock_counter,
            rssi: -40 + ((t * 0.5).sin() * 10.0) as i8,
            subcarriers,
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(0),     // Main content
            Constraint::Length(10), // Log messages
        ])
        .split(f.area());

    // Header
    render_header(f, chunks[0], app);

    // Main content based on view mode
    match app.view_mode {
        ViewMode::Amplitude => render_amplitude_chart(f, chunks[1], app),
        ViewMode::Phase => render_phase_chart(f, chunks[1], app),
        ViewMode::Heatmap => render_heatmap(f, chunks[1], app),
        ViewMode::Status => render_status(f, chunks[1], app),
    }

    // Log messages
    render_messages(f, chunks[2], app);
}

fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let mode_text = match app.view_mode {
        ViewMode::Amplitude => "AMPLITUDE",
        ViewMode::Phase => "PHASE",
        ViewMode::Heatmap => "HEATMAP",
        ViewMode::Status => "STATUS",
    };

    let connection_status = if app.mock_mode {
        "MOCK MODE"
    } else if app.connected {
        "CONNECTED"
    } else {
        "DISCONNECTED"
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "ESP32-C3 CSI Viewer",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::styled(mode_text, Style::default().fg(Color::Yellow)),
        Span::raw(" | "),
        Span::styled(
            connection_status,
            Style::default().fg(if app.connected || app.mock_mode {
                Color::Green
            } else {
                Color::Red
            }),
        ),
        Span::raw(" | "),
        Span::raw(format!("Samples: {}", app.csi_samples.len())),
        Span::raw(if app.csv_file.is_some() {
            format!(" | Saved: {}", app.samples_saved)
        } else {
            "".to_string()
        }),
        Span::raw(" | [A]mplitude [P]hase [H]eatmap [S]tatus [Q]uit"),
    ]))
    .block(Block::default().borders(Borders::ALL));

    f.render_widget(header, area);
}

fn render_amplitude_chart(f: &mut Frame, area: Rect, app: &App) {
    if app.csi_samples.is_empty() {
        let text = Paragraph::new("No data yet... waiting for samples")
            .block(Block::default().borders(Borders::ALL).title("Amplitude"))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(text, area);
        return;
    }

    // Get the latest sample
    let latest = &app.csi_samples[app.csi_samples.len() - 1];
    let amplitudes = latest.amplitude();

    // Create data points
    let data: Vec<(f64, f64)> = amplitudes
        .iter()
        .enumerate()
        .map(|(i, &amp)| (i as f64, amp as f64))
        .collect();

    let datasets = vec![Dataset::default()
        .name("Amplitude")
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Cyan))
        .data(&data)];

    let max_amp = amplitudes
        .iter()
        .fold(0.0f32, |max, &val| max.max(val))
        .ceil();

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("CSI Amplitude (RSSI: {} dBm)", latest.rssi)),
        )
        .x_axis(
            Axis::default()
                .title("Subcarrier")
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, 63.0])
                .labels(vec![
                    Span::raw("0"),
                    Span::raw("32"),
                    Span::raw("63"),
                ]),
        )
        .y_axis(
            Axis::default()
                .title("Amplitude")
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, max_amp as f64])
                .labels(vec![
                    Span::raw("0"),
                    Span::raw(format!("{:.1}", max_amp / 2.0)),
                    Span::raw(format!("{:.1}", max_amp)),
                ]),
        );

    f.render_widget(chart, area);
}

fn render_heatmap(f: &mut Frame, area: Rect, app: &App) {
    if app.csi_samples.is_empty() {
        let text = Paragraph::new("No data yet... waiting for samples")
            .block(Block::default().borders(Borders::ALL).title("Heatmap"))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(text, area);
        return;
    }

    // Create a simplified heatmap using colored bars
    // Take last 20 samples for time axis
    let sample_count = app.csi_samples.len().min(20);
    let samples = &app.csi_samples[app.csi_samples.len() - sample_count..];

    // Calculate dimensions
    let inner = area.inner(ratatui::layout::Margin { vertical: 1, horizontal: 1 });

    // For simplicity, show amplitude as colored text
    let mut lines = vec![];
    lines.push(Line::from(Span::styled(
        "CSI Amplitude Heatmap (Time vs Subcarrier)",
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    for (i, sample) in samples.iter().enumerate() {
        let amplitudes = sample.amplitude();
        let max_amp = amplitudes.iter().fold(0.0f32, |max, &val| max.max(val));
        
        let mut line_spans = vec![Span::raw(format!("{:3}: ", i))];
        
        for amp in amplitudes.iter().step_by(2) {
            let normalized = (amp / max_amp * 255.0) as u8;
            let color = if normalized > 200 {
                Color::Red
            } else if normalized > 150 {
                Color::Yellow
            } else if normalized > 100 {
                Color::Green
            } else if normalized > 50 {
                Color::Cyan
            } else {
                Color::Blue
            };
            line_spans.push(Span::styled("█", Style::default().fg(color)));
        }
        
        lines.push(Line::from(line_spans));
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Heatmap View"));

    f.render_widget(paragraph, area);
}

fn render_phase_chart(f: &mut Frame, area: Rect, app: &App) {
    if app.csi_samples.is_empty() {
        let text = Paragraph::new("No data yet... waiting for samples")
            .block(Block::default().borders(Borders::ALL).title("Phase"))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(text, area);
        return;
    }

    let latest = &app.csi_samples[app.csi_samples.len() - 1];
    let phases = latest.phase();

    let data: Vec<(f64, f64)> = phases
        .iter()
        .enumerate()
        .map(|(i, &phase)| (i as f64, phase as f64))
        .collect();

    let datasets = vec![Dataset::default()
        .name("Phase")
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Magenta))
        .data(&data)];

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("CSI Phase (RSSI: {} dBm)", latest.rssi)),
        )
        .x_axis(
            Axis::default()
                .title("Subcarrier")
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, 63.0])
                .labels(vec![
                    Span::raw("0"),
                    Span::raw("32"),
                    Span::raw("63"),
                ]),
        )
        .y_axis(
            Axis::default()
                .title("Phase (radians)")
                .style(Style::default().fg(Color::Gray))
                .bounds([-3.14, 3.14])
                .labels(vec![
                    Span::raw("-π"),
                    Span::raw("0"),
                    Span::raw("π"),
                ]),
        );

    f.render_widget(chart, area);
}

fn render_status(f: &mut Frame, area: Rect, app: &App) {
    let status_text = vec![
        format!("Port: {}", app.port_name),
        format!("Connected: {}", if app.connected || app.mock_mode { "Yes" } else { "No" }),
        format!("Mock Mode: {}", if app.mock_mode { "Yes" } else { "No" }),
        format!("Total Samples: {}", app.csi_samples.len()),
        format!("Samples Saved: {}", app.samples_saved),
        format!("Max Samples: {}", app.max_samples),
        format!("CSV Export: {}", if app.csv_file.is_some() { "Enabled" } else { "Disabled" }),
        format!("RRD Recording: {}", if app.rec.is_some() { "Enabled" } else { "Disabled" }),
        format!("RRD Path: {}", app.rrd_path.as_ref().unwrap_or(&"N/A".to_string())),
        "".to_string(),
        "Controls:".to_string(),
        "  [A] - Amplitude view".to_string(),
        "  [P] - Phase view".to_string(),
        "  [H] - Heatmap view".to_string(),
        "  [S] - Status view".to_string(),
        "  [Q] - Quit".to_string(),
    ];

    let paragraph = Paragraph::new(status_text.join("\n"))
        .block(Block::default().borders(Borders::ALL).title("Status"))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_messages(f: &mut Frame, area: Rect, app: &App) {
    let messages: Vec<ListItem> = app
        .messages
        .iter()
        .rev()
        .take(10)
        .rev()
        .map(|m| ListItem::new(m.clone()))
        .collect();

    let messages_list = List::new(messages)
        .block(Block::default().borders(Borders::ALL).title("Messages"));

    f.render_widget(messages_list, area);
}

fn handle_events(app: &mut App) -> Result<()> {
    if event::poll(Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => app.running = false,
                    KeyCode::Char('a') | KeyCode::Char('A') => {
                        app.view_mode = ViewMode::Amplitude;
                        app.add_message("Switched to Amplitude view".to_string());
                    }
                    KeyCode::Char('p') | KeyCode::Char('P') => {
                        app.view_mode = ViewMode::Phase;
                        app.add_message("Switched to Phase view".to_string());
                    }
                    KeyCode::Char('h') | KeyCode::Char('H') => {
                        app.view_mode = ViewMode::Heatmap;
                        app.add_message("Switched to Heatmap view".to_string());
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        app.view_mode = ViewMode::Status;
                        app.add_message("Switched to Status view".to_string());
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(cli.port.clone(), cli.mock, cli.csv.clone(), cli.rrd.clone(), cli.rerun_live)?;

    if cli.csv.is_some() {
        app.add_message(format!("CSV export enabled: {}", cli.csv.as_ref().unwrap()));
    }
    
    if cli.rrd.is_some() {
        app.add_message(format!("RRD export enabled: {}", cli.rrd.as_ref().unwrap()));
    }
    
    if cli.rerun_live {
        app.add_message("Rerun live streaming enabled".to_string());
    }

    // Try to open serial port if not in mock mode
    let mut port: Option<Box<dyn SerialPort>> = if !cli.mock {
        match serialport::new(&cli.port, cli.baud)
            .timeout(Duration::from_millis(100))
            .open()
        {
            Ok(p) => {
                app.add_message(format!("Connected to {}", cli.port));
                Some(p)
            }
            Err(e) => {
                app.add_message(format!("Failed to open port: {}", e));
                app.add_message("Switching to MOCK mode...".to_string());
                app.mock_mode = true;
                app.connected = false;
                None
            }
        }
    } else {
        app.add_message("Running in MOCK mode".to_string());
        None
    };

    let mut protocol = ProtocolHandler::new();

    // Send start command if connected
    if let Some(ref mut p) = port {
        let cmd = protocol.create_start_csi_command();
        if let Err(e) = p.write_all(&cmd) {
            app.add_message(format!("Failed to send START command: {}", e));
        } else {
            app.add_message("Sent START_CSI command".to_string());
        }
    }

    // Main loop
    let mut last_sample_time = Instant::now();
    while app.running {
        terminal.draw(|f| ui(f, &app))?;
        handle_events(&mut app)?;

        // Generate/read data every 100ms
        if last_sample_time.elapsed() > Duration::from_millis(100) {
            if app.mock_mode {
                let sample = app.generate_mock_sample();
                app.add_sample(sample);
            } else if let Some(ref mut p) = port {
                // Read from serial port
                let mut buf = [0u8; 512];
                match p.read(&mut buf) {
                    Ok(n) if n > 0 => {
                        protocol.add_data(&buf[..n]);
                        
                        // Try to parse frames
                        loop {
                            match protocol.try_parse_frame() {
                                Ok(Some(ParsedFrame::CsiData(packet))) => {
                                    let sample = CsiSample::from_packet(packet);
                                    app.add_message(format!("Received CSI data: {} subcarriers", sample.subcarriers.len()));
                                    app.add_sample(sample);
                                }
                                Ok(Some(ParsedFrame::Ack)) => {
                                    app.add_message("Received ACK".to_string());
                                }
                                Ok(Some(ParsedFrame::Nak)) => {
                                    app.add_message("Received NAK".to_string());
                                }
                                Ok(Some(_)) => {}
                                Ok(None) => break,
                                Err(e) => {
                                    app.add_message(format!("Parse error: {}", e));
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) if e.kind() != std::io::ErrorKind::TimedOut => {
                        app.add_message(format!("Serial read error: {}", e));
                    }
                    _ => {}
                }
            }
            last_sample_time = Instant::now();
        }
    }

    // Send stop command if connected
    if let Some(ref mut p) = port {
        let cmd = protocol.create_stop_csi_command();
        let _ = p.write_all(&cmd);
    }

    // Flush and close Rerun recording (saves automatically for file mode)
    if app.rec.is_some() {
        drop(app.rec.take());  // Explicitly drop to flush recording
        if app.rrd_path.is_some() {
            println!("RRD file saved (auto-flush on close)");
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    println!("ESP32-C3 CSI Viewer closed. {} samples collected.", app.csi_samples.len());
    if app.csv_file.is_some() {
        println!("CSV data saved: {} samples", app.samples_saved);
    }
    if app.rrd_path.is_some() {
        println!("RRD data saved to: {}", app.rrd_path.as_ref().unwrap());
    }

    Ok(())
}
