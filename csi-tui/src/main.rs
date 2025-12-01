mod app;
mod csi;
mod motion;
mod ui;

use app::App;
use color_eyre::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, Terminal};
use std::io::stdout;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Parse command line arguments
    let device_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| {
            // Try to auto-detect USB serial device
            if cfg!(target_os = "macos") {
                // On macOS, try to find tty.usbserial device (better for bidirectional communication)
                if let Ok(entries) = std::fs::read_dir("/dev") {
                    for entry in entries.flatten() {
                        let name = entry.file_name();
                        let name_str = name.to_string_lossy();
                        if name_str.starts_with("tty.usbserial") {
                            return format!("/dev/{}", name_str);
                        }
                    }
                }
                "/dev/tty.usbserial-110".to_string() // Fallback for macOS
            } else {
                "/dev/ttyUSB0".to_string() // Fallback for Linux
            }
        });

    // Check if we're using stdin mode (piped input)
    let using_stdin = device_path == "stdin" || device_path == "-";

    if !using_stdin {
        eprintln!("Using device: {}", device_path);
    }

    // Create CSI frame channel
    let (frame_tx, mut frame_rx) = mpsc::unbounded_channel();

    // Start CSI reader BEFORE setting up terminal (important for stdin mode)
    let reader = csi::CsiReader::new(device_path.clone(), frame_tx);
    tokio::spawn(async move {
        if let Err(e) = reader.start().await {
            eprintln!("CSI reader error: {}", e);
        }
    });

    // Give the reader task a moment to claim stdin before we set up the terminal
    if using_stdin {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    // Setup terminal (after starting reader, so stdin is claimed first in stdin mode)
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    // Create application
    let mut app = App::new(device_path.clone());

    app.connection_status = app::ConnectionStatus::Connected;
    app.stream_status = app::StreamStatus::Streaming;

    // Main event loop
    let result = run_app(&mut terminal, &mut app, &mut frame_rx, using_stdin).await;

    // Restore terminal
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    result
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    frame_rx: &mut mpsc::UnboundedReceiver<csi::CsiFrame>,
    using_stdin: bool,
) -> Result<()> {
    loop {
        // Render UI
        terminal.draw(|f| ui::render_main_layout(f, app))?;

        // Handle events (only when not using stdin, as stdin is being consumed by CSI reader)
        if !using_stdin {
            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        handle_key_event(app, key.code);
                    }
                }
            }
        } else {
            // In stdin mode, just sleep a bit to avoid busy loop
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        // Process CSI frames (non-blocking)
        while let Ok(frame) = frame_rx.try_recv() {
            app.process_csi_frame(frame);
        }

        if app.exit {
            break;
        }
    }

    Ok(())
}

fn handle_key_event(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => app.exit = true,
        KeyCode::Char('?') => app.toggle_help(),
        KeyCode::Char('c') => app.reset_motion_detector(),
        KeyCode::Char('r') => app.reset_motion_detector(),
        KeyCode::Up => app.increase_threshold(),
        KeyCode::Down => app.decrease_threshold(),
        KeyCode::Left => app.decrease_smoothing(),
        KeyCode::Right => app.increase_smoothing(),
        _ => {}
    }
}
