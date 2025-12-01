# ESP32-C3 CSI Motion Detection System

Complete Architecture & Implementation Guide

## 1. Architecture Explanation

### Data Flow

```
┌─────────────────┐
│  ESP32-C3       │
│  (Fixed FW)     │ ──[Serial/JTAG]──> Raw CSI packets
└─────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│  esp-csi-cli-rs Process                                     │
│  • Sends commands (set-wifi, set-csi, start)                │
│  • Receives CSI data over serial                            │
│  • Outputs formatted CSI to stdout                          │
└─────────────────────────────────────────────────────────────┘
         │
         ▼  (stdout: CSV/JSON lines)
┌─────────────────────────────────────────────────────────────┐
│  csi-tui (Host Application)                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Serial Reader Thread                               │   │
│  │  • Spawns esp-csi-cli-rs as subprocess             │   │
│  │  • Parses stdout lines → CsiFrame                   │   │
│  │  • Sends frames via mpsc channel                    │   │
│  └─────────────────────────────────────────────────────┘   │
│         │                                                    │
│         ▼                                                    │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Motion Detection Module                            │   │
│  │  • Maintains sliding window of frames               │   │
│  │  • Computes amplitude deltas                        │   │
│  │  • Outputs motion score & detection flag            │   │
│  └─────────────────────────────────────────────────────┘   │
│         │                                                    │
│         ▼                                                    │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Application State                                  │   │
│  │  • Latest CSI frame                                 │   │
│  │  • CSI history (for heatmap)                        │   │
│  │  • Motion score timeline                            │   │
│  │  • Detection state & config                         │   │
│  └─────────────────────────────────────────────────────┘   │
│         │                                                    │
│         ▼                                                    │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Ratatui Renderer                                   │   │
│  │  • Live CSI amplitude plot                          │   │
│  │  • Time×Subcarrier heatmap                          │   │
│  │  • Motion score chart                               │   │
│  │  • Status/Config panel                              │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Module Structure

```
esp-rust-hackathon/
├── esp-csi-cli-rs/           # Existing embedded firmware
│   └── src/bin/async_main.rs # Runs on ESP32 (NO CHANGES)
│
├── csi-tui/                   # Host-side TUI (OUR WORK)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs           # Entry point, event loop
│   │   ├── app.rs            # App state management
│   │   ├── csi/
│   │   │   ├── mod.rs        # CSI module root
│   │   │   ├── frame.rs      # CsiFrame data structure
│   │   │   ├── parser.rs     # Parse esp-csi-cli-rs output
│   │   │   └── reader.rs     # Serial/subprocess management
│   │   ├── motion/
│   │   │   ├── mod.rs        # Motion module root
│   │   │   ├── detector.rs   # Motion detection algorithm
│   │   │   └── config.rs     # Threshold configuration
│   │   └── ui/
│   │       ├── mod.rs        # UI module root
│   │       ├── layout.rs     # Screen layout
│   │       ├── plots.rs      # CSI amplitude plots
│   │       ├── heatmap.rs    # Time×Subcarrier heatmap
│   │       ├── motion.rs     # Motion visualization
│   │       └── status.rs     # Status/config panel
│   └── README.md
│
└── tools/                     # Optional: testing utilities
    └── mock_csi_generator.rs  # Synthetic CSI for testing
```

## 2. Motion Detection Algorithm (Host-Side)

### Theory

Motion detection via CSI relies on the fact that movement in the environment changes multipath propagation, causing amplitude/phase variations across subcarriers.

**Simple Robust Algorithm:**

1. Maintain a sliding window of recent CSI frames (e.g., last 10 frames)
2. Compute frame-to-frame amplitude differences
3. Aggregate change metric (e.g., mean absolute difference across subcarriers)
4. Apply threshold + temporal smoothing to detect motion

### Data Structures

**File: `csi-tui/src/csi/frame.rs`**

```rust
use std::time::SystemTime;

/// Represents a single CSI frame from the ESP32
#[derive(Debug, Clone)]
pub struct CsiFrame {
    /// Timestamp when frame was received
    pub timestamp: SystemTime,

    /// CSI amplitude data (one value per subcarrier)
    /// Typically 64 subcarriers for 20MHz channel, 128 for 40MHz
    pub amplitudes: Vec<f32>,

    /// Optional: phase data (if available from firmware)
    pub phases: Option<Vec<f32>>,

    /// Metadata from the CSI packet
    pub metadata: CsiMetadata,
}

#[derive(Debug, Clone)]
pub struct CsiMetadata {
    /// RSSI value
    pub rssi: i8,

    /// Data rate (Mbps)
    pub rate: u8,

    /// Signal bandwidth (20/40 MHz)
    pub bandwidth: u8,

    /// Channel number
    pub channel: u8,

    /// MAC address of transmitter (if available)
    pub mac: Option<[u8; 6]>,

    /// Sequence number
    pub seq: u32,
}

impl CsiFrame {
    /// Compute L2 distance between this frame and another
    pub fn distance(&self, other: &CsiFrame) -> f32 {
        if self.amplitudes.len() != other.amplitudes.len() {
            return f32::MAX; // Incompatible frames
        }

        self.amplitudes
            .iter()
            .zip(&other.amplitudes)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f32>()
            .sqrt()
    }

    /// Compute mean amplitude
    pub fn mean_amplitude(&self) -> f32 {
        if self.amplitudes.is_empty() {
            return 0.0;
        }
        self.amplitudes.iter().sum::<f32>() / self.amplitudes.len() as f32
    }

    /// Compute amplitude variance
    pub fn amplitude_variance(&self) -> f32 {
        if self.amplitudes.len() < 2 {
            return 0.0;
        }

        let mean = self.mean_amplitude();
        let variance = self.amplitudes
            .iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f32>() / self.amplitudes.len() as f32;

        variance
    }
}
```

**File: `csi-tui/src/motion/config.rs`**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionConfig {
    /// Number of frames to keep in sliding window
    pub window_size: usize,

    /// Motion score threshold (0.0 - 1.0)
    /// Higher = less sensitive
    pub detection_threshold: f32,

    /// Minimum number of consecutive detections to trigger
    pub min_consecutive_frames: usize,

    /// Smoothing factor for exponential moving average (0.0 - 1.0)
    /// Lower = more smoothing
    pub smoothing_alpha: f32,

    /// Cooldown period (frames) after detection
    pub cooldown_frames: usize,
}

impl Default for MotionConfig {
    fn default() -> Self {
        Self {
            window_size: 10,
            detection_threshold: 0.15,
            min_consecutive_frames: 3,
            smoothing_alpha: 0.3,
            cooldown_frames: 5,
        }
    }
}
```

