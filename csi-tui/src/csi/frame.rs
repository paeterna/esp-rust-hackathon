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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub mac: Option<[u8; 6]>,

    /// Sequence number
    #[allow(dead_code)]
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
