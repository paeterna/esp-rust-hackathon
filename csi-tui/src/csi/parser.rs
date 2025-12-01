use crate::csi::frame::{CsiFrame, CsiMetadata};
use std::time::SystemTime;

/// Parser for ESP32 CSI data that spans multiple lines
///
/// The ESP32 outputs CSI data in the following format:
/// ```
/// mac: XX:XX:XX:XX:XX:XX
/// rssi: -68
/// rate: 11
/// noise floor: 160
/// channel: 1
/// timestamp: 103572861
/// ...other metadata...
/// data length: 128
/// csi raw data:
/// [0, 0, 0, -14, -1, -15, ...]
/// ```
#[derive(Debug, Default)]
pub struct CsiParser {
    current_rssi: Option<i8>,
    current_rate: Option<u8>,
    current_channel: Option<u8>,
    current_mac: Option<String>,
    current_bandwidth: Option<u8>,
}

impl CsiParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a line of CSI output and potentially return a complete frame
    pub fn parse_line(&mut self, line: &str) -> Option<CsiFrame> {
        let line = line.trim();

        // Skip empty lines
        if line.is_empty() {
            return None;
        }

        // Parse metadata fields
        if let Some(value) = line.strip_prefix("mac: ") {
            self.current_mac = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("rssi: ") {
            self.current_rssi = value.parse().ok();
        } else if let Some(value) = line.strip_prefix("rate: ") {
            self.current_rate = value.parse().ok();
        } else if let Some(value) = line.strip_prefix("channel: ") {
            self.current_channel = value.parse().ok();
        } else if let Some(value) = line.strip_prefix("cwb: ") {
            // cwb is the channel width/bandwidth
            self.current_bandwidth = value.parse().ok();
        } else if line.starts_with("csi raw data:") {
            // Next line should contain the actual CSI data
            // We'll need to handle this in the reader
        } else if line.starts_with('[') && line.ends_with(']') {
            // This is the CSI raw data array
            return self.parse_csi_data_array(line);
        }

        None
    }

    /// Parse the CSI data array line
    fn parse_csi_data_array(&mut self, line: &str) -> Option<CsiFrame> {
        // Remove brackets and split by comma
        let data_str = line.trim_start_matches('[').trim_end_matches(']');
        let values: Vec<&str> = data_str.split(',').collect();

        // Parse pairs of I/Q values and compute magnitude
        let mut amplitudes = Vec::new();
        let mut i = 0;

        while i + 1 < values.len() {
            let real: f32 = values[i].trim().parse().ok()?;
            let imag: f32 = values[i + 1].trim().parse().ok()?;

            // Compute magnitude: sqrt(real^2 + imag^2)
            let magnitude = (real * real + imag * imag).sqrt();
            amplitudes.push(magnitude);

            i += 2;
        }

        // Only return a frame if we have all required metadata
        if amplitudes.is_empty() {
            return None;
        }

        let rssi = self.current_rssi?;
        let rate = self.current_rate?;
        let channel = self.current_channel?;
        let bandwidth = self.current_bandwidth.unwrap_or(20); // Default to 20MHz

        // Create the frame
        let frame = CsiFrame {
            timestamp: SystemTime::now(),
            amplitudes,
            phases: None,
            metadata: CsiMetadata {
                rssi,
                rate,
                bandwidth,
                channel,
                mac: None, // We could parse the MAC string if needed
                seq: 0,
            },
        };

        // Reset state for next frame
        self.current_rssi = None;
        self.current_rate = None;
        self.current_channel = None;
        self.current_mac = None;
        self.current_bandwidth = None;

        Some(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csi_frame() {
        let mut parser = CsiParser::new();

        // Simulate receiving lines from ESP32
        assert!(parser.parse_line("mac: CE:BD:8D:09:24:B7").is_none());
        assert!(parser.parse_line("rssi: -68").is_none());
        assert!(parser.parse_line("rate: 11").is_none());
        assert!(parser.parse_line("channel: 1").is_none());
        assert!(parser.parse_line("cwb: 0").is_none());
        assert!(parser.parse_line("csi raw data:").is_none());

        let frame = parser.parse_line("[0, 0, 0, -14, -1, -15, -2, -15]");
        assert!(frame.is_some());

        let frame = frame.unwrap();
        assert_eq!(frame.metadata.rssi, -68);
        assert_eq!(frame.metadata.rate, 11);
        assert_eq!(frame.metadata.channel, 1);
        assert_eq!(frame.amplitudes.len(), 4); // 8 values / 2 = 4 complex samples
    }
}