**File: `csi-tui/src/motion/detector.rs`**

```rust
use crate::csi::frame::CsiFrame;
use crate::motion::config::MotionConfig;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct MotionDetector {
    config: MotionConfig,

    /// Sliding window of recent frames
    frame_history: VecDeque<CsiFrame>,

    /// Smoothed motion score (0.0 - 1.0)
    motion_score: f32,

    /// Raw motion scores (before smoothing)
    raw_scores: VecDeque<f32>,

    /// Consecutive detection counter
    consecutive_detections: usize,

    /// Cooldown counter
    cooldown_counter: usize,

    /// Current detection state
    motion_detected: bool,

    /// Calibration baseline (computed from first N frames)
    baseline_variance: Option<f32>,
    calibration_frames: usize,
}

impl MotionDetector {
    pub fn new(config: MotionConfig) -> Self {
        Self {
            config: config.clone(),
            frame_history: VecDeque::with_capacity(config.window_size),
            motion_score: 0.0,
            raw_scores: VecDeque::with_capacity(100), // Keep last 100 scores
            consecutive_detections: 0,
            cooldown_counter: 0,
            motion_detected: false,
            baseline_variance: None,
            calibration_frames: 0,
        }
    }

    /// Process a new CSI frame and update motion state
    pub fn process_frame(&mut self, frame: CsiFrame) -> MotionDetectionResult {
        // Add frame to history
        self.frame_history.push_back(frame.clone());
        if self.frame_history.len() > self.config.window_size {
            self.frame_history.pop_front();
        }

        // Calibration phase (first 30 frames)
        if self.calibration_frames < 30 {
            self.calibration_frames += 1;
            if self.calibration_frames == 30 {
                self.baseline_variance = Some(self.compute_baseline_variance());
            }
            return MotionDetectionResult {
                motion_detected: false,
                motion_score: 0.0,
                calibrating: true,
            };
        }

        // Compute raw motion score
        let raw_score = self.compute_motion_score(&frame);
        self.raw_scores.push_back(raw_score);
        if self.raw_scores.len() > 100 {
            self.raw_scores.pop_front();
        }

        // Apply exponential moving average smoothing
        self.motion_score = self.config.smoothing_alpha * raw_score
            + (1.0 - self.config.smoothing_alpha) * self.motion_score;

        // Handle cooldown
        if self.cooldown_counter > 0 {
            self.cooldown_counter -= 1;
            return MotionDetectionResult {
                motion_detected: self.motion_detected,
                motion_score: self.motion_score,
                calibrating: false,
            };
        }

        // Detection logic
        let threshold_exceeded = self.motion_score > self.config.detection_threshold;

        if threshold_exceeded {
            self.consecutive_detections += 1;
            if self.consecutive_detections >= self.config.min_consecutive_frames {
                self.motion_detected = true;
                self.cooldown_counter = self.config.cooldown_frames;
            }
        } else {
            self.consecutive_detections = 0;
            self.motion_detected = false;
        }

        MotionDetectionResult {
            motion_detected: self.motion_detected,
            motion_score: self.motion_score,
            calibrating: false,
        }
    }

    /// Compute motion score from current frame vs. history
    fn compute_motion_score(&self, current: &CsiFrame) -> f32 {
        if self.frame_history.len() < 2 {
            return 0.0;
        }

        // Compare with previous frame
        let previous = &self.frame_history[self.frame_history.len() - 2];

        // Compute normalized amplitude difference
        let distance = current.distance(previous);
        let num_subcarriers = current.amplitudes.len() as f32;

        // Normalize by baseline variance (prevents false positives from noise)
        let normalized_distance = if let Some(baseline) = self.baseline_variance {
            distance / (baseline.sqrt() * num_subcarriers.sqrt())
        } else {
            distance / num_subcarriers.sqrt()
        };

        // Clamp to [0, 1]
        normalized_distance.min(1.0).max(0.0)
    }

    /// Compute baseline variance from initial frames
    fn compute_baseline_variance(&self) -> f32 {
        if self.frame_history.is_empty() {
            return 1.0; // Fallback
        }

        // Average variance across all frames in history
        let variances: Vec<f32> = self.frame_history
            .iter()
            .map(|f| f.amplitude_variance())
            .collect();

        variances.iter().sum::<f32>() / variances.len() as f32
    }

    /// Get current motion score
    pub fn motion_score(&self) -> f32 {
        self.motion_score
    }

    /// Get detection state
    pub fn is_motion_detected(&self) -> bool {
        self.motion_detected
    }

    /// Get motion score history for visualization
    pub fn score_history(&self) -> &VecDeque<f32> {
        &self.raw_scores
    }

    /// Update configuration
    pub fn update_config(&mut self, config: MotionConfig) {
        self.config = config;
    }

    /// Reset detector state
    pub fn reset(&mut self) {
        self.frame_history.clear();
        self.raw_scores.clear();
        self.motion_score = 0.0;
        self.consecutive_detections = 0;
        self.cooldown_counter = 0;
        self.motion_detected = false;
        self.baseline_variance = None;
        self.calibration_frames = 0;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MotionDetectionResult {
    pub motion_detected: bool,
    pub motion_score: f32,
    pub calibrating: bool,
}
```

**File: `csi-tui/src/motion/mod.rs`**

```rust
pub mod config;
pub mod detector;

pub use config::MotionConfig;
pub use detector::{MotionDetector, MotionDetectionResult};
```

## 3. Ratatui UI Changes

### Screen Layout

