# Team Plan: ESP32-C3 WiFi Motion Sensor with Rerun Integration

## Current Project State

You already have:
- **ESP32 firmware** (`esp32c3-rust/esp-hacathon/src/bin/main.rs`) - WiFi RSSI scanning & motion detection
- **Host TUI** (`host-tuis/hello_test/src/main.rs`) - Serial reader with ratatui visualization
- **Data protocol** - JSON over serial (115200 baud)
- **Working features** - Motion detection, auto-port selection, live visualization

---

## 1. Unified Data Model (Foundation - Everyone Aligns First)

### Current Data Structures (already working):
```rust
// Already in host-tuis/hello_test/src/main.rs
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
```

### Enhanced Data Model (add to shared module):
```rust
// Create: host-tuis/hello_test/src/model.rs

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Enhanced frame with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiFrame {
    pub timestamp: SystemTime,
    pub counter: u32,
    pub motion: u8,
    pub aps: Vec<AccessPoint>,
}

/// Track RSSI changes over time for motion detection
#[derive(Debug, Clone)]
pub struct RssiHistory {
    pub ssid: String,
    pub channel: u8,
    pub rssi_samples: Vec<(SystemTime, i8)>,  // (time, rssi)
    pub baseline: i8,
}

/// App-wide events for async communication
pub enum AppEvent {
    WifiFrame(WifiFrame),
    DeviceStatus(DeviceStatus),
    RecordingStatus(RecordingStatus),
    CameraFrame(CameraFrame),  // bonus
}

#[derive(Debug, Clone)]
pub enum DeviceStatus {
    Connected(String),  // port name
    Scanning,
    Disconnected,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct RecordingStatus {
    pub is_recording: bool,
    pub file_path: Option<String>,
    pub frames_recorded: u64,
}

#[derive(Debug, Clone)]
pub struct CameraFrame {
    pub timestamp: SystemTime,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,  // RGB or JPEG bytes
}
```

### Event Bus:
```rust
// Use tokio channels for async event distribution
use tokio::sync::broadcast;

type EventSender = broadcast::Sender<AppEvent>;
type EventReceiver = broadcast::Receiver<AppEvent>;
```

---

## 2. Team Roles (4 People)

### **Role 1  ESP32 Firmware & Device Integration Lead**

**Current code location:** `esp32c3-rust/esp-hacathon/src/bin/main.rs`

**Key responsibilities:**

1. **Enhance ESP32 firmware:**
   - Add timestamp field to JSON output (use counter * scan_interval for now)
   - Add MAC address to AccessPoint data (if available via `esp-radio`)
   - Implement configurable scan interval via serial commands
   - Add device identification in JSON (device_id, firmware version)

2. **Create device abstraction layer in TUI:**
   - **New file:** `host-tuis/hello_test/src/device/mod.rs`
   - Wrap serial port communication:
     ```rust
     pub struct DeviceManager {
         port: SerialPort,
         config: DeviceConfig,
     }

     pub struct DeviceConfig {
         pub port_name: String,
         pub baud_rate: u32,
         pub scan_interval_ms: u32,
     }

     impl DeviceManager {
         pub async fn start_stream(&mut self, tx: EventSender) -> Result<()>;
         pub async fn stop_stream(&mut self) -> Result<()>;
         pub async fn configure(&mut self, config: DeviceConfig) -> Result<()>;
     }
     ```

3. **Mock data generator for testing:**
   - **New file:** `host-tuis/hello_test/src/device/mock.rs`
   - Generate synthetic WiFi frames:
     ```rust
     pub fn generate_mock_stream(tx: EventSender, interval_ms: u64) {
         // Simulate 3-5 APs with varying RSSI
         // Inject motion events periodically
     }
     ```

**Deliverables:**
- Enhanced ESP32 firmware with richer JSON output
- `src/device/mod.rs` - device manager
- `src/device/mock.rs` - mock stream
- Device configuration UI integration points

---

### **Role 2  TUI Shell & Application Orchestration Lead**

**Current code location:** `host-tuis/hello_test/src/main.rs`

**Key responsibilities:**

1. **Refactor current TUI into modular architecture:**
   - **Keep:** Current `AppState`, `render()` function
   - **Split into:**
     - `src/app.rs` - main application struct with tokio runtime
     - `src/ui/layout.rs` - screen layout functions
     - `src/ui/config_view.rs` - device configuration tab
     - `src/ui/live_view.rs` - current visualization (Role 3 enhances this)
     - `src/ui/status_view.rs` - streaming/recording status
     - `src/ui/help_view.rs` - keyboard shortcuts

