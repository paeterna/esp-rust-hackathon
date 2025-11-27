# ESP32-C3 WiFi Motion Sensor

A Rust-based WiFi motion detection system using ESP32-C3 and RSSI (Received Signal Strength Indicator) analysis.

## Overview

This project uses WiFi signal strength variations to detect motion and environmental changes. When someone moves near the ESP32-C3 or when objects are displaced, the WiFi signal strength from nearby access points changes, allowing for passive motion detection without cameras or traditional sensors.

## Features

- **WiFi-based Motion Detection**: Detects environmental changes by monitoring RSSI variations
- **Real-time Monitoring**: Scans WiFi networks every second and reports changes
- **Live TUI Visualization**: Beautiful terminal interface showing:
  - Motion detection status with visual gauge
  - WiFi access points with color-coded signal strength
  - Real-time event log
- **Baseline Calibration**: Automatically establishes baseline RSSI values and adapts over time
- **Multi-AP Tracking**: Monitors up to 10 access points simultaneously

## How It Works

### Motion Detection Algorithm

1. **WiFi Scanning**: ESP32-C3 continuously scans for nearby WiFi access points
2. **RSSI Baseline**: Establishes baseline signal strength for each detected AP
3. **Change Detection**: Compares current RSSI with baseline
   - Motion detected when RSSI changes > 4 dBm
4. **Adaptive Baseline**: Uses exponential moving average to adapt to slow environmental changes

### Signal Strength Interpretation

- **Green** (-50 to 0 dBm): Excellent signal
- **Yellow** (-70 to -51 dBm): Good signal
- **Red** (< -70 dBm): Weak signal

## Project Structure

```
esp-rust-hackathon/
├── esp32c3-rust/esp-hacathon/      # ESP32-C3 firmware
│   ├── src/bin/main.rs              # WiFi scanning & motion detection
│   ├── Cargo.toml                   # Dependencies
│   └── flash-no-monitor.sh          # Flash without keeping monitor open
│
└── host-tuis/hello_test/            # Host TUI application
    ├── src/main.rs                  # Serial reader & visualization
    └── Cargo.toml                   # TUI dependencies
```

## Requirements

### Hardware
- ESP32-C3 development board
- USB cable for serial communication

### Software
- Rust toolchain (stable)
- `espflash` for flashing ESP32
- At least 2-3 WiFi access points in range (for better detection)

## Installation

### 1. Install Rust and Tools

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install espflash
cargo install espflash

# Add ESP32 Rust targets
rustup target add riscv32imc-unknown-none-elf
```

### 2. Clone and Build

```bash
cd esp-rust-hackathon

# Build ESP32 firmware
cd esp32c3-rust/esp-hacathon
cargo build --release

# Build TUI (in a separate terminal)
cd ../../host-tuis/hello_test
cargo build
```

## Usage

### Step 1: Flash ESP32-C3

```bash
cd esp32c3-rust/esp-hacathon

# Option A: Flash with monitor (view output immediately)
cargo run --release

# Option B: Flash without monitor (frees serial port for TUI)
./flash-no-monitor.sh
```

### Step 2: Run the TUI

In a separate terminal:

```bash
cd host-tuis/hello_test

# Auto-detect serial port
cargo run

# Or specify port manually
cargo run -- /dev/cu.usbserial-10  # macOS
cargo run -- /dev/ttyUSB0           # Linux
```

### Step 3: Test Motion Detection

1. Let the system calibrate for 5-10 seconds (baseline establishment)
2. Wave your hand near the ESP32-C3
3. Watch the TUI for "MOTION DETECTED" indicator (red gauge)
4. Move objects around or walk near the device to see RSSI changes

## Data Protocol

The ESP32 sends JSON-formatted data via serial (115200 baud):

```json
{
  "counter": 42,
  "motion": 1,
  "aps": [
    {"ssid": "MyWiFi", "rssi": -45, "ch": 6},
    {"ssid": "Neighbor", "rssi": -67, "ch": 11}
  ]
}
```

### Fields:
- `counter`: Scan iteration number
- `motion`: 1 if motion detected, 0 if still
- `aps`: Array of detected access points
  - `ssid`: Network name
  - `rssi`: Signal strength in dBm
  - `ch`: WiFi channel

## Troubleshooting

### "Device or resource busy" Error
The serial port is still open from flashing. Press `Ctrl+C` in the flash terminal, then restart the TUI.

### No Motion Detection
- Ensure at least 2-3 WiFi APs are in range
- Wait 5-10 seconds for baseline calibration
- Try moving closer to the ESP32-C3
- Check that motion creates RSSI change > 4 dBm

### Serial Port Not Found
```bash
# List available ports
ls /dev/cu.* /dev/tty.usbserial-* 2>/dev/null  # macOS
ls /dev/ttyUSB* 2>/dev/null                     # Linux
```

## Advanced: True CSI (Future Enhancement)

While this implementation uses RSSI for motion detection, true WiFi CSI (Channel State Information) provides much richer data including:
- Amplitude and phase for each subcarrier
- Multi-path signal analysis
- Higher sensitivity to environmental changes

CSI requires:
- Direct ESP-IDF WiFi driver integration
- Custom FFI bindings
- Signal processing libraries (FFT, filtering)

This RSSI-based approach provides a practical stepping stone to full CSI implementation.

## Performance

- **Scan Rate**: 1 Hz (1 scan per second)
- **Motion Detection Latency**: ~1 second
- **Baseline Adaptation**: 90% retention, 10% new data
- **Serial Baud Rate**: 115,200 bps
- **AP Tracking**: Up to 10 access points

## Tips for Best Results

1. **Environment**: Works best in areas with multiple WiFi networks
2. **Placement**: Position ESP32-C3 where people/objects will pass nearby
3. **Calibration**: Let system run for 10-15 seconds before testing
4. **Sensitivity**: Adjust RSSI threshold in code (currently 4 dBm) for your environment

## Contributing

This is a hackathon project exploring WiFi sensing capabilities. Feel free to:
- Experiment with detection algorithms
- Add visualization features
- Implement true CSI support
- Create object mapping/tracking

## License

MIT

## Acknowledgments

- Built with `esp-hal` and `esp-radio` for ESP32-C3 support
- TUI powered by `ratatui`
- Inspired by WiFi sensing research and CSI-based human activity recognition