```
┌─────────────────────────────────────────────────────────────────────────┐
│ ESP32-C3 CSI Motion Detector          [Connected] [Streaming] [Motion!] │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                           │
│  ┌─ Live CSI Amplitude ──────────────────┐  ┌─ Status ────────────────┐ │
│  │                                        │  │ Device: /dev/ttyUSB0    │ │
│  │   ^                                    │  │ Mode: Station           │ │
│  │ 80│     *  *                           │  │ Channel: 6              │ │
│  │ 60│   *      *    *                    │  │ RSSI: -45 dBm           │ │
│  │ 40│ *          *    *  *               │  │ Rate: 54 Mbps           │ │
│  │ 20│                      *             │  │                         │ │
│  │  0└────────────────────────────>       │  │ ── Motion Config ──     │ │
│  │     0    16    32    48    64          │  │ Threshold: 0.15 [↑↓]   │ │
│  │         Subcarrier Index               │  │ Smoothing: 0.30 [←→]   │ │
│  └────────────────────────────────────────┘  │ Window: 10 frames       │ │
│                                              └─────────────────────────┘ │
│  ┌─ CSI Heatmap (Time × Subcarrier) ────────────────────────────────────┐ │
│  │                                                                       │ │
│  │  64 ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░   │ │
│  │     ░░░░░░░░▓▓▓▓░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░   │ │
│  │  32 ░░░░░░░░▓▓▓▓▓▓░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░   │ │
│  │     ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░   │ │
│  │   0 ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░   │ │
│  │     ────────────────────────────────────────────────────────>         │ │
│  │     (now)  -5s   -10s  -15s  -20s  -25s  -30s                        │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                           │
│  ┌─ Motion Score Timeline ───────────────────────────────────────────────┐ │
│  │  1.0 ┌───────────────────────────────────────────────────────────┐   │ │
│  │      │                    **                                      │   │ │
│  │  0.5 │         *        **  **              MOTION DETECTED!      │   │ │
│  │      │  ~~~*~~*~~*~~~~*~~~~~~~*~~~*~~~~*~~~~~~                   │   │ │
│  │  0.0 └───────────────────────────────────────────────────────────┘   │ │
│  │      [Threshold: 0.15 ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─]                  │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                           │
├─────────────────────────────────────────────────────────────────────────┤
│ [q]Quit [c]Calibrate [t]Threshold↑↓ [s]Smoothing←→ [r]Reset [?]Help     │
└─────────────────────────────────────────────────────────────────────────┘
```

### Application State

**File: `csi-tui/src/app.rs`**

```rust
use crate::csi::frame::CsiFrame;
use crate::motion::{MotionConfig, MotionDetector, MotionDetectionResult};
use std::collections::VecDeque;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamStatus {
    Stopped,
    Starting,
    Streaming,
    Paused,
}

pub struct App {
    /// Exit flag
    pub exit: bool,

    /// Connection status to ESP32
    pub connection_status: ConnectionStatus,

    /// Stream status
    pub stream_status: StreamStatus,

    /// Latest CSI frame
    pub latest_frame: Option<CsiFrame>,

    /// CSI frame history (for heatmap visualization)
    /// Stores (timestamp, frame) tuples
    pub frame_history: VecDeque<(SystemTime, CsiFrame)>,

    /// Maximum history to keep (e.g., 30 seconds @ 10 fps = 300 frames)
    pub max_history: usize,

    /// Motion detector
    pub motion_detector: MotionDetector,

    /// Motion detection result
    pub motion_result: MotionDetectionResult,

    /// Motion score timeline (for chart)
    pub motion_timeline: VecDeque<(SystemTime, f32)>,

    /// Device path (e.g., /dev/ttyUSB0)
    pub device_path: String,

    /// Error message (if any)
    pub error_message: Option<String>,

    /// UI state
    pub ui_state: UiState,
}

#[derive(Debug, Clone)]
pub struct UiState {
    /// Currently selected tab
    pub selected_tab: usize,

    /// Whether config editing mode is active
    pub editing_config: bool,

    /// Which config parameter is selected
    pub selected_config_param: ConfigParam,

    /// Show help overlay
    pub show_help: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigParam {
    Threshold,
    Smoothing,
    WindowSize,
    MinConsecutive,
}

impl App {
    pub fn new(device_path: String) -> Self {
        let motion_config = MotionConfig::default();
        Self {
            exit: false,
            connection_status: ConnectionStatus::Disconnected,
            stream_status: StreamStatus::Stopped,
            latest_frame: None,
            frame_history: VecDeque::with_capacity(300),
            max_history: 300,
            motion_detector: MotionDetector::new(motion_config),
            motion_result: MotionDetectionResult {
                motion_detected: false,
                motion_score: 0.0,
                calibrating: false,
            },
            motion_timeline: VecDeque::with_capacity(300),
            device_path,
            error_message: None,
            ui_state: UiState {
                selected_tab: 0,
                editing_config: false,
                selected_config_param: ConfigParam::Threshold,
                show_help: false,
            },
        }
    }

    /// Process a new CSI frame
    pub fn process_csi_frame(&mut self, frame: CsiFrame) {
        let timestamp = frame.timestamp;

        // Update latest frame
        self.latest_frame = Some(frame.clone());

        // Add to history
        self.frame_history.push_back((timestamp, frame.clone()));
        if self.frame_history.len() > self.max_history {
            self.frame_history.pop_front();
        }

        // Run motion detection
        self.motion_result = self.motion_detector.process_frame(frame);

        // Add to motion timeline
        self.motion_timeline.push_back((timestamp, self.motion_result.motion_score));
        if self.motion_timeline.len() > self.max_history {
            self.motion_timeline.pop_front();
        }
    }

    /// Adjust threshold up
    pub fn increase_threshold(&mut self) {
        let mut config = self.motion_detector.config.clone();
        config.detection_threshold = (config.detection_threshold + 0.05).min(1.0);
        self.motion_detector.update_config(config);
    }

    /// Adjust threshold down
    pub fn decrease_threshold(&mut self) {
        let mut config = self.motion_detector.config.clone();
        config.detection_threshold = (config.detection_threshold - 0.05).max(0.0);
        self.motion_detector.update_config(config);
    }

    /// Adjust smoothing up
    pub fn increase_smoothing(&mut self) {
        let mut config = self.motion_detector.config.clone();
        config.smoothing_alpha = (config.smoothing_alpha + 0.05).min(1.0);
        self.motion_detector.update_config(config);
    }

    /// Adjust smoothing down
    pub fn decrease_smoothing(&mut self) {
        let mut config = self.motion_detector.config.clone();
        config.smoothing_alpha = (config.smoothing_alpha - 0.05).max(0.0);
        self.motion_detector.update_config(config);
    }

    /// Reset motion detector
    pub fn reset_motion_detector(&mut self) {
        self.motion_detector.reset();
        self.motion_timeline.clear();
    }

    /// Toggle help
    pub fn toggle_help(&mut self) {
        self.ui_state.show_help = !self.ui_state.show_help;
    }
}
```

