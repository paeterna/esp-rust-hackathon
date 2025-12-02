use anyhow::{anyhow, Result};
use num_complex::Complex;

const MAGIC_HEADER: [u8; 2] = [0xC5, 0x1A];

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum MessageType {
    // Commands (Host → ESP32)
    CmdPing = 0x01,
    CmdStartCsi = 0x10,
    CmdStopCsi = 0x11,
    CmdConfigCsi = 0x12,
    CmdSetChannel = 0x20,
    CmdGetStatus = 0x30,
    CmdReset = 0xFF,

    // Responses (ESP32 → Host)
    RspAck = 0x81,
    RspNak = 0x82,
    RspCsiData = 0x90,
    RspStatus = 0xA0,
    RspError = 0xE0,
}

#[derive(Debug, Clone)]
pub struct CsiDataPacket {
    pub timestamp: u32,
    pub rssi: i8,
    #[allow(dead_code)]
    pub rate: u8,
    #[allow(dead_code)]
    pub channel: u8,
    #[allow(dead_code)]
    pub mac: [u8; 6],
    pub subcarriers: Vec<Complex<f32>>,
}

pub struct ProtocolHandler {
    buffer: Vec<u8>,
}

impl ProtocolHandler {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
        }
    }

    pub fn create_command(&self, msg_type: MessageType, payload: &[u8]) -> Vec<u8> {
        let mut frame = Vec::new();
        
        // Header
        frame.extend_from_slice(&MAGIC_HEADER);
        
        // Type
        frame.push(msg_type as u8);
        
        // Length (little endian)
        let len = payload.len() as u16;
        frame.extend_from_slice(&len.to_le_bytes());
        
        // Payload
        frame.extend_from_slice(payload);
        
        // CRC-8
        let crc = self.calculate_crc8(&frame[2..]); // CRC over type + length + payload
        frame.push(crc);
        
        frame
    }

    pub fn create_start_csi_command(&self) -> Vec<u8> {
        self.create_command(MessageType::CmdStartCsi, &[])
    }

    pub fn create_stop_csi_command(&self) -> Vec<u8> {
        self.create_command(MessageType::CmdStopCsi, &[])
    }

    #[allow(dead_code)]
    pub fn create_ping_command(&self) -> Vec<u8> {
        self.create_command(MessageType::CmdPing, &[])
    }

    pub fn add_data(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    pub fn try_parse_frame(&mut self) -> Result<Option<ParsedFrame>> {
        // Need at least header + type + length + crc = 6 bytes
        if self.buffer.len() < 6 {
            return Ok(None);
        }

        // Look for magic header
        if self.buffer[0] != MAGIC_HEADER[0] || self.buffer[1] != MAGIC_HEADER[1] {
            // Try to find next valid header
            if let Some(pos) = self.find_next_header() {
                self.buffer.drain(0..pos);
                return Ok(None);
            } else {
                self.buffer.clear();
                return Ok(None);
            }
        }

        // Parse length
        let msg_type = self.buffer[2];
        let length = u16::from_le_bytes([self.buffer[3], self.buffer[4]]) as usize;

        // Check if we have complete frame
        let frame_size = 6 + length; // header(2) + type(1) + length(2) + payload(length) + crc(1)
        if self.buffer.len() < frame_size {
            return Ok(None); // Need more data
        }

        // Verify CRC
        let expected_crc = self.buffer[frame_size - 1];
        let actual_crc = self.calculate_crc8(&self.buffer[2..frame_size - 1]);
        
        if expected_crc != actual_crc {
            // Bad CRC, skip this frame
            self.buffer.drain(0..2);
            return Err(anyhow!("CRC mismatch"));
        }

        // Extract payload
        let payload = self.buffer[5..frame_size - 1].to_vec();

        // Remove processed frame
        self.buffer.drain(0..frame_size);

        // Parse based on message type
        match msg_type {
            0x90 => Ok(Some(ParsedFrame::CsiData(self.parse_csi_data(&payload)?))),
            0x81 => Ok(Some(ParsedFrame::Ack)),
            0x82 => Ok(Some(ParsedFrame::Nak)),
            0xA0 => Ok(Some(ParsedFrame::Status)),
            0xE0 => Ok(Some(ParsedFrame::Error)),
            _ => Ok(Some(ParsedFrame::Unknown(msg_type))),
        }
    }

    fn find_next_header(&self) -> Option<usize> {
        for i in 1..self.buffer.len() - 1 {
            if self.buffer[i] == MAGIC_HEADER[0] && self.buffer[i + 1] == MAGIC_HEADER[1] {
                return Some(i);
            }
        }
        None
    }

    fn parse_csi_data(&self, payload: &[u8]) -> Result<CsiDataPacket> {
        if payload.len() < 13 {
            return Err(anyhow!("CSI data payload too short"));
        }

        let timestamp = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
        let rssi = payload[4] as i8;
        let rate = payload[5];
        let channel = payload[6];
        let mut mac = [0u8; 6];
        mac.copy_from_slice(&payload[7..13]);

        let count = payload[13] as usize;
        let mut subcarriers = Vec::new();

        let expected_len = 14 + count * 4; // header(13) + count(1) + data(count * 4)
        if payload.len() < expected_len {
            return Err(anyhow!("Incomplete CSI subcarrier data"));
        }

        for i in 0..count {
            let offset = 14 + i * 4;
            let re = i16::from_le_bytes([payload[offset], payload[offset + 1]]) as f32 / 100.0;
            let im = i16::from_le_bytes([payload[offset + 2], payload[offset + 3]]) as f32 / 100.0;
            subcarriers.push(Complex::new(re, im));
        }

        Ok(CsiDataPacket {
            timestamp,
            rssi,
            rate,
            channel,
            mac,
            subcarriers,
        })
    }

    fn calculate_crc8(&self, data: &[u8]) -> u8 {
        let mut crc: u8 = 0;
        for &byte in data {
            crc ^= byte;
            for _ in 0..8 {
                if (crc & 0x80) != 0 {
                    crc = (crc << 1) ^ 0x07;
                } else {
                    crc <<= 1;
                }
            }
        }
        crc
    }
}

#[derive(Debug)]
pub enum ParsedFrame {
    CsiData(CsiDataPacket),
    Ack,
    Nak,
    Status,
    Error,
    #[allow(dead_code)]
    Unknown(u8),
}
