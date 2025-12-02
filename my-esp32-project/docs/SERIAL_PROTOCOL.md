# Serial Protocol Specification

## Overview

Communication protocol between ESP32-C3 firmware and TUI application via USB serial.

- **Baud Rate**: 115200 (configurable)
- **Data Format**: Binary with text fallback
- **Flow Control**: None (software handshake via ACK/NAK)
- **Byte Order**: Little Endian

## Message Format

### Frame Structure

```
┌─────────┬─────────┬─────────┬──────────┬─────────┐
│ Header  │ Type    │ Length  │ Payload  │ CRC-8   │
│ (2B)    │ (1B)    │ (2B)    │ (var)    │ (1B)    │
└─────────┴─────────┴─────────┴──────────┴─────────┘
```

- **Header**: Magic bytes `0xC5 0x1A` (CSI header)
- **Type**: Message type (see below)
- **Length**: Payload length (0-1024 bytes)
- **Payload**: Message-specific data
- **CRC**: CRC-8 checksum of Type + Length + Payload

## Message Types

### Host → ESP32 (Commands)

| Type | Value | Name | Description |
|------|-------|------|-------------|
| CMD_PING | 0x01 | Ping | Check device connectivity |
| CMD_START_CSI | 0x10 | Start CSI | Begin CSI collection |
| CMD_STOP_CSI | 0x11 | Stop CSI | Stop CSI collection |
| CMD_CONFIG_CSI | 0x12 | Configure CSI | Set CSI parameters |
| CMD_SET_CHANNEL | 0x20 | Set WiFi Channel | Change WiFi channel (1-13) |
| CMD_GET_STATUS | 0x30 | Get Status | Request device status |
| CMD_RESET | 0xFF | Reset | Reset device |

### ESP32 → Host (Responses)

| Type | Value | Name | Description |
|------|-------|------|-------------|
| RSP_ACK | 0x81 | Acknowledge | Command accepted |
| RSP_NAK | 0x82 | Not Acknowledge | Command rejected |
| RSP_CSI_DATA | 0x90 | CSI Data | CSI data packet |
| RSP_STATUS | 0xA0 | Status | Device status info |
| RSP_ERROR | 0xE0 | Error | Error message |

## Message Payloads

### CMD_START_CSI (0x10)

**Payload**: None

**Example**:
```
C5 1A 10 00 00 XX
```

### CMD_CONFIG_CSI (0x12)

**Payload** (8 bytes):
```
┌──────────┬──────────┬──────────┬──────────┐
│ Flags    │ Filter   │ Reserved │ Reserved │
│ (1B)     │ (1B)     │ (3B)     │ (3B)     │
└──────────┴──────────┴──────────┴──────────┘
```

**Flags** (bitmask):
- Bit 0: LLTF enable
- Bit 1: HT-LTF enable
- Bit 2: STBC HT-LTF2 enable
- Bit 3: LTF merge enable
- Bit 4: Channel filter enable
- Bit 5: Manual scale
- Bits 6-7: Reserved

**Example** (all features enabled):
```
C5 1A 12 08 00 3F 01 00 00 00 00 00 00 XX
```

### CMD_SET_CHANNEL (0x20)

**Payload** (1 byte):
```
┌──────────┐
│ Channel  │
│ (1B)     │
└──────────┘
```

**Example** (set to channel 6):
```
C5 1A 20 01 00 06 XX
```

### RSP_ACK (0x81)

**Payload** (1 byte):
```
┌──────────┐
│ Cmd Type │
│ (1B)     │
└──────────┘
```

**Example** (ACK for START_CSI):
```
C5 1A 81 01 00 10 XX
```

### RSP_CSI_DATA (0x90)

**Payload**:
```
┌──────────┬──────────┬──────────┬──────────┬──────────┬──────────┐
│Timestamp │ RSSI     │ Rate     │ Channel  │ MAC      │ CSI Data │
│(4B)      │(1B)      │(1B)      │(1B)      │(6B)      │(variable)│
└──────────┴──────────┴──────────┴──────────┴──────────┴──────────┘
```

**CSI Data Format**:
```
┌──────────┬──────────────────────────────────┐
│ Count    │ Subcarrier Data                  │
│ (1B)     │ (Count × 4 bytes)                │
└──────────┴──────────────────────────────────┘

Subcarrier Data (4 bytes per subcarrier):
┌──────────┬──────────┐
│ I (Real) │ Q (Imag) │
│ (2B)     │ (2B)     │
└──────────┴──────────┘
```

**Example** (2 subcarriers):
```
C5 1A 90 0F 00 
[Timestamp: 4B] 
[RSSI: -45 dBm = 0xD3] 
[Rate: MCS0 = 0x00] 
[Channel: 6 = 0x06]
[MAC: 6B]
[Count: 2 = 0x02]
[I0: 2B][Q0: 2B]
[I1: 2B][Q1: 2B]
XX
```

