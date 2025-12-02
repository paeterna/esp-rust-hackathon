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

    /// Maximum number of samples to keep in memory for visualization (default: 500)
    #[arg(long, default_value = "500")]
    max_samples: usize,
}

#[derive(Debug, Clone)]
struct CsiSample {
    timestamp: u64,
    rssi: i8,
    subcarriers: Vec<Complex<f32>>,
}

impl CsiSample {
    /// Count how many subcarriers have non-zero values
    fn count_nonzero_subcarriers(&self) -> usize {
        self.subcarriers
            .iter()
            .filter(|c| c.norm() > 0.01)  // Threshold to avoid floating point errors
            .count()
    }
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
    Motion,
}

enum HeatmapMode {
    ColorBlocks,  // Press 1: colored blocks only
    Detailed,     // Press 2: amplitude and phase numbers
}

#[derive(Debug, Clone)]
struct MotionEvent {
    timestamp: u64,
    confidence: f32,
    rssi_delta: i8,
    affected_subcarriers: usize,
    avg_amplitude_change: f32,
}

struct MotionDetector {
    // Configurable thresholds
    rssi_threshold: f32,           // Minimum RSSI change to detect motion (dBm)
    amplitude_variance_threshold: f32, // Minimum variance in amplitude
    min_affected_subcarriers: usize,   // Minimum number of subcarriers that must change

    // Detection history
    motion_events: Vec<MotionEvent>,
    last_baseline: Option<CsiSample>,
    motion_score_history: Vec<f32>,

    // Motion indicator persistence
    last_motion_time: Option<Instant>,
    motion_display_active: bool,
}

impl MotionDetector {
    fn new() -> Self {
        Self {
            rssi_threshold: 30.0,
            amplitude_variance_threshold: 45.0,
            min_affected_subcarriers: 10,
            motion_events: Vec::new(),
            last_baseline: None,
            motion_score_history: Vec::with_capacity(100),
            last_motion_time: None,
            motion_display_active: false,
        }
    }

    fn update_display_state(&mut self) {
        // Check if motion indicator should still be active (1 second persistence)
        if let Some(last_time) = self.last_motion_time {
            if last_time.elapsed() < Duration::from_secs(1) {
                self.motion_display_active = true;
            } else {
                self.motion_display_active = false;
            }
        } else {
            self.motion_display_active = false;
        }
    }

    fn is_motion_displayed(&self) -> bool {
        self.motion_display_active
    }

    fn detect(&mut self, current: &CsiSample, previous: Option<&CsiSample>) -> Option<MotionEvent> {
        let prev = match previous {
            Some(p) => p,
            None => {
                self.last_baseline = Some(current.clone());
                return None;
            }
        };

        // Calculate RSSI change
        let rssi_delta = (current.rssi - prev.rssi).abs();

        // Calculate per-subcarrier amplitude changes
        let mut amplitude_changes = Vec::new();
        let mut affected_count = 0;

        for i in 0..current.subcarriers.len() {
            let curr_amp = current.subcarriers[i].norm();
            let prev_amp = prev.subcarriers[i].norm();
            let delta = (curr_amp - prev_amp).abs();

            amplitude_changes.push(delta);

            if delta > self.amplitude_variance_threshold {
                affected_count += 1;
            }
        }

        // Calculate average amplitude change
        let avg_change: f32 = amplitude_changes.iter().sum::<f32>() / amplitude_changes.len() as f32;

        // Calculate variance
        let mean = avg_change;
        let variance: f32 = amplitude_changes.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f32>() / amplitude_changes.len() as f32;

        // Calculate confidence score (0-100)
        let rssi_score = ((rssi_delta as f32 / self.rssi_threshold).min(1.0) * 30.0).round();
        let variance_score = ((variance / (self.amplitude_variance_threshold * 10.0)).min(1.0) * 40.0).round();
        let subcarrier_score = ((affected_count as f32 / self.min_affected_subcarriers as f32).min(1.0) * 30.0).round();

        let confidence = rssi_score + variance_score + subcarrier_score;

        // Store motion score
        self.motion_score_history.push(confidence);
        if self.motion_score_history.len() > 100 {
            self.motion_score_history.remove(0);
        }

        // Detect motion if confidence is high enough
        if confidence > 30.0 {
            let event = MotionEvent {
                timestamp: current.timestamp,
                confidence,
                rssi_delta: rssi_delta as i8,
                affected_subcarriers: affected_count,
                avg_amplitude_change: avg_change,
            };

            self.motion_events.push(event.clone());
            if self.motion_events.len() > 50 {
                self.motion_events.remove(0);
            }

            // Update motion display timer
            self.last_motion_time = Some(Instant::now());
            self.motion_display_active = true;

            Some(event)
        } else {
            None
        }
    }
}

