use ratatui::{DefaultTerminal, Frame};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, BarChart, Gauge};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use crossterm::event::{self, Event, KeyCode};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AccessPoint {
    ssid: String,
    rssi: i8,
    ch: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Esp32Data {
    counter: u32,
    motion: u8,
    aps: Vec<AccessPoint>,
}

struct AppState {
    latest_data: Option<Esp32Data>,
    messages: Vec<String>,
    port_name: String,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    // List available serial ports
    let ports = serialport::available_ports()?;

    if ports.is_empty() {
        eprintln!("No serial ports found!");
        return Ok(());
    }

    println!("Available serial ports:");
    for (i, p) in ports.iter().enumerate() {
        println!("  [{}] {}", i, p.port_name);
    }

    // Use first port or specified port
    let port_name = std::env::args()
        .nth(1)
        .unwrap_or_else(|| ports[0].port_name.clone());

    println!("Using port: {}", port_name);

    let state = Arc::new(Mutex::new(AppState {
        latest_data: None,
        messages: Vec::new(),
        port_name: port_name.clone(),
    }));

    // Spawn serial reader thread
    let state_clone = Arc::clone(&state);
    thread::spawn(move || {
        if let Err(e) = read_serial_port(&port_name, state_clone) {
            eprintln!("Serial port error: {}", e);
        }
    });

    // Give serial thread time to start
    thread::sleep(Duration::from_millis(500));

    ratatui::run(|terminal| app(terminal, &state))?;
    Ok(())
}

fn read_serial_port(port_name: &str, state: Arc<Mutex<AppState>>) -> color_eyre::Result<()> {
    let mut port = serialport::new(port_name, 115_200)
        .timeout(Duration::from_millis(100))
        .open()?;

    let mut buffer = String::new();
    let mut read_buf = [0u8; 256];

    loop {
        match port.read(&mut read_buf) {
            Ok(n) if n > 0 => {
                let chunk = String::from_utf8_lossy(&read_buf[..n]);
                buffer.push_str(&chunk);

                // Process complete lines
                while let Some(line_end) = buffer.find('\n') {
                    let line = buffer[..line_end].trim().to_string();
                    buffer.drain(..=line_end);

                    if line.is_empty() {
                        continue;
                    }

                    let mut state = state.lock().unwrap();

                    // Try to parse as JSON
                    if let Ok(data) = serde_json::from_str::<Esp32Data>(&line) {
                        let motion_status = if data.motion > 0 { "MOTION!" } else { "still" };
                        state.messages.push(format!("Counter: {} | {} | {} APs",
                            data.counter, motion_status, data.aps.len()));
                        state.latest_data = Some(data.clone());
                    } else {
                        state.messages.push(format!("Raw: {}", line));
                    }

                    // Keep only last 100 messages
                    if state.messages.len() > 100 {
                        state.messages.remove(0);
                    }
                }
            }
            _ => {
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

fn app(terminal: &mut DefaultTerminal, state: &Arc<Mutex<AppState>>) -> std::io::Result<()> {
    loop {
        terminal.draw(|frame| render(frame, state))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                    break Ok(());
                }
            }
        }
    }
}

fn render(frame: &mut Frame, state: &Arc<Mutex<AppState>>) {
    let state = state.lock().unwrap();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),    // Header
            Constraint::Length(3),    // Motion indicator
            Constraint::Min(10),      // RSSI bars
            Constraint::Length(10),   // Messages
        ])
        .split(frame.area());

    // Header
    let motion_status = state.latest_data.as_ref()
        .map(|d| if d.motion > 0 { "⚠ MOTION DETECTED" } else { "• Still" })
        .unwrap_or("• Waiting...");

    let header = Paragraph::new(format!(
        "ESP32-C3 WiFi Motion Sensor | Port: {} | {} | Press 'q' to quit",
        state.port_name, motion_status
    ))
    .block(Block::default().borders(Borders::ALL).title("Status"))
    .style(Style::default().fg(Color::Cyan));
    frame.render_widget(header, chunks[0]);

    // Motion indicator gauge
    if let Some(ref data) = state.latest_data {
        let motion_color = if data.motion > 0 { Color::Red } else { Color::Green };
        let motion_label = if data.motion > 0 { "MOTION!" } else { "Still" };

        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Motion Status"))
            .gauge_style(Style::default().fg(motion_color).add_modifier(Modifier::BOLD))
            .ratio(if data.motion > 0 { 1.0 } else { 0.0 })
            .label(motion_label);
        frame.render_widget(gauge, chunks[1]);
    } else {
        let waiting = Paragraph::new("Waiting for data...")
            .block(Block::default().borders(Borders::ALL).title("Motion Status"));
        frame.render_widget(waiting, chunks[1]);
    }

    // RSSI visualization with bars
    if let Some(ref data) = state.latest_data {
        let mut ap_info: Vec<Line> = Vec::new();

        for ap in data.aps.iter().take(10) {
            // Color based on signal strength
            let signal_color = match ap.rssi {
                -50..=0 => Color::Green,
                -70..=-51 => Color::Yellow,
                _ => Color::Red,
            };

            // Truncate SSID if too long
            let ssid_display = if ap.ssid.len() > 12 {
                format!("{}...", &ap.ssid[..9])
            } else {
                ap.ssid.clone()
            };

            ap_info.push(Line::from(vec![
                Span::raw(format!("{:12} ", ssid_display)),
                Span::styled(
                    format!("{:3} dBm ", ap.rssi),
                    Style::default().fg(signal_color)
                ),
                Span::raw(format!("Ch:{}", ap.ch)),
            ]));
        }

        // Create AP info panel
        let ap_list = Paragraph::new(ap_info)
            .block(Block::default().borders(Borders::ALL).title(format!(
                "WiFi Access Points ({} detected)",
                data.aps.len()
            )))
            .style(Style::default().fg(Color::White));
        frame.render_widget(ap_list, chunks[2]);
    } else {
        let waiting = Paragraph::new("Waiting for WiFi scan data...")
            .block(Block::default().borders(Borders::ALL).title("WiFi Access Points"));
        frame.render_widget(waiting, chunks[2]);
    }

    // Message log
    let messages: Vec<ListItem> = state
        .messages
        .iter()
        .rev()
        .take(8)
        .map(|m| ListItem::new(m.as_str()))
        .collect();

    let messages_widget = List::new(messages)
        .block(Block::default().borders(Borders::ALL).title("Event Log"))
        .style(Style::default().fg(Color::Gray));
    frame.render_widget(messages_widget, chunks[3]);
}