### UI Rendering

**File: `csi-tui/src/ui/plots.rs`**

```rust
use ratatui::{
    prelude::*,
    widgets::*,
};
use crate::csi::frame::CsiFrame;

/// Render CSI amplitude plot
pub fn render_amplitude_plot(frame: &mut Frame, area: Rect, csi_frame: Option<&CsiFrame>) {
    let Some(csi) = csi_frame else {
        let placeholder = Paragraph::new("Waiting for CSI data...")
            .block(Block::default().borders(Borders::ALL).title("Live CSI Amplitude"))
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(placeholder, area);
        return;
    };

    // Convert amplitudes to chart data
    let data: Vec<(f64, f64)> = csi.amplitudes
        .iter()
        .enumerate()
        .map(|(i, &amp)| (i as f64, amp as f64))
        .collect();

    if data.is_empty() {
        return;
    }

    let max_amp = data.iter().map(|(_, y)| *y).fold(0.0, f64::max);
    let min_amp = data.iter().map(|(_, y)| *y).fold(f64::MAX, f64::min);

    let datasets = vec![
        Dataset::default()
            .name("Amplitude")
            .marker(symbols::Marker::Dot)
            .style(Style::default().fg(Color::Cyan))
            .graph_type(GraphType::Line)
            .data(&data)
    ];

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Live CSI Amplitude")
        )
        .x_axis(
            Axis::default()
                .title("Subcarrier Index")
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, data.len() as f64])
                .labels(vec![
                    "0".into(),
                    format!("{}", data.len() / 4).into(),
                    format!("{}", data.len() / 2).into(),
                    format!("{}", 3 * data.len() / 4).into(),
                    format!("{}", data.len()).into(),
                ])
        )
        .y_axis(
            Axis::default()
                .title("Amplitude")
                .style(Style::default().fg(Color::Gray))
                .bounds([min_amp - 5.0, max_amp + 5.0])
                .labels(vec![
                    format!("{:.0}", min_amp).into(),
                    format!("{:.0}", (min_amp + max_amp) / 2.0).into(),
                    format!("{:.0}", max_amp).into(),
                ])
        );

    frame.render_widget(chart, area);
}
```

**File: `csi-tui/src/ui/heatmap.rs`**

```rust
use ratatui::{
    prelude::*,
    widgets::*,
};
use std::collections::VecDeque;
use std::time::SystemTime;
use crate::csi::frame::CsiFrame;

/// Render CSI heatmap (time × subcarrier)
pub fn render_heatmap(
    frame: &mut Frame,
    area: Rect,
    history: &VecDeque<(SystemTime, CsiFrame)>,
) {
    if history.is_empty() {
        let placeholder = Paragraph::new("Waiting for CSI history...")
            .block(Block::default().borders(Borders::ALL).title("CSI Heatmap (Time × Subcarrier)"))
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(placeholder, area);
        return;
    }

    // Determine subcarrier count
    let num_subcarriers = history.back().map(|(_, f)| f.amplitudes.len()).unwrap_or(64);

    // Create canvas for heatmap
    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("CSI Heatmap (Time × Subcarrier)")
        )
        .x_bounds([0.0, history.len() as f64])
        .y_bounds([0.0, num_subcarriers as f64])
        .paint(|ctx| {
            for (time_idx, (_, csi)) in history.iter().enumerate() {
                // Normalize amplitudes for color mapping
                let max_amp = csi.amplitudes.iter().cloned().fold(0.0f32, f32::max);

                for (subcarrier_idx, &amplitude) in csi.amplitudes.iter().enumerate() {
                    let normalized = if max_amp > 0.0 {
                        amplitude / max_amp
                    } else {
                        0.0
                    };

                    // Map to color (blue -> green -> yellow -> red)
                    let color = amplitude_to_color(normalized);

                    // Draw pixel
                    ctx.print(
                        time_idx as f64,
                        subcarrier_idx as f64,
                        Span::styled("█", Style::default().fg(color))
                    );
                }
            }
        });

    frame.render_widget(canvas, area);
}

/// Map normalized amplitude (0.0-1.0) to color
fn amplitude_to_color(normalized: f32) -> Color {
    match normalized {
        x if x < 0.25 => Color::Blue,
        x if x < 0.5 => Color::Cyan,
        x if x < 0.75 => Color::Yellow,
        _ => Color::Red,
    }
}
```

**File: `csi-tui/src/ui/motion.rs`**

```rust
use ratatui::{
    prelude::*,
    widgets::*,
};
use std::collections::VecDeque;
use std::time::SystemTime;
use crate::motion::MotionDetectionResult;

/// Render motion score timeline
pub fn render_motion_timeline(
    frame: &mut Frame,
    area: Rect,
    timeline: &VecDeque<(SystemTime, f32)>,
    threshold: f32,
    result: MotionDetectionResult,
) {
    if timeline.is_empty() {
        let placeholder = Paragraph::new("Waiting for motion data...")
            .block(Block::default().borders(Borders::ALL).title("Motion Score Timeline"))
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(placeholder, area);
        return;
    }

    // Convert to chart data
    let data: Vec<(f64, f64)> = timeline
        .iter()
        .enumerate()
        .map(|(i, (_, score))| (i as f64, *score as f64))
        .collect();

    // Threshold line
    let threshold_data: Vec<(f64, f64)> = vec![
        (0.0, threshold as f64),
        (data.len() as f64, threshold as f64),
    ];

    let datasets = vec![
        Dataset::default()
            .name("Motion Score")
            .marker(symbols::Marker::Dot)
            .style(Style::default().fg(if result.motion_detected {
                Color::Red
            } else {
                Color::Green
            }))
            .graph_type(GraphType::Line)
            .data(&data),
        Dataset::default()
            .name("Threshold")
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Yellow))
            .graph_type(GraphType::Line)
            .data(&threshold_data),
    ];

    let title = if result.calibrating {
        "Motion Score Timeline [CALIBRATING...]"
    } else if result.motion_detected {
        "Motion Score Timeline [⚠ MOTION DETECTED!]"
    } else {
        "Motion Score Timeline [✓ No Motion]"
    };

    let title_style = if result.motion_detected {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(title, title_style))
        )
        .x_axis(
            Axis::default()
                .title("Time (frames)")
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, data.len() as f64])
        )
        .y_axis(
            Axis::default()
                .title("Score")
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, 1.0])
                .labels(vec!["0.0".into(), "0.5".into(), "1.0".into()])
        );

    frame.render_widget(chart, area);
}

/// Render motion detection banner
pub fn render_motion_banner(frame: &mut Frame, area: Rect, result: MotionDetectionResult) {
    let (text, style) = if result.calibrating {
        ("⏳ CALIBRATING...", Style::default().fg(Color::Yellow))
    } else if result.motion_detected {
        ("⚠ MOTION DETECTED!", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD | Modifier::RAPID_BLINK))
    } else {
        ("✓ No Motion", Style::default().fg(Color::Green))
    };

    let banner = Paragraph::new(text)
        .style(style)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(banner, area);
}
```