2. **Implement tabbed navigation:**
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq)]
   pub enum Tab {
       Config,
       LiveView,
       Status,
       Help,
   }

   struct AppState {
       current_tab: Tab,
       device_status: DeviceStatus,
       latest_frame: Option<WifiFrame>,
       is_recording: bool,
       is_streaming_rerun: bool,
       camera_active: bool,
   }
   ```

3. **Build async event loop:**
   ```rust
   // Convert current blocking loop to tokio-based
   async fn app_loop(
       terminal: &mut Terminal<impl Backend>,
       mut event_rx: EventReceiver,
   ) -> Result<()> {
       loop {
           tokio::select! {
               Some(event) = event_rx.recv() => {
                   // Update state based on AppEvent
               }
               Ok(true) = poll_input(Duration::from_millis(100)) => {
                   // Handle keyboard input
               }
           }
           terminal.draw(|f| render(f, &state))?;
       }
   }
   ```

4. **Config tab features:**
   - Serial port selector (use current auto-detection)
   - Scan interval slider
   - RSSI threshold for motion detection
   - "Start/Stop Streaming" button
   - "Start/Stop Recording" button
   - "Enable Camera" toggle

**Deliverables:**
- Refactored modular TUI architecture
- Tab navigation system
- Async event loop with tokio
- Configuration UI
- Integration hooks for all other roles

---

### **Role 3  Visualization & Advanced Plotting Lead**

**Current code location:** `host-tuis/hello_test/src/main.rs:149-248` (render function)

**Key responsibilities:**

1. **Enhance current live view:**
   - **Keep:** Current AP list with color-coded RSSI
   - **Add:** Multiple visualization modes

2. **Implement visualization modes:**

   **Mode 1: Time-series RSSI plot** (line chart)
   ```rust
   // New file: src/ui/plots/rssi_timeseries.rs
   // X-axis: time (last 60 seconds)
   // Y-axis: RSSI
   // Multiple lines for different APs
   ```

   **Mode 2: RSSI heatmap**
   ```rust
   // New file: src/ui/plots/rssi_heatmap.rs
   // X-axis: time (last 5 minutes)
   // Y-axis: AP index
   // Color: RSSI strength (red = weak, green = strong)
   ```

   **Mode 3: Motion history**
   ```rust
   // New file: src/ui/plots/motion_history.rs
   // Timeline showing when motion was detected
   // Bar chart or color-coded timeline
   ```

   **Mode 4: Channel utilization**
   ```rust
   // New file: src/ui/plots/channel_view.rs
   // Show which WiFi channels are crowded
   // Bar chart by channel number
   ```

3. **Efficient history management:**
   ```rust
   // New file: src/visualization/history.rs
   use std::collections::VecDeque;

   pub struct VisualizationState {
       pub selected_mode: PlotMode,
       pub rssi_history: HashMap<String, VecDeque<(SystemTime, i8)>>,
       pub motion_history: VecDeque<(SystemTime, bool)>,
       pub max_history_seconds: u64,
   }

   impl VisualizationState {
       pub fn update(&mut self, frame: &WifiFrame) {
           // Add to history, trim old entries
       }

       pub fn get_plot_data(&self) -> PlotData {
           // Transform history into plot-ready format
       }
   }
   ```

4. **View controls:**
   - Keyboard shortcuts to switch modes (1-4)
   - Zoom in/out on timeseries
   - Select specific APs to highlight
   - Pause/resume updates

**Deliverables:**
- `src/ui/plots/` - all plot implementations
- `src/visualization/history.rs` - data management
- Enhanced live_view.rs with mode switching
- Smooth, performant real-time updates

---

### **Role 4  Streaming, Storage & Camera Lead**

**Key responsibilities:**

1. **Rerun.io integration:**
   - **New file:** `src/streaming/rerun.rs`
   - Add dependency: `rerun = "0.21"`

   ```rust
   pub struct RerunStreamer {
       rec: rerun::RecordingStream,
   }

   impl RerunStreamer {
       pub fn new(app_name: &str) -> Result<Self>;

       pub fn log_wifi_frame(&self, frame: &WifiFrame) {
           // Log RSSI as time series
           for ap in &frame.aps {
               self.rec.log(
                   format!("wifi/{}", ap.ssid),
                   &rerun::TimeSeriesScalar::new(ap.rssi as f64),
               )?;
           }

           // Log motion as indicator
           self.rec.log(
               "motion/detected",
               &rerun::TimeSeriesScalar::new(frame.motion as f64),
           )?;
       }

       pub fn start_recording(&mut self, path: &Path) -> Result<()>;
       pub fn stop_recording(&mut self) -> Result<()>;
   }
   ```

2. **CSV recording:**
   - **New file:** `src/storage/csv.rs`

   ```rust
   pub struct CsvRecorder {
       writer: csv::Writer<File>,
       frames_written: u64,
   }

   impl CsvRecorder {
       pub fn new(path: &Path) -> Result<Self>;

       pub fn write_frame(&mut self, frame: &WifiFrame) -> Result<()> {
           // One row per AP per frame
           for ap in &frame.aps {
               self.writer.serialize(CsvRow {
                   timestamp: frame.timestamp,
                   counter: frame.counter,
                   motion: frame.motion,
                   ssid: &ap.ssid,
                   rssi: ap.rssi,
                   channel: ap.ch,
               })?;
           }
       }
   }
   ```

3. **Camera integration (bonus):**
   - **New file:** `src/camera/mod.rs`
   - Use `nokhwa` crate (cross-platform camera access)

   ```rust
   pub struct CameraStreamer {
       camera: nokhwa::Camera,
   }

   impl CameraStreamer {
       pub fn start(&mut self, tx: EventSender) -> Result<()> {
           // Capture frames at 5 fps
           // Send as AppEvent::CameraFrame
       }

       pub fn log_to_rerun(&self, rerun: &RerunStreamer, frame: &CameraFrame) {
           // Log camera frame as Image to rerun
       }
   }
   ```

4. **Integration with TUI:**
   - Listen to AppEvent::WifiFrame
   - Toggle recording on/off via commands from Role 2
   - Display recording status (file path, frame count, file size)

**Deliverables:**
- `src/streaming/rerun.rs` - rerun integration
- `src/storage/csv.rs` - CSV recorder
- `src/camera/mod.rs` - camera streaming
- Recording controls in status view

---

## 3. Execution Timeline (2-3 Days)

### Phase 0: Alignment (2-3 hours)
**Everyone:**
- Review current codebase together
- Agree on data model (`WifiFrame`, `AppEvent`, etc.)
- Set up project structure:
  ```
  host-tuis/hello_test/src/
     main.rs           (entry point)
     model.rs          (shared data structures)
     app.rs            (main app logic - Role 2)
     device/
        mod.rs        (device manager - Role 1)
        mock.rs       (mock generator - Role 1)
     ui/
        layout.rs     (Role 2)
        config_view.rs (Role 2)
        live_view.rs  (Role 3)
        status_view.rs (Role 2)
        plots/        (Role 3)
     visualization/
        history.rs    (Role 3)
     streaming/
        rerun.rs      (Role 4)
     storage/
        csv.rs        (Role 4)
     camera/
         mod.rs        (Role 4)
  ```
- Add dependencies to `host-tuis/hello_test/Cargo.toml`:
  ```toml
  tokio = { version = "1", features = ["full"] }
  rerun = "0.21"
  csv = "1.3"
  nokhwa = "0.10"  # camera
  ```

### Phase 1: Skeleton & Mock (0.5 day)

**Role 1:**
- Create mock WiFi generator
- Test with current TUI

**Role 2:**
- Refactor current code into modules
- Implement tab navigation
- Get async event loop working with mock data

**Role 3:**
- Build first visualization mode (RSSI timeseries)
- Test with mock data

**Role 4:**
- Set up rerun SDK
- Create minimal CSV writer
- Test with mock data

**Milestone 1:** TUI with tabs, mock data flowing, basic rerun + CSV output

### Phase 2: Real Integration (1 day)

**Role 1:**
- Enhance ESP32 firmware (timestamps, etc.)
- Test real device connection
- Replace mock with real stream

**Role 2:**
- Build config tab (port selector, controls)
- Wire up start/stop controls
- Integrate device status display

**Role 3:**
- Implement all 4 visualization modes
- Add mode switching (keys 1-4)
- Optimize performance for real-time updates

**Role 4:**
- Complete rerun integration with real frames
- Implement .rrd recording toggle
- Finalize CSV format
- Start camera integration

**Milestone 2:** Full end-to-end with real ESP32, all visualizations, streaming to rerun, recording CSV

### Phase 3: Polish & Demo Prep (4-6 hours)

**All:**
- Error handling and recovery
- Performance optimization
- Help screen with keyboard shortcuts
- Status indicators (�connected, �recording, �streaming)

**Demo script:**
1. Show config tab � connect ESP32
2. Show live view � wave hand � motion detected
3. Switch visualization modes (1-4)
4. Start rerun streaming � show rerun viewer
5. Start recording � stop � show CSV file
6. (Bonus) Show camera overlay in rerun

---

## 4. Integration Points

### Data Flow:
```
ESP32 (UART/Serial)
    � JSON
