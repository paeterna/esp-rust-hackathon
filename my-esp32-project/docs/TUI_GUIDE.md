# Building esp-csi-tui-rs: Step-by-Step Guide

## Project Structure

```
workspace/
├── esp-firmware/           # Your current ESP32-C3 project
│   └── (current project files)
│
└── esp-csi-tui-rs/        # New TUI application
    ├── Cargo.toml
    ├── src/
    │   ├── main.rs        # Entry point
    │   ├── serial.rs      # Serial communication
    │   ├── ui/
    │   │   ├── mod.rs
    │   │   ├── app.rs     # Application state
    │   │   ├── plots.rs   # CSI visualization
    │   │   └── config.rs  # Configuration UI
    │   ├── csi/
    │   │   ├── mod.rs
    │   │   ├── parser.rs  # Parse CSI data
    │   │   └── types.rs   # CSI data structures
    │   ├── storage/
    │   │   ├── mod.rs
    │   │   ├── csv.rs     # CSV export
    │   │   └── rrd.rs     # Rerun format
    │   └── streaming/
    │       └── rerun.rs   # Rerun.io integration
    └── README.md
```

## Step 1: Create the TUI Project

```bash
# Navigate to your workspace
cd /home/daniah

# Create new project
cargo new --bin esp-csi-tui-rs
cd esp-csi-tui-rs
```

## Step 2: Add Dependencies

Edit `Cargo.toml`:

```toml
[package]
name = "esp-csi-tui-rs"
version = "0.1.0"
edition = "2021"

[dependencies]
# Terminal UI
ratatui = "0.28"
crossterm = "0.28"

# Serial communication
serialport = "4.5"
tokio = { version = "1.40", features = ["full"] }
tokio-serial = "5.4"

# Data processing
num-complex = "0.4"
ndarray = "0.16"

# Data storage
csv = "1.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Rerun.io integration
rerun = "0.20"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Logging
log = "0.4"
env_logger = "0.11"

# CLI argument parsing
clap = { version = "4.5", features = ["derive"] }

[dev-dependencies]
criterion = "0.5"
```

## Step 3: Core Modules Implementation

### 3.1 Serial Communication Module

```rust
// src/serial.rs
use anyhow::{Context, Result};
use serialport::SerialPort;
use tokio::sync::mpsc;

pub struct SerialConnection {
    port: Box<dyn SerialPort>,
    rx_channel: mpsc::Receiver<Vec<u8>>,
    tx_channel: mpsc::Sender<Vec<u8>>,
}

impl SerialConnection {
    pub fn new(port_name: &str, baud_rate: u32) -> Result<Self> {
        let port = serialport::new(port_name, baud_rate)
            .timeout(Duration::from_millis(100))
            .open()
            .context("Failed to open serial port")?;
        
        // Setup channels for async communication
        let (tx, rx) = mpsc::channel(100);
        
        Ok(Self { port, rx_channel: rx, tx_channel: tx })
    }
    
    pub fn send_command(&mut self, cmd: Command) -> Result<()> {
        // Serialize and send command
        todo!()
    }
    
    pub fn read_response(&mut self) -> Result<Response> {
        // Read and parse response
        todo!()
    }
}
```

### 3.2 CSI Data Types

```rust
// src/csi/types.rs
use serde::{Deserialize, Serialize};
use num_complex::Complex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsiData {
    pub timestamp: u64,
    pub rssi: i8,
    pub rate: u8,
    pub channel: u8,
    pub mac_address: [u8; 6],
    pub subcarriers: Vec<Complex<f32>>,
}

impl CsiData {
    pub fn amplitude(&self) -> Vec<f32> {
        self.subcarriers.iter().map(|c| c.norm()).collect()
    }
    
    pub fn phase(&self) -> Vec<f32> {
        self.subcarriers.iter().map(|c| c.arg()).collect()
    }
    
    pub fn to_csv_row(&self) -> String {
        // Format for CSV export
        todo!()
    }
}
```

### 3.3 Ratatui UI

```rust
// src/ui/app.rs
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
    layout::{Layout, Constraint, Direction},
    widgets::{Block, Borders, Paragraph, Chart, Axis, Dataset},
};

pub struct App {
    pub running: bool,
    pub csi_data: Vec<CsiData>,
    pub current_view: ViewMode,
    pub config: AppConfig,
}

pub enum ViewMode {
    Amplitude2D,
    Phase2D,
    Heatmap,
    ColorDomain,
    Config,
}

impl App {
    pub fn new() -> Self {
        Self {
            running: true,
            csi_data: Vec::new(),
            current_view: ViewMode::Amplitude2D,
            config: AppConfig::default(),
        }
    }
    
    pub fn render(&mut self, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),    // Header
                    Constraint::Min(0),        // Main view
                    Constraint::Length(3),     // Status bar
                ])
                .split(frame.area());
            
            // Render header
            self.render_header(frame, chunks[0]);
            
            // Render main view based on mode
            match self.current_view {
                ViewMode::Amplitude2D => self.render_amplitude_plot(frame, chunks[1]),
                ViewMode::Phase2D => self.render_phase_plot(frame, chunks[1]),
                ViewMode::Heatmap => self.render_heatmap(frame, chunks[1]),
                ViewMode::ColorDomain => self.render_color_domain(frame, chunks[1]),
                ViewMode::Config => self.render_config(frame, chunks[1]),
            }
            
            // Render status bar
            self.render_status(frame, chunks[2]);
        })?;
        
        Ok(())
    }
}
```