struct App {
    running: bool,
    view_mode: ViewMode,
    heatmap_mode: HeatmapMode,
    csi_samples: Vec<CsiSample>,
    max_samples: usize,
    port_name: String,
    connected: bool,
    messages: Vec<String>,
    mock_mode: bool,
    mock_counter: u64,
    csv_file: Option<File>,
    samples_saved: usize,
    rec: Option<rerun::RecordingStream>,
    rrd_path: Option<String>,
    motion_detector: MotionDetector,
}

impl App {
    fn new(port_name: String, mock_mode: bool, csv_path: Option<String>, rrd_path: Option<String>, rerun_live: bool, max_samples: usize) -> Result<Self> {
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
            let builder = rerun::RecordingStreamBuilder::new("esp-csi-tui-rs");

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
            heatmap_mode: HeatmapMode::ColorBlocks,
            csi_samples: Vec::with_capacity(max_samples),
            max_samples,
            port_name,
            connected: !mock_mode,
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
            motion_detector: MotionDetector::new(),
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

        // Run motion detection
        let previous = if self.csi_samples.len() > 0 {
            Some(&self.csi_samples[self.csi_samples.len() - 1])
        } else {
            None
        };

        if let Some(event) = self.motion_detector.detect(&sample, previous) {
            self.add_message(format!(
                "🚶 Motion detected! Confidence: {:.0}%, RSSI Δ: {}, SCs: {}",
                event.confidence, event.rssi_delta, event.affected_subcarriers
            ));
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

    #[allow(dead_code)]
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
        ViewMode::Motion => render_motion_detection(f, chunks[1], app),
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
        ViewMode::Motion => "MOTION DETECTION",
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
        Span::raw(" | [A]mplitude [P]hase [H]eatmap [M]otion [S]tatus [Q]uit"),
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

    // Separate valid and null subcarriers for visualization
    // HT20 guard band is typically SC 28-35 (8 subcarriers around DC)
    let valid_data: Vec<(f64, f64)> = amplitudes
        .iter()
        .enumerate()
        .filter(|(_i, &amp)| amp > 0.01)  // Only non-zero
        .map(|(i, &amp)| (i as f64, amp as f64))
        .collect();

    let null_data: Vec<(f64, f64)> = amplitudes
        .iter()
        .enumerate()
        .filter(|(_i, &amp)| amp <= 0.01)  // Zero/null subcarriers
        .map(|(i, _)| (i as f64, 0.0))
        .collect();

    let mut datasets = vec![
        Dataset::default()
            .name("Valid CSI")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&valid_data),
    ];

    // Add markers for null subcarriers
    if !null_data.is_empty() {
        datasets.push(Dataset::default()
            .name("Null/Guard")
            .marker(symbols::Marker::Dot)
            .graph_type(GraphType::Scatter)
            .style(Style::default().fg(Color::Red))
            .data(&null_data));
    }

    let max_amp = amplitudes
        .iter()
        .fold(0.0f32, |max, &val| max.max(val))
        .ceil();

    // Calculate stats for the problematic range (SC 27-35)
    let mid_range_amps: Vec<f32> = amplitudes[27..=35].to_vec();
    let mid_avg = mid_range_amps.iter().sum::<f32>() / mid_range_amps.len() as f32;
    let mid_min = mid_range_amps.iter().fold(f32::MAX, |min, &val| min.min(val));
    let mid_max = mid_range_amps.iter().fold(f32::MIN, |max, &val| max.max(val));

    // Count non-zero subcarriers
    let nonzero_count = latest.count_nonzero_subcarriers();

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    "CSI (RSSI: {} dBm) | {}/64 non-zero | SC27-35: {:.1}-{:.1} avg={:.1}",
                    latest.rssi, nonzero_count, mid_min, mid_max, mid_avg
                )),
        )
        .x_axis(
            Axis::default()
                .title("Subcarrier Index")
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, 63.0])
                .labels(vec![
                    Span::raw("0"),
                    Span::raw("27"),
                    Span::raw("32"),
                    Span::raw("35"),
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

    match app.heatmap_mode {
        HeatmapMode::ColorBlocks => render_heatmap_color_blocks(f, area, app),
        HeatmapMode::Detailed => render_heatmap_detailed(f, area, app),
    }
}

fn render_heatmap_color_blocks(f: &mut Frame, area: Rect, app: &App) {
    // Prepare samples (time axis) and subcarriers (frequency axis)
    let sample_count = app.csi_samples.len();
    let subcarrier_count = app.csi_samples.last().unwrap().subcarriers.len();

    // Compute available drawing area and cell sizing
    let area_width = area.width as usize;
    let area_height = area.height as usize;

    // Reserve space for labels: left label column only (no top header)
    let left_label_width = 6; // e.g. "scXX: "
    let top_label_height = 2; // Info line + legend line

    // Each cell is now just 2 characters wide for a colored block
    let cell_w = 2usize; // "██" per cell
    let available_w = if area_width > left_label_width { area_width - left_label_width } else { 0 };
    let max_cols = if cell_w > 0 { available_w / cell_w } else { 0 };
    let cols = std::cmp::min(max_cols, sample_count).max(1);

    // Rows calculation: leave space for title
    let available_h = if area_height > top_label_height { area_height - top_label_height } else { 0 };
    let rows = std::cmp::min(available_h, subcarrier_count).max(1);

    // Select the most recent `cols` samples (time axis, left=old, right=new)
    let start_sample = if sample_count > cols { sample_count - cols } else { 0 };
    let samples = &app.csi_samples[start_sample..];

    // Choose stepping for subcarriers to fit into `rows`
    let step = (subcarrier_count + rows - 1) / rows; // ceil division

    // Compute global max amplitude in the selected window for normalization
    let mut global_max_amp = 0.0f32;
    for sample in samples {
        for sc_index in (0..subcarrier_count).step_by(step) {
            let amp = sample.subcarriers[sc_index].norm();
            if amp > global_max_amp {
                global_max_amp = amp;
            }
        }
    }
    if global_max_amp <= 0.0 {
        global_max_amp = 1.0; // avoid div by zero
    }

    // Build lines
    let mut lines: Vec<Line> = Vec::with_capacity(rows + 3);

    // Info line showing time range
    let oldest_ts = samples.first().unwrap().timestamp;
    let newest_ts = samples.last().unwrap().timestamp;
    let info_line = format!("Time: {} → {} ({} samples) | Press [1] Color [2] Detailed", oldest_ts, newest_ts, samples.len());
    lines.push(Line::from(Span::styled(info_line, Style::default().fg(Color::Gray))));

    // Color legend line
    let legend_spans = vec![
        Span::styled("Legend: ", Style::default().fg(Color::White)),
        Span::styled("██", Style::default().fg(Color::Red)),
        Span::styled(" >80% ", Style::default().fg(Color::Gray)),
        Span::styled("██", Style::default().fg(Color::Yellow)),
        Span::styled(" 60-80% ", Style::default().fg(Color::Gray)),
        Span::styled("██", Style::default().fg(Color::Green)),
        Span::styled(" 40-60% ", Style::default().fg(Color::Gray)),
        Span::styled("██", Style::default().fg(Color::Cyan)),
        Span::styled(" 20-40% ", Style::default().fg(Color::Gray)),
        Span::styled("██", Style::default().fg(Color::Blue)),
        Span::styled(" <20%", Style::default().fg(Color::Gray)),
    ];
    lines.push(Line::from(legend_spans));

    // For each row (representing a subcarrier index), construct a line with colored blocks
    let mut sc_idx = 0usize;
    while sc_idx < subcarrier_count {
        let mut spans = Vec::with_capacity(cols + 1);
        // Left label with subcarrier index
        spans.push(Span::styled(format!("sc{:02}: ", sc_idx), Style::default().fg(Color::Cyan)));

        for (_sample_i, sample) in samples.iter().enumerate() {
            // Clamp sc_idx to valid index
            let idx = sc_idx.min(subcarrier_count - 1);
            let c = &sample.subcarriers[idx];
            let amp = c.norm();

            // Normalize amplitude to 0..255 for color mapping
            let normalized = (amp / global_max_amp * 255.0).clamp(0.0, 255.0) as u8;
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

            // Use colored block characters
            spans.push(Span::styled("██", Style::default().fg(color)));
        }

        lines.push(Line::from(spans));

        sc_idx += step;
    }

    // Compose paragraph and render
    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Heatmap - Color Mode (Red=high→Blue=low)"));

    f.render_widget(paragraph, area);
}

fn render_heatmap_detailed(f: &mut Frame, area: Rect, app: &App) {
    // Prepare samples (time axis) and subcarriers (frequency axis)
    let sample_count = app.csi_samples.len();
    let subcarrier_count = app.csi_samples.last().unwrap().subcarriers.len();

    // Compute available drawing area and cell sizing
    let area_width = area.width as usize;
    let area_height = area.height as usize;

    // Reserve space for labels: left label column and top header
    let left_label_width = 6; // e.g. "scXX: "
    let top_label_height = 3; // Timestamp header + info line + legend

    // Determine number of columns (time) and rows (subcarriers) we can render
    // Each cell will take up cell_w characters horizontally for "A:XXX P:+X.XX"
    let cell_w = 13usize; // width per cell for "A:XXX P:+X.XX"
    let available_w = if area_width > left_label_width { area_width - left_label_width } else { 0 };
    let max_cols = if cell_w > 0 { available_w / cell_w } else { 0 };
    let cols = std::cmp::min(max_cols, sample_count).max(1);

    // Rows calculation: leave space for top labels
    let available_h = if area_height > top_label_height { area_height - top_label_height } else { 0 };
    let rows = std::cmp::min(available_h, subcarrier_count).max(1);

    // Select the most recent `cols` samples (time axis, left=old, right=new)
    let start_sample = if sample_count > cols { sample_count - cols } else { 0 };
    let samples = &app.csi_samples[start_sample..];

    // Choose stepping for subcarriers to fit into `rows`
    let step = (subcarrier_count + rows - 1) / rows; // ceil division

    // Compute global max amplitude in the selected window for normalization
    let mut global_max_amp = 0.0f32;
    for sample in samples {
        for sc_index in (0..subcarrier_count).step_by(step) {
            let amp = sample.subcarriers[sc_index].norm();
            if amp > global_max_amp {
                global_max_amp = amp;
            }
        }
    }
    if global_max_amp <= 0.0 {
        global_max_amp = 1.0; // avoid div by zero
    }

    // Build lines: top header with timestamps
    let mut lines: Vec<Line> = Vec::with_capacity(top_label_height + rows + 1);

    // Top header: empty left label area then column headers (time index or timestamp)
    let mut header_spans = vec![Span::raw(" ".repeat(left_label_width))];
    for s in samples.iter() {
        let ts = s.timestamp;
        let label = format!("{:>12} ", ts);
        header_spans.push(Span::styled(label, Style::default().fg(Color::Yellow)));
    }
    lines.push(Line::from(header_spans));

    // Info line
    let info_line = format!("{}Press [1] Color [2] Detailed", " ".repeat(left_label_width));
    lines.push(Line::from(Span::styled(info_line, Style::default().fg(Color::Gray))));

    // Color legend line (compact for detailed mode)
    let legend_spans = vec![
        Span::raw(" ".repeat(left_label_width)),
        Span::styled("Legend: ", Style::default().fg(Color::White)),
        Span::styled("█", Style::default().fg(Color::Red)),
        Span::styled(">80% ", Style::default().fg(Color::Gray)),
        Span::styled("█", Style::default().fg(Color::Yellow)),
        Span::styled("60-80% ", Style::default().fg(Color::Gray)),
        Span::styled("█", Style::default().fg(Color::Green)),
        Span::styled("40-60% ", Style::default().fg(Color::Gray)),
        Span::styled("█", Style::default().fg(Color::Cyan)),
        Span::styled("20-40% ", Style::default().fg(Color::Gray)),
        Span::styled("█", Style::default().fg(Color::Blue)),
        Span::styled("<20%", Style::default().fg(Color::Gray)),
    ];
    lines.push(Line::from(legend_spans));

    // For each row (representing a subcarrier index), construct a line with cells for each time sample
    let mut sc_idx = 0usize;
    while sc_idx < subcarrier_count {
        let mut spans = Vec::with_capacity(cols + 1);
        // Left label with subcarrier index
        spans.push(Span::styled(format!("sc{:02}: ", sc_idx), Style::default().fg(Color::Cyan)));

        for (_sample_i, sample) in samples.iter().enumerate() {
            // Clamp sc_idx to valid index
            let idx = sc_idx.min(subcarrier_count - 1);
            let c = &sample.subcarriers[idx];
            let amp = c.norm();
            let ph = c.arg();

            // Normalize amplitude to 0..255 for color mapping
            let normalized = (amp / global_max_amp * 255.0).clamp(0.0, 255.0) as u8;
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

            // Cell text: amplitude (rounded) and phase
            // Format to fit into `cell_w`: "A:XXX P:+X.XX "
            let cell_text = format!("A:{:>3.0} P:{:+.2} ", amp, ph);

            // Ensure fixed width by truncation/padding
            let mut cell_fixed = cell_text;
            if cell_fixed.len() > cell_w {
                cell_fixed.truncate(cell_w);
            } else {
                let pad = cell_w - cell_fixed.len();
                cell_fixed.push_str(&" ".repeat(pad));
            }

            spans.push(Span::styled(cell_fixed, Style::default().fg(color)));
        }

        lines.push(Line::from(spans));

        sc_idx += step;
    }

    // Compose paragraph and render
    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Heatmap - Detailed Mode (A=Amplitude, P=Phase)"));

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
    let mut status_text = vec![
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
    ];

    // Add subcarrier analysis if we have samples
    if let Some(latest) = app.csi_samples.last() {
        let nonzero = latest.count_nonzero_subcarriers();
        status_text.push(format!("CSI Info:"));
        status_text.push(format!("  Non-zero subcarriers: {}/64", nonzero));
        status_text.push(format!("  Null subcarriers: {}/64", 64 - nonzero));

        // Show which subcarriers are null
        let mut null_sc = vec![];
        for (i, sc) in latest.subcarriers.iter().enumerate() {
            if sc.norm() <= 0.01 {
                null_sc.push(i);
            }
        }

        if !null_sc.is_empty() {
            let null_str = if null_sc.len() <= 15 {
                format!("  Null SCs: {:?}", null_sc)
            } else {
                format!("  Null SCs: {:?}...", &null_sc[..15])
            };
            status_text.push(null_str);
        }
        status_text.push("".to_string());
    }

    status_text.extend(vec![
        "Controls:".to_string(),
        "  [A] - Amplitude view".to_string(),
        "  [P] - Phase view".to_string(),
        "  [H] - Heatmap view".to_string(),
        "  [M] - Motion Detection view".to_string(),
        "  [S] - Status view".to_string(),
        "  [1] - Heatmap: Color blocks".to_string(),
        "  [2] - Heatmap: Detailed values".to_string(),
        "".to_string(),
        "Motion Detection Controls:".to_string(),
        "  [+/-] - RSSI threshold".to_string(),
        "  [[/]] - Amplitude threshold".to_string(),
        "  [,/.] - Min subcarriers".to_string(),
        "  [Q] - Quit".to_string(),
    ]);

    let paragraph = Paragraph::new(status_text.join("\n"))
        .block(Block::default().borders(Borders::ALL).title("Status"))
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_motion_detection(f: &mut Frame, area: Rect, app: &App) {
    use ratatui::layout::{Constraint, Direction, Layout};
    use ratatui::widgets::{Block, Borders, Paragraph};

    // Split area into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // Configuration panel
            Constraint::Length(12), // Motion score graph
            Constraint::Min(10),    // Recent events list
        ])
        .split(area);

    // Split configuration panel into left and right sections
    let config_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70),  // Configuration text
            Constraint::Percentage(30),  // Motion indicator
        ])
        .split(chunks[0]);

    // Configuration Panel (Left side)
    let config_text = vec![
        format!("Motion Detection Configuration:"),
        format!(""),
        format!("  RSSI Threshold: {:.1} dBm         [+/-] to adjust", app.motion_detector.rssi_threshold),
        format!("  Amplitude Threshold: {:.1}        [[/]] to adjust", app.motion_detector.amplitude_variance_threshold),
        format!("  Min Subcarriers: {}              [,/.] to adjust", app.motion_detector.min_affected_subcarriers),
        format!(""),
        format!("Total Motion Events: {}", app.motion_detector.motion_events.len()),
    ];

    let config_panel = Paragraph::new(config_text.join("\n"))
        .block(Block::default().borders(Borders::ALL).title("Configuration"))
        .style(Style::default().fg(Color::White));
    f.render_widget(config_panel, config_chunks[0]);

    // Motion Indicator (Right side) - A visual rectangle with two states
    // Update display state based on timer
    let motion_active = app.motion_detector.is_motion_displayed();
    let current_score = app.motion_detector.motion_score_history.last().copied().unwrap_or(0.0);

    // Only two colors: Red for motion detected, Gray for no motion
    let (indicator_color, status_text, text_color) = if motion_active {
        (Color::Red, " MOTION! ", Color::White)
    } else {
        (Color::Gray, "NO MOTION", Color::DarkGray)
    };

    // Create the motion indicator display with a rectangular block
    let indicator_lines = vec![
        Line::from(Span::styled("", Style::default())),
        Line::from(Span::styled("█████████████", Style::default().fg(indicator_color).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("█████████████", Style::default().fg(indicator_color).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(format!("██{}██", status_text), Style::default().fg(text_color).bg(indicator_color).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("█████████████", Style::default().fg(indicator_color).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("█████████████", Style::default().fg(indicator_color).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("", Style::default())),
    ];

    let indicator_panel = Paragraph::new(indicator_lines)
        .block(Block::default().borders(Borders::ALL).title(format!("Status {:.0}%", current_score)))
        .alignment(ratatui::layout::Alignment::Center)
        .style(Style::default());
    f.render_widget(indicator_panel, config_chunks[1]);

    // Motion Score History Graph
    if app.motion_detector.motion_score_history.is_empty() {
        let empty_text = Paragraph::new("No motion data yet... waiting for samples")
            .block(Block::default().borders(Borders::ALL).title("Motion Score History"))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(empty_text, chunks[1]);
    } else {
        let scores = &app.motion_detector.motion_score_history;
        let max_score = scores.iter().fold(0.0f32, |max, &val| max.max(val)).max(100.0);

        // Create data points for the chart
        let data: Vec<(f64, f64)> = scores
            .iter()
            .enumerate()
            .map(|(i, &score)| (i as f64, score as f64))
            .collect();

        let dataset = Dataset::default()
            .name("Motion Score")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Green))
            .data(&data);

        let chart = Chart::new(vec![dataset])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Motion Score History (30% threshold)"),
            )
            .x_axis(
                Axis::default()
                    .title("Sample")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, scores.len() as f64]),
            )
            .y_axis(
                Axis::default()
                    .title("Confidence %")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, max_score as f64])
                    .labels(vec![
                        Span::raw("0"),
                        Span::raw("30"),
                        Span::raw("50"),
                        Span::raw("100"),
                    ]),
            );

        f.render_widget(chart, chunks[1]);
    }

    // Recent Motion Events List
    let events = &app.motion_detector.motion_events;
    if events.is_empty() {
        let empty_text = Paragraph::new("No motion events detected yet")
            .block(Block::default().borders(Borders::ALL).title("Recent Motion Events"))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(empty_text, chunks[2]);
    } else {
        let mut event_lines = vec![
            Line::from(Span::styled(
                format!("{:<12} {:<12} {:<10} {:<15} {:<15}",
                        "Timestamp", "Confidence", "RSSI Δ", "Affected SCs", "Avg Δ Amp"),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            )),
        ];

        // Show last 20 events
        for event in events.iter().rev().take(20) {
            let confidence_color = if event.confidence > 70.0 {
                Color::Red
            } else if event.confidence > 50.0 {
                Color::Yellow
            } else {
                Color::Green
            };

            event_lines.push(Line::from(vec![
                Span::raw(format!("{:<12} ", event.timestamp)),
                Span::styled(
                    format!("{:<11.0}%", event.confidence),
                    Style::default().fg(confidence_color)
                ),
                Span::raw(format!(" {:<9} ", event.rssi_delta)),
                Span::raw(format!("{:<15} ", event.affected_subcarriers)),
                Span::raw(format!("{:<.2}", event.avg_amplitude_change)),
            ]));
        }

        let events_para = Paragraph::new(event_lines)
            .block(Block::default().borders(Borders::ALL).title(format!("Recent Motion Events ({} total)", events.len())))
            .wrap(Wrap { trim: false });

        f.render_widget(events_para, chunks[2]);
    }
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
                    KeyCode::Char('m') | KeyCode::Char('M') => {
                        app.view_mode = ViewMode::Motion;
                        app.add_message("Switched to Motion Detection view".to_string());
                    }
                    KeyCode::Char('1') => {
                        app.heatmap_mode = HeatmapMode::ColorBlocks;
                        app.add_message("Heatmap: Color blocks mode".to_string());
                    }
                    KeyCode::Char('2') => {
                        app.heatmap_mode = HeatmapMode::Detailed;
                        app.add_message("Heatmap: Detailed values mode".to_string());
                    }
                    // Motion detection threshold controls
                    KeyCode::Char('+') | KeyCode::Char('=') => {
                        app.motion_detector.rssi_threshold += 0.5;
                        app.add_message(format!("RSSI threshold: {:.1} dBm", app.motion_detector.rssi_threshold));
                    }
                    KeyCode::Char('-') | KeyCode::Char('_') => {
                        app.motion_detector.rssi_threshold = (app.motion_detector.rssi_threshold - 0.5).max(0.5);
                        app.add_message(format!("RSSI threshold: {:.1} dBm", app.motion_detector.rssi_threshold));
                    }
                    KeyCode::Char('[') | KeyCode::Char('{') => {
                        app.motion_detector.amplitude_variance_threshold = (app.motion_detector.amplitude_variance_threshold - 1.0).max(1.0);
                        app.add_message(format!("Amplitude threshold: {:.1}", app.motion_detector.amplitude_variance_threshold));
                    }
                    KeyCode::Char(']') | KeyCode::Char('}') => {
                        app.motion_detector.amplitude_variance_threshold += 1.0;
                        app.add_message(format!("Amplitude threshold: {:.1}", app.motion_detector.amplitude_variance_threshold));
                    }
                    KeyCode::Char(',') | KeyCode::Char('<') => {
                        app.motion_detector.min_affected_subcarriers = app.motion_detector.min_affected_subcarriers.saturating_sub(1).max(1);
                        app.add_message(format!("Min subcarriers: {}", app.motion_detector.min_affected_subcarriers));
                    }
                    KeyCode::Char('.') | KeyCode::Char('>') => {
                        app.motion_detector.min_affected_subcarriers = (app.motion_detector.min_affected_subcarriers + 1).min(64);
                        app.add_message(format!("Min subcarriers: {}", app.motion_detector.min_affected_subcarriers));
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

    // Validate serial port BEFORE setting up terminal (so we can show error messages)
    if !cli.mock {
        match serialport::new(&cli.port, cli.baud)
            .timeout(Duration::from_millis(100))
            .open()
        {
            Ok(_) => {
                // Port exists and is accessible, close it for now
            }
            Err(e) => {
                eprintln!("❌ Failed to open serial port {}: {}", cli.port, e);
                eprintln!("\n📋 Available ports:");
                if let Ok(ports) = serialport::available_ports() {
                    if ports.is_empty() {
                        eprintln!("   (none found)");
                    } else {
                        for p in ports {
                            eprintln!("   • {}", p.port_name);
                        }
                    }
                }
                eprintln!("\n💡 Tip: Use --mock flag to run in mock mode without hardware");
                eprintln!("   Example: cargo run --release -- --mock");
                return Err(e.into());
            }
        }
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(cli.port.clone(), cli.mock, cli.csv.clone(), cli.rrd.clone(), cli.rerun_live, cli.max_samples)?;

    if cli.csv.is_some() {
        app.add_message(format!("CSV export enabled: {}", cli.csv.as_ref().unwrap()));
    }

    if cli.rrd.is_some() {
        app.add_message(format!("RRD export enabled: {}", cli.rrd.as_ref().unwrap()));
    }

    if cli.rerun_live {
        app.add_message("Rerun live streaming enabled".to_string());
    }

    // Open serial port if not in mock mode (we already validated it exists)
    let mut port: Option<Box<dyn SerialPort>> = if !cli.mock {
        let p = serialport::new(&cli.port, cli.baud)
            .timeout(Duration::from_millis(100))
            .open()?;
        app.add_message(format!("✓ Connected to {}", cli.port));
        Some(p)
    } else {
        app.add_message("🎭 Running in MOCK mode - simulated data".to_string());
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
        // Update motion display state (checks if 1 second has elapsed)
        app.motion_detector.update_display_state();

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