### RSP_STATUS (0xA0)

**Payload**:
```
┌──────────┬──────────┬──────────┬──────────┬──────────┐
│ State    │ Channel  │ CSI Count│ Dropped  │ Uptime   │
│ (1B)     │ (1B)     │ (4B)     │ (4B)     │ (4B)     │
└──────────┴──────────┴──────────┴──────────┴──────────┘
```

**State values**:
- 0x00: IDLE
- 0x01: CSI_ACTIVE
- 0x02: ERROR

### RSP_ERROR (0xE0)

**Payload**:
```
┌──────────┬──────────────────┐
│ Code     │ Message          │
│ (1B)     │ (variable)       │
└──────────┴──────────────────┘
```

**Error codes**:
- 0x01: INVALID_COMMAND
- 0x02: INVALID_PARAMETER
- 0x03: CSI_INIT_FAILED
- 0x04: CSI_ALREADY_RUNNING
- 0x05: CSI_NOT_RUNNING
- 0xFF: UNKNOWN_ERROR

## Example Communication Session

```
# 1. Host sends PING
→ C5 1A 01 00 00 XX

# 2. ESP32 responds with ACK
← C5 1A 81 01 00 01 XX

# 3. Host configures CSI
→ C5 1A 12 08 00 3F 01 00 00 00 00 00 00 XX

# 4. ESP32 ACKs configuration
← C5 1A 81 01 00 12 XX

# 5. Host starts CSI collection
→ C5 1A 10 00 00 XX

# 6. ESP32 ACKs start
← C5 1A 81 01 00 10 XX

# 7. ESP32 streams CSI data (continuous)
← C5 1A 90 [length] [CSI data...] XX
← C5 1A 90 [length] [CSI data...] XX
← C5 1A 90 [length] [CSI data...] XX
...

# 8. Host requests status
→ C5 1A 30 00 00 XX

# 9. ESP32 sends status
← C5 1A A0 0E 00 [status data] XX

# 10. Host stops CSI
→ C5 1A 11 00 00 XX

# 11. ESP32 ACKs stop
← C5 1A 81 01 00 11 XX
```

## Implementation Notes

### ESP32 Side (Embedded)

```rust
// Message frame
#[repr(C, packed)]
struct Frame {
    header: [u8; 2],  // [0xC5, 0x1A]
    msg_type: u8,
    length: u16,
    // payload: [u8; length]
    // crc: u8
}

// CSI data structure
#[repr(C, packed)]
struct CsiDataMsg {
    timestamp: u32,
    rssi: i8,
    rate: u8,
    channel: u8,
    mac: [u8; 6],
    count: u8,
    // subcarriers: [Complex<i16>; count]
}
```

### TUI Side (Host)

```rust
// Message types
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum MessageType {
    // Commands
    CmdPing = 0x01,
    CmdStartCsi = 0x10,
    CmdStopCsi = 0x11,
    CmdConfigCsi = 0x12,
    CmdSetChannel = 0x20,
    CmdGetStatus = 0x30,
    CmdReset = 0xFF,
    
    // Responses
    RspAck = 0x81,
    RspNak = 0x82,
    RspCsiData = 0x90,
    RspStatus = 0xA0,
    RspError = 0xE0,
}

// Frame parser
fn parse_frame(data: &[u8]) -> Result<Frame> {
    if data.len() < 6 {
        return Err("Incomplete frame");
    }
    if data[0] != 0xC5 || data[1] != 0x1A {
        return Err("Invalid header");
    }
    // Parse rest...
}
```

## CRC-8 Calculation

**Polynomial**: 0x07 (x^8 + x^2 + x + 1)  
**Initial value**: 0x00  
**Final XOR**: 0x00

```rust
fn calculate_crc8(data: &[u8]) -> u8 {
    let mut crc: u8 = 0;
    for byte in data {
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
```

## Timing Specifications

- **Command timeout**: 1000ms
- **ACK timeout**: 100ms
- **CSI data rate**: Target 100 Hz (10ms period)
- **Max packet size**: 1024 bytes
- **Status update interval**: 1000ms (when requested)

## Error Handling

1. **Timeout**: Retransmit command (max 3 attempts)
2. **CRC error**: Discard packet, request retransmit
3. **Invalid frame**: Log error, continue
4. **Buffer overflow**: Drop oldest packets
5. **NAK received**: Parse error code, display to user

## Future Extensions

- **Version field**: Protocol version negotiation
- **Compression**: LZ4 compression for CSI data
- **Encryption**: Optional AES encryption
- **Batch mode**: Multiple CSI samples per packet
- **Calibration**: PHY calibration data exchange