### 3.4 Rerun.io Integration

```rust
// src/streaming/rerun.rs
use rerun::{RecordingStream, RecordingStreamBuilder};
use anyhow::Result;

pub struct RerunStreamer {
    rec: RecordingStream,
}

impl RerunStreamer {
    pub fn new(app_id: &str) -> Result<Self> {
        let rec = RecordingStreamBuilder::new(app_id)
            .connect()?;
        Ok(Self { rec })
    }
    
    pub fn stream_csi_data(&self, data: &CsiData) -> Result<()> {
        // Log CSI data to Rerun viewer
        // Use timeseries, tensors, or 3D visualizations
        
        self.rec.log(
            "csi/amplitude",
            &rerun::TimeSeriesScalar::new(data.timestamp as f64)
        )?;
        
        Ok(())
    }
}
```

## Step 4: Main Application Loop

```rust
// src/main.rs
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(name = "esp-csi-tui-rs")]
#[command(about = "ESP32 CSI Data Collection TUI", long_about = None)]
struct Cli {
    /// Serial port path
    #[arg(short, long, default_value = "/dev/ttyUSB0")]
    port: String,
    
    /// Baud rate
    #[arg(short, long, default_value = "115200")]
    baud: u32,
    
    /// Enable Rerun streaming
    #[arg(short, long)]
    rerun: bool,
    
    /// Output CSV file
    #[arg(short, long)]
    output: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    
    // Initialize terminal
    let mut terminal = setup_terminal()?;
    
    // Connect to ESP32
    let mut serial = SerialConnection::new(&cli.port, cli.baud)?;
    
    // Initialize app
    let mut app = App::new();
    
    // Optional: Start Rerun streaming
    let rerun_streamer = if cli.rerun {
        Some(RerunStreamer::new("esp-csi-tui")?)
    } else {
        None
    };
    
    // Main event loop
    while app.running {
        // Handle keyboard input
        if event::poll(Duration::from_millis(100))? {
            handle_events(&mut app)?;
        }
        
        // Read CSI data from serial
        if let Ok(data) = serial.read_csi_data() {
            app.csi_data.push(data.clone());
            
            // Stream to Rerun if enabled
            if let Some(ref streamer) = rerun_streamer {
                streamer.stream_csi_data(&data)?;
            }
            
            // Save to CSV if specified
            if let Some(ref output) = cli.output {
                save_to_csv(output, &data)?;
            }
        }
        
        // Render UI
        app.render(&mut terminal)?;
    }
    
    // Cleanup
    restore_terminal(&mut terminal)?;
    
    Ok(())
}
```

## Step 5: Key Features to Implement

### Priority 1: Must Have
- [ ] Serial communication with ESP32
- [ ] Basic TUI with Ratatui
- [ ] 2D amplitude plot
- [ ] CSV data export
- [ ] Device configuration UI

### Priority 2: Important
- [ ] Phase plotting
- [ ] Heatmap visualization
- [ ] Rerun.io streaming
- [ ] .rrd format export
- [ ] Multiple subcarrier selection

### Priority 3: Nice to Have
- [ ] 3D visualization
- [ ] Color domain plot
- [ ] Camera integration
- [ ] Real-time filtering
- [ ] Export presets

## Step 6: Testing Strategy

```bash
# Test serial communication
cargo test serial_connection

# Test CSI parsing
cargo test csi_parser

# Test data export
cargo test csv_export

# Run with mock data
cargo run -- --mock-data

# Run with real ESP32
cargo run -- --port /dev/ttyUSB0 --baud 115200
```

## Step 7: Development Workflow

1. **Start Simulator** (if no ESP32 yet):
   ```bash
   # Create mock data generator
   cargo run --bin mock-esp32
   ```

2. **Run TUI in Development**:
   ```bash
   RUST_LOG=debug cargo run -- --port /dev/ttyUSB0
   ```

3. **Test with Rerun**:
   ```bash
   # Terminal 1: Start Rerun viewer
   rerun
   
   # Terminal 2: Run TUI with streaming
   cargo run -- --port /dev/ttyUSB0 --rerun
   ```

## Resources

- **Ratatui Examples**: https://github.com/ratatui/ratatui/tree/main/examples
- **Serialport Rust**: https://docs.rs/serialport/
- **Rerun.io Rust API**: https://docs.rs/rerun/
- **CSI Visualization Papers**: Research for plot types

## Timeline Suggestion

- **Week 1**: Serial + Basic TUI + Simple plotting
- **Week 2**: All plot types + CSV export
- **Week 3**: Rerun integration + .rrd format
- **Week 4**: Polish + Camera bonus + Testing

Good luck! 🚀
