# CSI Data Collection Implementation Guide

## What is Channel State Information (CSI)?

CSI describes how a WiFi signal travels from transmitter to receiver. It captures:
- **Amplitude**: Signal strength per subcarrier
- **Phase**: Signal phase per subcarrier  
- **Frequency**: Information across multiple OFDM subcarriers

CSI is richer than RSSI (single value) - it provides detailed channel characteristics useful for:
- Indoor positioning
- Motion detection
- Gesture recognition
- Through-wall sensing

## ESP32-C3 CSI Capabilities

The ESP32-C3 WiFi chip can capture CSI data from:
- Beacon frames
- Data packets
- Management frames

Each CSI sample contains:
- 64 subcarriers (for 20MHz bandwidth)
- Complex numbers (I/Q values) per subcarrier
- Metadata (MAC address, RSSI, timestamp, etc.)

## Implementation Steps

### 1. Enable CSI in WiFi Stack

The `esp-wifi` crate needs to be configured to expose CSI data. Currently, CSI support in esp-wifi is limited, so you may need to:

**Option A**: Use the low-level esp-wifi-sys bindings
**Option B**: Wait for/contribute CSI support to esp-wifi
**Option C**: Create a hybrid approach using C bindings

### 2. Configure CSI Collection

```rust
// CSI Configuration Parameters
struct CsiConfig {
    lltf_en: bool,        // Long training field
    htltf_en: bool,       // HT long training field  
    stbc_htltf2_en: bool, // STBC HT-LTF2
    ltf_merge_en: bool,   // LTF merging
    channel_filter_en: bool,
    manu_scale: bool,
}
```

### 3. Serial Protocol Design

Design a simple protocol for TUI ↔ ESP32 communication:

```
Commands (TUI → ESP32):
- START_CSI: Begin CSI collection
- STOP_CSI: Stop collection
- CONFIG_CSI: Set CSI parameters
- GET_STATUS: Query device status
- SET_WIFI_CH: Change WiFi channel

Responses (ESP32 → TUI):
- ACK: Command acknowledged
- CSI_DATA: CSI packet (binary format)
- STATUS: Device status info
- ERROR: Error message
```

### 4. Data Format

Efficient binary format for serial transmission:

```
Header (16 bytes):
  - Magic: 0xC51 (2 bytes)
  - Length: Data length (2 bytes)
  - Timestamp: Unix timestamp (4 bytes)
  - RSSI: Signal strength (1 byte)
  - Rate: Data rate (1 byte)
  - Channel: WiFi channel (1 byte)
  - MAC: Source MAC (6 bytes)
  
CSI Data (variable):
  - Subcarrier count (1 byte)
  - For each subcarrier:
      - I value (2 bytes, signed)
      - Q value (2 bytes, signed)
```

### 5. Current Limitations

⚠️ **Important**: The current `esp-wifi` crate (v0.10.1) has limited CSI support. You may need to:

1. **Check for updates**: `esp-wifi` is actively developed
2. **Use esp-idf**: Switch to std with esp-idf framework (has full CSI support)
3. **Contribute**: Help add CSI support to esp-wifi
4. **Hybrid approach**: Use unsafe FFI to call esp-idf CSI functions

## Alternative: Using ESP-IDF with Rust

For full CSI support, consider using `esp-idf-hal` instead of `esp-hal`:

```toml
[dependencies]
esp-idf-hal = "0.44"
esp-idf-sys = "0.35"
```

This gives you access to the full ESP-IDF API including CSI functions.

## Example CSI Data Structure

```rust
#[repr(C)]
pub struct CsiData {
    pub rx_ctrl: RxCtrl,
    pub mac: [u8; 6],
    pub len: u16,
    pub data: [i8; 384], // 64 subcarriers * 2 (I/Q) * 3 (spatial streams)
}

#[repr(C)]
pub struct RxCtrl {
    pub rssi: i8,
    pub rate: u8,
    pub sig_mode: u8,
    pub mcs: u8,
    pub cwb: u8,
    pub smoothing: u8,
    pub not_sounding: u8,
    pub aggregation: u8,
    pub stbc: u8,
    pub fec_coding: u8,
    pub sgi: u8,
    pub noise_floor: i8,
    pub ampdu_cnt: u8,
    pub channel: u8,
    pub secondary_channel: u8,
    pub timestamp: u32,
    pub ant: u8,
    pub sig_len: u16,
    pub rx_state: u8,
}
```

## Next Steps

1. **Research**: Check latest esp-wifi documentation for CSI support
2. **Prototype**: Build a simple CSI collector using available APIs
3. **Serial Protocol**: Implement command/response handler
4. **Testing**: Verify CSI data quality and collection rate
5. **Optimization**: Tune for maximum throughput

## References

- [ESP-IDF CSI Documentation](https://docs.espressif.com/projects/esp-idf/en/latest/esp32c3/api-reference/network/esp_wifi.html#_CPPv418wifi_csi_config_t)
- [esp-wifi Repository](https://github.com/esp-rs/esp-wifi)
- [CSI Research Papers](https://scholar.google.com/scholar?q=wifi+csi+channel+state+information)