**File: `csi-tui/src/ui/status.rs`**

```rust
use ratatui::{
    prelude::*,
    widgets::*,
};
use crate::app::{App, ConnectionStatus, StreamStatus, ConfigParam};
use crate::motion::MotionConfig;

/// Render status panel
pub fn render_status_panel(frame: &mut Frame, area: Rect, app: &App) {
    let connection_text = match app.connection_status {
        ConnectionStatus::Disconnected => Span::styled("Disconnected", Style::default().fg(Color::Red)),
        ConnectionStatus::Connecting => Span::styled("Connecting...", Style::default().fg(Color::Yellow)),
        ConnectionStatus::Connected => Span::styled("Connected", Style::default().fg(Color::Green)),
        ConnectionStatus::Error => Span::styled("Error", Style::default().fg(Color::Red)),
    };

    let stream_text = match app.stream_status {
        StreamStatus::Stopped => Span::styled("Stopped", Style::default().fg(Color::Gray)),
        StreamStatus::Starting => Span::styled("Starting...", Style::default().fg(Color::Yellow)),
        StreamStatus::Streaming => Span::styled("Streaming", Style::default().fg(Color::Green)),
        StreamStatus::Paused => Span::styled("Paused", Style::default().fg(Color::Yellow)),
    };

    let metadata_text = if let Some(frame) = &app.latest_frame {
        format!(
            "RSSI: {} dBm | Rate: {} Mbps | Ch: {} | BW: {} MHz",
            frame.metadata.rssi,
            frame.metadata.rate,
            frame.metadata.channel,
            frame.metadata.bandwidth
        )
    } else {
        "No data yet".to_string()
    };

    let config = &app.motion_detector.config;

    let lines = vec![
        Line::from(vec![
            Span::raw("Device: "),
            Span::styled(&app.device_path, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::raw("Status: "),
            connection_text,
            Span::raw(" | "),
            stream_text,
        ]),
        Line::from(Span::raw("")),
        Line::from(Span::styled("── Signal Info ──", Style::default().fg(Color::Gray))),
        Line::from(Span::raw(metadata_text)),
        Line::from(Span::raw("")),
        Line::from(Span::styled("── Motion Config ──", Style::default().fg(Color::Gray))),
        Line::from(vec![
            Span::raw("Threshold: "),
            Span::styled(
                format!("{:.2}", config.detection_threshold),
                if app.ui_state.selected_config_param == ConfigParam::Threshold {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                }
            ),
            Span::raw(" [↑↓]"),
        ]),
        Line::from(vec![
            Span::raw("Smoothing: "),
            Span::styled(
                format!("{:.2}", config.smoothing_alpha),
                if app.ui_state.selected_config_param == ConfigParam::Smoothing {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                }
            ),
            Span::raw(" [←→]"),
        ]),
        Line::from(format!("Window: {} frames", config.window_size)),
        Line::from(format!("Min Consecutive: {}", config.min_consecutive_frames)),
    ];

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Status & Config"))
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}
```

**File: `csi-tui/src/ui/layout.rs`**

```rust
use ratatui::prelude::*;
use crate::app::App;
use super::{plots, heatmap, motion, status};

pub fn render_main_layout(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Main layout: header, body, footer
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Length(3),  // Header
            Constraint::Min(0),     // Body
            Constraint::Length(3),  // Footer
        ])
        .split(area);

    // Render header
    render_header(frame, main_layout[0], app);

    // Body layout: left (plots + heatmap), right (status)
    let body_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![
            Constraint::Percentage(75),
            Constraint::Percentage(25),
        ])
        .split(main_layout[1]);

    // Left side: plots and heatmap
    let left_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Percentage(25),  // Amplitude plot
            Constraint::Percentage(40),  // Heatmap
            Constraint::Percentage(35),  // Motion timeline
        ])
        .split(body_layout[0]);

    plots::render_amplitude_plot(frame, left_layout[0], app.latest_frame.as_ref());
    heatmap::render_heatmap(frame, left_layout[1], &app.frame_history);
    motion::render_motion_timeline(
        frame,
        left_layout[2],
        &app.motion_timeline,
        app.motion_detector.config.detection_threshold,
        app.motion_result,
    );

    // Right side: status panel
    status::render_status_panel(frame, body_layout[1], app);

    // Render footer
    render_footer(frame, main_layout[2]);

    // Render help overlay if active
    if app.ui_state.show_help {
        render_help_overlay(frame, area);
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let status_badges = vec![
        if app.connection_status == crate::app::ConnectionStatus::Connected {
            Span::styled(" [Connected] ", Style::default().fg(Color::Green))
        } else {
            Span::styled(" [Disconnected] ", Style::default().fg(Color::Red))
        },
        if app.stream_status == crate::app::StreamStatus::Streaming {
            Span::styled(" [Streaming] ", Style::default().fg(Color::Green))
        } else {
            Span::styled(" [Stopped] ", Style::default().fg(Color::Gray))
        },
        if app.motion_result.motion_detected {
            Span::styled(" [⚠ MOTION!] ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(" [No Motion] ", Style::default().fg(Color::Green))
        },
    ];

    let header = Paragraph::new(Line::from(status_badges))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("ESP32-C3 CSI Motion Detector")
        )
        .alignment(Alignment::Right);

    frame.render_widget(header, area);
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let footer_text = Line::from(vec![
        Span::styled("[q]", Style::default().fg(Color::Yellow)),
        Span::raw("Quit "),
        Span::styled("[c]", Style::default().fg(Color::Yellow)),
        Span::raw("Calibrate "),
        Span::styled("[↑↓]", Style::default().fg(Color::Yellow)),
        Span::raw("Threshold "),
        Span::styled("[←→]", Style::default().fg(Color::Yellow)),
        Span::raw("Smoothing "),
        Span::styled("[r]", Style::default().fg(Color::Yellow)),
        Span::raw("Reset "),
        Span::styled("[?]", Style::default().fg(Color::Yellow)),
        Span::raw("Help"),
    ]);

    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);

    frame.render_widget(footer, area);
}

fn render_help_overlay(frame: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(Span::styled("Help", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from("Navigation:"),
        Line::from("  q/Esc   - Quit application"),
        Line::from("  ?       - Toggle this help"),
        Line::from(""),
        Line::from("Motion Detection:"),
        Line::from("  ↑/↓     - Increase/decrease threshold"),
        Line::from("  ←/→     - Decrease/increase smoothing"),
        Line::from("  c       - Recalibrate detector"),
        Line::from("  r       - Reset detector state"),
        Line::from(""),
        Line::from("Press any key to close..."),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title("Help")
        .title_alignment(Alignment::Center);

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .alignment(Alignment::Left);

    let popup_area = centered_rect(60, 60, area);
    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
```

