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
