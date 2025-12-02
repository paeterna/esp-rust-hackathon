# ESP32-C3 WiFi CSI Capture System - Complete Guide

## Table of Contents
1. [Overview](#overview)
2. [System Architecture](#system-architecture)
3. [Hardware Setup](#hardware-setup)
4. [Software Setup](#software-setup)
5. [Building & Flashing](#building--flashing)
6. [Running the System](#running-the-system)
7. [Understanding the Data](#understanding-the-data)
8. [Troubleshooting](#troubleshooting)
9. [Development Workflow](#development-workflow)
10. [File Structure](#file-structure)

---

## Overview

This project captures **WiFi Channel State Information (CSI)** from an ESP32-C3 microcontroller in real-time and visualizes it using a Terminal User Interface (TUI).

### What is CSI?
- **Channel State Information** describes how WiFi signals propagate through space
- Contains 64 complex subcarriers (I/Q values) representing frequency-selective fading
- Useful for indoor localization, human activity recognition, gesture detection, etc.
- Each subcarrier shows amplitude and phase at different frequencies

### Key Features
✅ Real-time CSI capture from ESP32-C3 hardware
✅ Promiscuous mode WiFi packet capture
✅ Binary protocol over UART (115200 baud)
✅ Terminal visualization with amplitude, phase, and heatmap views
✅ Auto-reconnect when device is unplugged/replugged
✅ Persistent device naming with udev rules

---

## System Architecture

```
┌─────────────────┐         UART          ┌──────────────────┐
│   ESP32-C3      │    (115200 baud)      │   Linux Host     │
│                 │◄─────────────────────►│                  │
│ - WiFi Radio    │   Binary Protocol     │ - TUI Display    │
│ - CSI Capture   │   (Magic + CRC-8)     │ - Visualization  │
│ - Promiscuous   │                       │ - CSV Export     │
└─────────────────┘                       └──────────────────┘
```

### Components

1. **ESP32-C3 Firmware** (`/my-esp32-project/`)
   - Written in Rust using ESP-IDF framework
   - Captures CSI from WiFi packets in promiscuous mode
   - Transmits data via UART in binary protocol

2. **TUI Viewer** (`/esp-csi-tui-rs/`)
   - Terminal-based visualization
   - Parses binary protocol
   - Real-time graphs and statistics

3. **Auto-Reconnect Script** (`scripts/run_csi_viewer.sh`)
   - Monitors device connection
   - Automatically restarts TUI on reconnect

---

## Hardware Setup

### Required Hardware
- **ESP32-C3** development board (any variant)
- **USB cable** (for programming and data transmission)
- **Linux computer** (tested on Ubuntu/Debian)

### ESP32-C3 Pinout
The firmware uses the USB-Serial bridge built into the ESP32-C3:
- **GPIO21**: TX (UART0) - Transmits CSI data
- **GPIO20**: RX (UART0) - Not used but configured
- **USB**: Powers the board and provides serial connection

### Physical Connection
1. Connect ESP32-C3 to computer via USB
2. Device appears as `/dev/ttyUSB*` or `/dev/ttyACM*`
3. After installing udev rules, appears as `/dev/esp32-csi`

---

## Software Setup

### Prerequisites

#### 1. Rust Toolchain
```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install nightly toolchain
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly

# Set nightly as default (or use rust-toolchain.toml)
rustup default nightly
```

#### 2. ESP32 Tools
```bash
# Install espflash (for programming ESP32)
cargo install espflash

# Install ldproxy (linker proxy for ESP-IDF)
cargo install ldproxy
```

#### 3. System Dependencies (Ubuntu/Debian)
```bash
sudo apt update
sudo apt install -y git curl gcc clang cmake ninja-build \
    libudev-dev pkg-config libssl-dev python3 python3-pip
```

#### 4. User Permissions
```bash
# Add your user to dialout group for serial port access
sudo usermod -aG dialout $USER

# Log out and back in for changes to take effect
```

### Repository Setup

```bash
# Clone the repository
cd /home/daniah/esp-rust-hackathon
git checkout working-esp

# The repository contains two projects:
# 1. my-esp32-project/    - ESP32 firmware
# 2. esp-csi-tui-rs/      - TUI viewer
```

### Install udev Rules (Persistent Device Naming)

```bash
cd /home/daniah/esp-rust-hackathon/my-esp32-project
./scripts/install_udev_rule.sh

# Unplug and replug your ESP32
# Device will now appear as /dev/esp32-csi
```

This creates a symlink so your ESP32 always appears at `/dev/esp32-csi` regardless of whether it's on `ttyUSB0`, `ttyUSB1`, etc.

---

## Building & Flashing

### Build the Firmware

```bash
cd /home/daniah/esp-rust-hackathon/my-esp32-project

# Build for release (optimized, smaller binary)
cargo build --release --target riscv32imc-esp-espidf

# Or use the build script
./scripts/build.sh release
```

**First build takes 10-20 minutes** as it:
- Downloads ESP-IDF (~1GB)
- Compiles all dependencies
- Builds the firmware

Subsequent builds are much faster (~30 seconds).

### Flash to ESP32

```bash
# Flash the firmware
espflash flash target/riscv32imc-esp-espidf/release/my-esp32-project

# Or use the flash script
./scripts/flash.sh release

# Flash with serial monitor (see boot logs)
espflash flash target/riscv32imc-esp-espidf/release/my-esp32-project --monitor
```

**What happens during flash:**
1. Connects to ESP32 via USB serial
2. Resets the device into bootloader mode
3. Uploads firmware (~963KB)
4. Verifies the upload
5. Resets the device to run new firmware

### Build the TUI (Optional)

The TUI can run without building if you use `cargo run`:

```bash
cd /home/daniah/esp-rust-hackathon/esp-csi-tui-rs

# Run directly (compiles automatically)
cargo run --release -- --port /dev/esp32-csi

# Or build explicitly
cargo build --release
./target/release/esp-csi-tui-rs --port /dev/esp32-csi
```

---

## Running the System

### Method 1: Auto-Reconnect Script (Recommended)

```bash
cd /home/daniah/esp-rust-hackathon/my-esp32-project
./scripts/run_csi_viewer.sh
```

**What it does:**
- Detects ESP32 on any serial port (`/dev/esp32-csi`, `/dev/ttyUSB*`, etc.)
- Launches the TUI automatically
- Waits if device is disconnected
- Restarts TUI when device reconnects
- Press `Ctrl+C` to exit

### Method 2: Manual TUI Launch

```bash
cd /home/daniah/esp-rust-hackathon/esp-csi-tui-rs
cargo run --release -- --port /dev/esp32-csi
```

### Method 3: Using Different Ports

```bash
# If your device is on a different port
cargo run --release -- --port /dev/ttyUSB0

# Or with specific baud rate (default is 115200)
cargo run --release -- --port /dev/esp32-csi --baud 115200

# Mock data mode (no hardware needed)
cargo run --release -- --mock

# Save data to CSV
cargo run --release -- --port /dev/esp32-csi --csv output.csv
```

### TUI Controls

Once the TUI is running:
- **[A]** - Amplitude view (default)
- **[P]** - Phase view
- **[H]** - Heatmap view (CSI over time)
- **[S]** - Status view (connection info)
- **[Q]** - Quit

---

## Understanding the Data

### Binary Protocol Format

The ESP32 transmits CSI data using a custom binary protocol:

```
┌────────────┬──────────┬────────────┬─────────┬────────┐
│  Magic     │  Type    │  Length    │ Payload │  CRC   │
│  (2 bytes) │ (1 byte) │ (2 bytes)  │ (N)     │ (1 byte)│
└────────────┴──────────┴────────────┴─────────┴────────┘
```

#### Frame Structure
- **Magic Header**: `[0xC5, 0x1A]` - Identifies start of frame
- **Message Type**: `0x90` - CSI data packet
- **Length**: 2 bytes (little-endian) - Payload size
- **Payload**: Variable length
- **CRC-8**: 1 byte - Error detection (polynomial 0x07)

#### Payload Structure (279 bytes)
```
Offset | Size | Field           | Description
-------|------|-----------------|----------------------------------
0      | 4    | Timestamp       | Milliseconds since boot
4      | 1    | RSSI            | Signal strength (dBm)
5      | 1    | Rate            | WiFi data rate
6      | 1    | Channel         | WiFi channel number
7      | 6    | MAC Address     | Source MAC address
13     | 2    | Num Subcarriers | Number of CSI subcarriers (64)
15     | 256  | CSI Data        | 64 × (I:2 bytes + Q:2 bytes)
271    | 1    | CRC-8           | Checksum
```

#### CSI Data Format
Each subcarrier contains:
- **I (In-phase)**: 2 bytes (signed, little-endian)
- **Q (Quadrature)**: 2 bytes (signed, little-endian)

Calculate amplitude: `sqrt(I² + Q²)`
Calculate phase: `atan2(Q, I)`

### Understanding CSI Patterns

#### Amplitude View
- **Y-axis**: Signal strength (arbitrary units)
- **X-axis**: Subcarrier index (0-63)
- **Pattern**: Shows frequency-selective fading
  - Peaks indicate constructive interference
  - Valleys indicate destructive interference
  - Pattern changes with environment and movement

#### Phase View
- **Y-axis**: Phase angle (-π to +π radians)
- **X-axis**: Subcarrier index (0-63)
- **Pattern**: Phase shifts indicate propagation delay
  - Linear phase = direct path
  - Non-linear phase = multipath propagation

#### Heatmap View
- **Y-axis**: Time (most recent at top)
- **X-axis**: Subcarrier index (0-63)
- **Color**: Amplitude (brighter = stronger)
- **Pattern**: Shows CSI changes over time
  - Horizontal bands = stable environment
  - Vertical bands = frequency-selective fading
  - Diagonal patterns = moving objects

### Real-World CSI Values

Typical values you'll see:
- **RSSI**: -30 to -80 dBm (closer = stronger)
- **Amplitude**: 0 to ~500 (depends on signal strength)
- **Phase**: -180° to +180°
- **Update Rate**: 10-100 Hz (depends on WiFi traffic)

---

## Troubleshooting

### Build Issues

#### "error: no targets specified in the manifest"
**Cause**: Running cargo from wrong directory
**Solution**: 
```bash
# Make sure you're in the correct project directory
cd /home/daniah/esp-rust-hackathon/my-esp32-project  # For firmware
cd /home/daniah/esp-rust-hackathon/esp-csi-tui-rs    # For TUI
```

#### "error: failed to run custom build command for `esp-idf-sys`"
**Cause**: Missing ESP-IDF or build dependencies
**Solution**:
```bash
# Install required packages
sudo apt install git cmake ninja-build python3

# Clean and rebuild
cargo clean
cargo build --release --target riscv32imc-esp-espidf
```

#### "rust-src component not found"
**Solution**:
```bash
rustup component add rust-src --toolchain nightly
```

### Flash Issues

#### "error: espflash::timeout"
**Cause**: Can't connect to ESP32
**Solution**:
1. Check USB cable is connected
2. Try pressing BOOT button while connecting
3. Check device exists: `ls -l /dev/ttyUSB*`
4. Check permissions: `groups | grep dialout`

#### "Permission denied" on /dev/ttyUSB0
**Solution**:
```bash
sudo usermod -aG dialout $USER
# Log out and back in
```

### Runtime Issues

#### "CRC mismatch" errors in TUI
**Cause**: Protocol version mismatch
**Solution**: Rebuild and reflash both firmware and TUI

#### "Waiting for device..." never finds ESP32
**Solution**:
```bash
# Check device exists
ls -l /dev/ttyUSB* /dev/esp32-csi

# Check if udev rule is installed
ls -l /etc/udev/rules.d/99-esp32-csi.rules

# Reinstall udev rule
cd /home/daniah/esp-rust-hackathon/my-esp32-project
./scripts/install_udev_rule.sh
```

#### TUI shows "waiting for samples" but no data
**Cause**: No WiFi traffic to capture
**Solution**: 
- Move near WiFi access points
- Generate WiFi traffic (browse internet, stream video)
- Check ESP32 is in promiscuous mode (should capture all packets)

#### Device switches between /dev/ttyUSB0 and /dev/ttyUSB1
**Cause**: Linux dynamic device numbering
**Solution**: Install udev rule for persistent `/dev/esp32-csi` name

---

## Development Workflow

### Modify Firmware

```bash
cd /home/daniah/esp-rust-hackathon/my-esp32-project

# Edit source code
vim src/main.rs

# Build and flash
cargo build --release --target riscv32imc-esp-espidf
espflash flash target/riscv32imc-esp-espidf/release/my-esp32-project

# Or use script
./scripts/flash.sh release
```

### Modify TUI

```bash
cd /home/daniah/esp-rust-hackathon/esp-csi-tui-rs

# Edit source code
vim src/main.rs

# Run (automatically rebuilds)
cargo run --release -- --port /dev/esp32-csi
```

### View Serial Monitor

```bash
# See ESP32 debug output (if logging is enabled)
espflash monitor

# Or during flash
espflash flash target/riscv32imc-esp-espidf/release/my-esp32-project --monitor
```

### Testing Changes

1. **Firmware changes**: Flash to ESP32, restart TUI
2. **Protocol changes**: Update both firmware and TUI, rebuild both
3. **TUI changes**: Just restart TUI (firmware doesn't need reflash)

### Git Workflow

```bash
# Make changes
git add .
git commit -m "Description of changes"

# Push to GitHub
git push origin working-esp

# Pull latest changes
git pull origin working-esp
```

---

## File Structure

### ESP32 Firmware (`/my-esp32-project/`)

```
my-esp32-project/
├── src/
│   ├── main.rs                    # Main firmware code
│   ├── main_csi_template.rs       # Template/backup
│   └── main_protocol.rs.backup    # Protocol backup
├── scripts/
│   ├── build.sh                   # Build script
│   ├── flash.sh                   # Flash script
│   ├── run_csi_viewer.sh          # Auto-reconnect launcher
│   ├── install_udev_rule.sh       # udev rule installer
│   └── 99-esp32-csi.rules         # udev rule file
├── .cargo/
│   └── config.toml                # Cargo configuration
├── Cargo.toml                     # Project dependencies
├── build.rs                       # Build script
├── memory.x                       # Memory layout
├── rust-toolchain.toml            # Rust version config
├── sdkconfig.defaults             # ESP-IDF configuration
└── README.md                      # Project documentation
```

### TUI Viewer (`/esp-csi-tui-rs/`)

```
esp-csi-tui-rs/
├── src/
│   ├── main.rs                    # TUI application
│   └── protocol.rs                # Binary protocol parser
├── Cargo.toml                     # Project dependencies
└── README.md                      # TUI documentation
```

### Key Files Explained

#### `src/main.rs` (Firmware)
- `csi_rx_callback()`: Called when WiFi packet received
- `send_csi_data()`: Formats and transmits CSI over UART
- `calculate_crc8()`: CRC-8 checksum calculation
- `main()`: Initializes WiFi, UART, and CSI capture

#### `src/protocol.rs` (TUI)
- `ProtocolHandler`: Manages serial communication
- `try_parse_frame()`: Parses binary protocol
- `calculate_crc8()`: CRC-8 validation
- `CsiDataPacket`: CSI data structure

#### `scripts/run_csi_viewer.sh`
- Auto-detects serial port
- Launches TUI from correct directory
- Restarts on disconnect/reconnect

#### `scripts/99-esp32-csi.rules`
- udev rule for persistent device naming
- Creates `/dev/esp32-csi` symlink
- Based on USB vendor/product ID

---

## Advanced Topics

### Changing CSI Configuration

Edit `src/main.rs` in the firmware:

```rust
let mut csi_config = esp_idf_sys::wifi_csi_config_t {
    lltf_en: true,           // Long Training Field
    htltf_en: true,          // HT Long Training Field
    stbc_htltf2_en: true,    // STBC HT-LTF2
    ltf_merge_en: true,      // LTF merging
    channel_filter_en: false, // Channel filter
    manu_scale: true,        // Manual scaling
    shift: 0,                // Scaling shift
    dump_ack_en: true,       // Dump ACK frames
};
```

### Changing WiFi Channel

Add to firmware `main()` function:

```rust
unsafe {
    // Set WiFi channel (1-13 for 2.4GHz)
    esp_idf_sys::esp_wifi_set_channel(6, esp_idf_sys::wifi_second_chan_t_WIFI_SECOND_CHAN_NONE);
}
```

### Exporting Data

```bash
# Save CSI data to CSV
cargo run --release -- --port /dev/esp32-csi --csv output.csv

# Format: timestamp, rssi, subcarrier_0_amp, subcarrier_0_phase, ...
```

### Performance Tuning

- **UART Speed**: Change in firmware and TUI (currently 115200)
- **Update Rate**: Depends on WiFi traffic in environment
- **Buffer Size**: Increase for high-traffic environments

---

## Summary

### Quick Start Checklist

- [ ] Install Rust nightly and ESP tools
- [ ] Clone repository and checkout `working-esp` branch
- [ ] Install udev rule with `./scripts/install_udev_rule.sh`
- [ ] Build firmware: `cargo build --release --target riscv32imc-esp-espidf`
- [ ] Flash firmware: `espflash flash target/...`
- [ ] Run auto-reconnect script: `./scripts/run_csi_viewer.sh`
- [ ] See real-time CSI data in TUI!

### Common Commands

```bash
# Build firmware
cd /home/daniah/esp-rust-hackathon/my-esp32-project
cargo build --release --target riscv32imc-esp-espidf

# Flash firmware
espflash flash target/riscv32imc-esp-espidf/release/my-esp32-project

# Run TUI with auto-reconnect
./scripts/run_csi_viewer.sh

# Run TUI manually
cd /home/daniah/esp-rust-hackathon/esp-csi-tui-rs
cargo run --release -- --port /dev/esp32-csi
```

### Getting Help

- Check `GETTING_STARTED.md` for setup instructions
- Check `docs/` directory for detailed documentation
- Review `STATUS.md` for known issues
- Check GitHub issues for common problems

---

**Project Repository**: https://github.com/paeterna/esp-rust-hackathon
**Branch**: `working-esp`
**Last Updated**: December 2, 2025