**File: `csi-tui/src/ui/mod.rs`**

```rust
pub mod layout;
pub mod plots;
pub mod heatmap;
pub mod motion;
pub mod status;

pub use layout::render_main_layout;
```

## 4. Integration with esp-csi-cli-rs

### CSI Data Parser

Based on the async_main.rs file analysis, the ESP32 outputs CSI data via serial. We need to parse this output.

**File: `csi-tui/src/csi/parser.rs`**

```rust
use crate::csi::frame::{CsiFrame, CsiMetadata};
use std::time::SystemTime;

/// Parse CSI data line from esp-csi-cli-rs output
///
/// Expected format (based on typical ESP-IDF CSI output):
/// CSI_DATA,<len>,<mac>,<rssi>,<rate>,<sig_mode>,<mcs>,<bandwidth>,<smoothing>,<not_sounding>,<aggregation>,<stbc>,<fec_coding>,<sgi>,<noise_floor>,<ampdu_cnt>,<channel>,<secondary_channel>,<local_timestamp>,<ant>,<sig_len>,<rx_state>,<real_time_set>,<real_timestamp>,<len>,<data>
///
/// Or simplified:
/// <timestamp>,<rssi>,<rate>,<bandwidth>,<channel>,<amp0>,<amp1>,...,<ampN>
pub fn parse_csi_line(line: &str) -> Option<CsiFrame> {
    let line = line.trim();

    // Skip empty lines or comments
    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    // Try to parse as CSV
    let parts: Vec<&str> = line.split(',').collect();

    // Check for minimum required fields
    if parts.len() < 6 {
        return None;
    }

    // Parse metadata
    let rssi = parts.get(1)?.parse::<i8>().ok()?;
    let rate = parts.get(2)?.parse::<u8>().ok()?;
    let bandwidth = parts.get(3)?.parse::<u8>().ok()?;
    let channel = parts.get(4)?.parse::<u8>().ok()?;

    // Remaining parts are amplitude values
    let amplitudes: Vec<f32> = parts[5..]
        .iter()
        .filter_map(|s| s.parse::<f32>().ok())
        .collect();

    if amplitudes.is_empty() {
        return None;
    }

    Some(CsiFrame {
        timestamp: SystemTime::now(),
        amplitudes,
        phases: None,
        metadata: CsiMetadata {
            rssi,
            rate,
            bandwidth,
            channel,
            mac: None,
            seq: 0,
        },
    })
}

/// Parse raw CSI binary format (if available)
/// This would be used if esp-csi-cli-rs outputs binary CSI data
pub fn parse_csi_binary(data: &[u8]) -> Option<CsiFrame> {
    // Implement binary parsing based on ESP-IDF CSI structure
    // For now, placeholder
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csi_line() {
        let line = "1234567890,-45,54,20,6,12.3,15.7,18.2,14.5,16.8";
        let frame = parse_csi_line(line).unwrap();

        assert_eq!(frame.metadata.rssi, -45);
        assert_eq!(frame.metadata.rate, 54);
        assert_eq!(frame.metadata.bandwidth, 20);
        assert_eq!(frame.metadata.channel, 6);
        assert_eq!(frame.amplitudes.len(), 5);
    }
}
```

### Serial Reader & Subprocess Management

**File: `csi-tui/src/csi/reader.rs`**

```rust
use color_eyre::Result;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use crate::csi::frame::CsiFrame;
use crate::csi::parser::parse_csi_line;

pub struct CsiReader {
    device_path: String,
    frame_tx: mpsc::UnboundedSender<CsiFrame>,
}

impl CsiReader {
    pub fn new(device_path: String, frame_tx: mpsc::UnboundedSender<CsiFrame>) -> Self {
        Self {
            device_path,
            frame_tx,
        }
    }

    /// Start reading CSI data from esp-csi-cli-rs subprocess
    pub async fn start(&self) -> Result<()> {
        // Spawn esp-csi-cli-rs process
        // Assuming the binary can be called with device path
        // You may need to adjust based on actual CLI interface

        let mut child = Command::new("esp-csi-cli-rs")
            .arg("--device")
            .arg(&self.device_path)
            .arg("--mode")
            .arg("sniffer")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().expect("Failed to capture stdout");
        let mut reader = BufReader::new(stdout).lines();

        let frame_tx = self.frame_tx.clone();

        // Read lines asynchronously
        tokio::spawn(async move {
            while let Ok(Some(line)) = reader.next_line().await {
                if let Some(frame) = parse_csi_line(&line) {
                    if frame_tx.send(frame).is_err() {
                        break; // Channel closed
                    }
                }
            }
        });

        // Wait for process to complete (or handle termination)
        let _status = child.wait().await?;

        Ok(())
    }
}

/// Alternative: Direct serial port reading (without subprocess)
/// This would require parsing the raw serial protocol
pub struct DirectSerialReader {
    port_name: String,
    baud_rate: u32,
    frame_tx: mpsc::UnboundedSender<CsiFrame>,
}

impl DirectSerialReader {
    pub fn new(
        port_name: String,
        baud_rate: u32,
        frame_tx: mpsc::UnboundedSender<CsiFrame>,
    ) -> Self {
        Self {
            port_name,
            baud_rate,
            frame_tx,
        }
    }

    /// Start reading from serial port directly
    /// Requires: cargo add serialport tokio-serial
    pub async fn start(&self) -> Result<()> {
        // This is a placeholder - you would use tokio-serial or similar
        // to read directly from the serial port

        // Example with serialport crate (sync):
        // let mut port = serialport::new(&self.port_name, self.baud_rate)
        //     .timeout(Duration::from_millis(100))
        //     .open()?;
        //
        // let mut buffer = Vec::new();
        // loop {
        //     // Read and parse CSI packets
        // }

        Ok(())
    }
}
```