DeviceManager (Role 1)
    � WifiFrame
EventBus (broadcast channel)
    � TUI (Role 2) � Visualization (Role 3)
    � Rerun Streamer (Role 4)
    � CSV Recorder (Role 4)
    � Camera (Role 4) � Rerun
```

### API Contracts:

**Role 1 provides:**
```rust
fn start_stream(config: DeviceConfig, tx: EventSender) -> Result<()>
fn stop_stream() -> Result<()>
```

**Role 2 provides:**
```rust
fn run_app(event_rx: EventReceiver) -> Result<()>
// Keyboard shortcuts call Role 1, 4 functions
```

**Role 3 provides:**
```rust
fn draw_live_view(frame: &mut Frame, area: Rect, state: &VisualizationState)
fn update_visualization(state: &mut VisualizationState, frame: &WifiFrame)
```

**Role 4 provides:**
```rust
fn start_rerun_stream() -> Result<RerunStreamer>
fn start_csv_recording(path: &Path) -> Result<CsvRecorder>
fn start_camera() -> Result<CameraStreamer>
```

---

## 5. Key Differences from Original Plan

1. **No esp-csi-cli-rs** - You're using direct serial communication to ESP32, which is simpler
2. **RSSI instead of CSI** - Motion detection based on signal strength, not full channel state
3. **Existing working prototype** - You already have baseline functionality to build upon
4. **Simpler firmware** - ESP32 code is straightforward; most complexity in host TUI
5. **WiFi scanning focus** - Not general CSI processing, but specific to AP detection

---

## 6. Collaboration Mechanics

To keep teamwork smooth:
- **Single repo** with modular structure
- **Branching by feature:**
  - `feature/device-cli`
  - `feature/tui-shell`
  - `feature/visualization`
  - `feature/rerun-csv`
- **Regular short syncs:**
  - Every 23 hours: 1015 minutes to:
    - Confirm API shapes (WifiFrame, events)
    - Resolve integration issues early
- **Code owner responsibility:**
  - Each role owns merging PRs in their area
  - One person acts as "integration lead" (probably Role 2) to ensure everything still compiles and runs

---

## 7. Success Criteria

**Must Have:**
-  Real-time TUI with multiple views
-  Motion detection visualization
-  Rerun.io streaming (live + .rrd recording)
-  CSV export
-  Clean, modular code architecture

**Nice to Have:**
- <� Camera integration with synced video
- =� Advanced analytics (motion patterns, heatmaps)
- <� Interactive controls (zoom, replay)
- =� Configuration persistence (save/load presets)

---

## 8. File Structure Reference

### Current Files:
```
esp-rust-hackathon/
   README.md
   Project_plan.md (this file)
   esp32c3-rust/esp-hacathon/
      Cargo.toml
      .cargo/config.toml
      flash-no-monitor.sh
      src/bin/main.rs         (ESP32 firmware)
   host-tuis/hello_test/
       Cargo.toml
       src/main.rs               (Current TUI)
```

### Target Structure:
```
esp-rust-hackathon/
   README.md
   Project_plan.md
   esp32c3-rust/esp-hacathon/
      src/bin/main.rs         (Enhanced ESP32 firmware)
   host-tuis/hello_test/
       src/
           main.rs              (Entry point)
           model.rs             (Shared data types)
           app.rs               (Main app logic)
           device/
              mod.rs           (Device manager)
              mock.rs          (Mock generator)
           ui/
              layout.rs        (Screen layouts)
              config_view.rs   (Config tab)
              live_view.rs     (Live visualization)
              status_view.rs   (Status tab)
              help_view.rs     (Help screen)
              plots/
                  rssi_timeseries.rs
                  rssi_heatmap.rs
                  motion_history.rs
                  channel_view.rs
           visualization/
              history.rs       (History management)
           streaming/
              rerun.rs         (Rerun integration)
           storage/
              csv.rs           (CSV recorder)
           camera/
               mod.rs           (Camera streamer)
```

---

This plan leverages your existing working code while scaling it into a complete, production-quality hackathon project. The architecture is clean, parallel work is possible, and you'll have impressive visualizations for demo day! =�
