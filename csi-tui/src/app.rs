use crate::csi::frame::CsiFrame;
use crate::motion::{MotionConfig, MotionDetector, MotionDetectionResult};
use std::collections::VecDeque;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
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
    #[allow(dead_code)]
    pub error_message: Option<String>,

    /// UI state
    pub ui_state: UiState,
}

#[derive(Debug, Clone)]
pub struct UiState {
    /// Currently selected tab
    #[allow(dead_code)]
    pub selected_tab: usize,

    /// Whether config editing mode is active
    #[allow(dead_code)]
    pub editing_config: bool,

    /// Which config parameter is selected
    pub selected_config_param: ConfigParam,

    /// Show help overlay
    pub show_help: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
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