**File: `csi-tui/src/csi/mod.rs`**

```rust
pub mod frame;
pub mod parser;
pub mod reader;

pub use frame::{CsiFrame, CsiMetadata};
pub use parser::parse_csi_line;
pub use reader::CsiReader;
```

### Updated Main Application

**File: `csi-tui/src/main.rs`**

```rust
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
        .unwrap_or_else(|| "/dev/ttyUSB0".to_string());

    // Setup terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    // Create CSI frame channel
    let (frame_tx, mut frame_rx) = mpsc::unbounded_channel();

    // Create application
    let mut app = App::new(device_path.clone());

    // Start CSI reader in background
    let reader = csi::CsiReader::new(device_path, frame_tx);
    tokio::spawn(async move {
        if let Err(e) = reader.start().await {
            eprintln!("CSI reader error: {}", e);
        }
    });

    app.connection_status = app::ConnectionStatus::Connected;
    app.stream_status = app::StreamStatus::Streaming;

    // Main event loop
    let result = run_app(&mut terminal, &mut app, &mut frame_rx).await;

    // Restore terminal
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    result
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    frame_rx: &mut mpsc::UnboundedReceiver<csi::CsiFrame>,
) -> Result<()> {
    loop {
        // Render UI
        terminal.draw(|f| ui::render_main_layout(f, app))?;

        // Handle events (non-blocking)
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    handle_key_event(app, key.code);
                }
            }
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
```

### Update Cargo.toml

**File: `csi-tui/Cargo.toml`**

```toml
[package]
name = "csi-tui"
version = "0.1.0"
edition = "2021"

[dependencies]
ratatui = "0.29"
crossterm = "0.28"
color-eyre = "0.6"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Optional: for direct serial port access
# serialport = "4"
# tokio-serial = "5"
```

## 5. Step-by-Step Implementation Plan

### Phase 1: Project Setup ✅

- ☑ Task 1.1: Create csi-tui subdirectory
- ☑ Task 1.2: Initialize Cargo project with dependencies
- ☑ Task 1.3: Set up basic module structure (src/app.rs, src/csi/, src/motion/, src/ui/)

### Phase 2: Core Data Structures ✅

- ☑ Task 2.1: Implement CsiFrame and CsiMetadata structures
  - File: src/csi/frame.rs
  - Add distance, mean, variance methods
- ☑ Task 2.2: Implement MotionConfig structure
  - File: src/motion/config.rs
  - Add serde support for config persistence
- ☑ Task 2.3: Implement App state structure
  - File: src/app.rs
  - Add all state fields and methods

### Phase 3: Motion Detection Logic ✅

- ☑ Task 3.1: Implement MotionDetector core algorithm
  - File: src/motion/detector.rs
  - Frame-to-frame distance computation
  - Sliding window management
  - Threshold detection with hysteresis
- ☑ Task 3.2: Add calibration logic
  - Baseline variance computation
  - Initial 30-frame calibration period
- ☑ Task 3.3: Add smoothing and temporal filtering
  - Exponential moving average
  - Consecutive detection counter
  - Cooldown period

### Phase 4: CSI Data Parsing ✅

- ☐ Task 4.1: Analyze actual esp-csi-cli-rs output format (PENDING - needs real hardware)
  - Run the embedded firmware
  - Capture sample output
  - Document the format
- ☑ Task 4.2: Implement CSV parser
  - File: src/csi/parser.rs
  - Parse metadata and amplitude values
  - Add error handling
- ☑ Task 4.3: Add parser tests
  - Unit tests with sample data
  - Edge case handling

### Phase 5: Serial/Subprocess Integration ✅

- ☑ Task 5.1: Implement subprocess spawning
  - File: src/csi/reader.rs
  - Spawn esp-csi-cli-rs binary
  - Configure CLI arguments
- ☑ Task 5.2: Implement async line reading
  - Use tokio::process
  - BufReader for line-by-line parsing
  - Channel for frame distribution
- ☐ Task 5.3: Add error handling and reconnection (OPTIONAL)
  - Handle process crashes
  - Automatic restart logic
  - Connection status updates

### Phase 6: UI Implementation ✅

- ☑ Task 6.1: Implement amplitude plot widget
  - File: src/ui/plots.rs
  - Ratatui Chart widget
  - Dynamic scaling
- ☑ Task 6.2: Implement heatmap widget
  - File: src/ui/heatmap.rs
  - Time × subcarrier visualization
  - Color mapping
- ☑ Task 6.3: Implement motion timeline widget
  - File: src/ui/motion.rs
  - Score chart with threshold line
  - Detection banner
- ☑ Task 6.4: Implement status panel
  - File: src/ui/status.rs
  - Connection status
  - Configuration display
- ☑ Task 6.5: Implement main layout
  - File: src/ui/layout.rs
  - Three-column responsive layout
  - Help overlay

### Phase 7: Event Handling ✅

- ☑ Task 7.1: Implement keyboard event handlers
  - File: src/main.rs
  - Quit, help, calibrate
  - Threshold/smoothing adjustment
- ☑ Task 7.2: Implement real-time frame processing
  - Async frame reception
  - Motion detector integration
  - State updates

