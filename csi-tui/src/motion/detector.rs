use crate::csi::frame::CsiFrame;
use crate::motion::config::MotionConfig;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct MotionDetector {
    pub config: MotionConfig,

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
    #[allow(dead_code)]
    pub fn motion_score(&self) -> f32 {
        self.motion_score
    }

    /// Get detection state
    #[allow(dead_code)]
    pub fn is_motion_detected(&self) -> bool {
        self.motion_detected
    }

    /// Get motion score history for visualization
    #[allow(dead_code)]
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