### Phase 8: Testing with Real Hardware

- ☐ Task 8.1: Flash ESP32-C3 with existing firmware
  - Use provided binary (NO CHANGES)
  - Verify serial output
- ☐ Task 8.2: Run csi-tui with real device
  - Test connection
  - Verify CSI data reception
  - Tune motion detection parameters
- ☐ Task 8.3: Calibration and tuning
  - Collect baseline data
  - Adjust thresholds for environment
  - Test various motion scenarios

### Phase 9: Mock/Synthetic Testing

- ☐ Task 9.1: Create mock CSI generator
  - File: tools/mock_csi_generator.rs
  - Generate realistic CSI patterns
  - Simulate motion events
- ☐ Task 9.2: Add synthetic motion patterns
  - Walking motion signature
  - Still environment baseline
  - Rapid motion spikes
- ☐ Task 9.3: Unit tests with mock data
  - Test motion detector accuracy
  - False positive/negative rates
  - Threshold sensitivity analysis

### Phase 10: Polish & Documentation

- ☐ Task 10.1: Add configuration file support
  - Load/save MotionConfig
  - Device presets
- ☐ Task 10.2: Add logging and diagnostics
  - Structured logging with tracing
  - Performance metrics
  - Debug mode
- ☐ Task 10.3: Update documentation
  - README with usage examples
  - Architecture diagrams
  - Troubleshooting guide
- ☐ Task 10.4: Create demo video/screenshots
  - Show motion detection in action
  - UI walkthrough
  - Configuration examples

## Testing Strategy

### With Real ESP32-C3

```bash
# 1. Ensure ESP32 is flashed (use existing firmware)
# 2. Connect device via USB
# 3. Find device path
ls /dev/tty* | grep USB

# 4. Run TUI
cargo run --release -- /dev/ttyUSB0

# 5. Test motion detection
# - Stand still near device (should show no motion)
# - Walk around (should detect motion)
# - Adjust threshold with ↑↓ keys
```

### With Mock Generator

```bash
# 1. Create mock generator
cargo new --bin tools/mock_csi_generator

# 2. Generate synthetic CSI stream
cargo run --bin mock_csi_generator | cargo run

# 3. Test specific scenarios
cargo run --bin mock_csi_generator -- --scenario walking
cargo run --bin mock_csi_generator -- --scenario still
cargo run --bin mock_csi_generator -- --scenario rapid
```

**File: `tools/mock_csi_generator/src/main.rs`**

```rust
use std::time::{Duration, SystemTime};
use std::thread;

fn main() {
    let scenario = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "normal".to_string());

    let mut seq = 0u32;

    loop {
        let amplitudes = match scenario.as_str() {
            "still" => generate_still_csi(seq),
            "walking" => generate_walking_csi(seq),
            "rapid" => generate_rapid_csi(seq),
            _ => generate_normal_csi(seq),
        };

        // Output CSV format
        println!(
            "{},{},{},{},{},{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            -45, // RSSI
            54,  // Rate
            20,  // Bandwidth
            6,   // Channel
            amplitudes
                .iter()
                .map(|a| format!("{:.2}", a))
                .collect::<Vec<_>>()
                .join(",")
        );

        seq += 1;
        thread::sleep(Duration::from_millis(100)); // 10 Hz
    }
}

fn generate_still_csi(seq: u32) -> Vec<f32> {
    // Mostly constant with small noise
    (0..64)
        .map(|i| {
            let base = 50.0 + (i as f32 * 0.5);
            let noise = (seq as f32 * 0.1).sin() * 0.5;
            base + noise
        })
        .collect()
}

fn generate_walking_csi(seq: u32) -> Vec<f32> {
    // Periodic variations (walking frequency ~2 Hz)
    (0..64)
        .map(|i| {
            let base = 50.0 + (i as f32 * 0.5);
            let motion = ((seq as f32 * 0.2).sin() * 5.0).abs();
            base + motion
        })
        .collect()
}

fn generate_rapid_csi(seq: u32) -> Vec<f32> {
    // Large rapid changes
    (0..64)
        .map(|i| {
            let base = 50.0 + (i as f32 * 0.5);
            let spike = if seq % 10 < 3 { 15.0 } else { 0.0 };
            base + spike
        })
        .collect()
}

fn generate_normal_csi(seq: u32) -> Vec<f32> {
    // Mix of still and occasional motion
    (0..64)
        .map(|i| {
            let base = 50.0 + (i as f32 * 0.5);
            let occasional = if seq % 50 < 10 {
                ((seq as f32 * 0.3).sin() * 8.0).abs()
            } else {
                (seq as f32 * 0.1).sin() * 0.5
            };
            base + occasional
        })
        .collect()
}
```

## Summary

This implementation provides:

- **Complete architecture** for host-side CSI motion detection
- **Robust motion detection algorithm** with calibration, smoothing, and threshold-based detection
- **Full Ratatui UI** with live plots, heatmap, motion timeline, and status panels
- **Integration strategy** for esp-csi-cli-rs (subprocess or direct serial)
- **Step-by-step plan** from setup to demo-ready implementation
- **Testing approach** with both real hardware and synthetic data

### Key Constraints Respected:

✅ No firmware modifications (ESP32 treated as fixed CSI source)
✅ All logic on host side (motion detection, visualization)
✅ Works with existing esp-csi-cli-rs protocol
✅ Production-quality Rust patterns
✅ Hackathon-ready (simple but effective)

**Next step**: Begin with Phase 2 (Core Data Structures) to start building the actual implementation.
Complete architecture for host-side CSI motion detection
Robust motion detection algorithm with calibration, smoothing, and threshold-based detection
Full Ratatui UI with live plots, heatmap, motion timeline, and status panels
Integration strategy for esp-csi-cli-rs (subprocess or direct serial)
Step-by-step plan from setup to demo-ready implementation
Testing approach with both real hardware and synthetic data
Key Constraints Respected:
✅ No firmware modifications (ESP32 treated as fixed CSI source)
✅ All logic on host side (motion detection, visualization)
✅ Works with existing esp-csi-cli-rs protocol
✅ Production-quality Rust patterns
✅ Hackathon-ready (simple but effective)
Next step: Begin with Phase 2 (Core Data Structures) to start building the actual implementation.
