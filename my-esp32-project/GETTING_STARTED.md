# Getting Started with ESP32-C3 for WiFi CSI Hackathon

## 🎯 Hackathon Goal
Build `esp-csi-tui-rs` - a terminal user interface application in Rust using Ratatui to visualize and collect Channel State Information (CSI) data from ESP32 devices.

## 📋 Prerequisites

### Hardware
- ESP32-C3 development board
- USB cable (USB-C or micro-USB depending on your board)

### Software Check
Run these commands to verify your setup:

```bash
# Check Rust toolchain
rustc --version
cargo --version

# Check ESP tooling
espflash --version  # If not installed: cargo install espflash
espmonitor --version  # If not installed: cargo install espmonitor

# Check target
rustup target list | grep riscv32imc-unknown-none-elf
```

## 🔌 Step 1: Connect Your ESP32-C3

1. **Plug in your ESP32-C3** via USB to your laptop

2. **Find the device port**:
   ```bash
   # List all serial devices
   ls /dev/ttyUSB* /dev/ttyACM* 2>/dev/null
   
   # Or use this to see more details
   dmesg | grep -i "tty" | tail -20
   ```
   
   Common ports:
   - `/dev/ttyUSB0` or `/dev/ttyUSB1`
   - `/dev/ttyACM0` or `/dev/ttyACM1`

3. **Set permissions** (if needed):
   ```bash
   sudo usermod -a -G dialout $USER
   # Log out and back in for this to take effect
   
   # Or temporarily:
   sudo chmod 666 /dev/ttyUSB0  # Replace with your port
   ```

## 🏗️ Step 2: Build Your Project

```bash
# Build in release mode (recommended)
cargo build --release

# Or use the script
./scripts/build.sh release
```

## 📤 Step 3: Flash to ESP32-C3

### Option A: Using espflash (Recommended)
```bash
espflash flash --monitor target/riscv32imc-unknown-none-elf/release/my-esp32-project

# Or specify port explicitly
espflash flash --port /dev/ttyUSB0 --monitor target/riscv32imc-unknown-none-elf/release/my-esp32-project
```

### Option B: Using web-flash
```bash
./scripts/flash.sh release
```

### Option C: Using cargo-espflash
```bash
cargo install cargo-espflash
cargo espflash flash --release --monitor
```

## 📊 Step 4: Monitor Serial Output

If you didn't use `--monitor` flag:
```bash
espmonitor /dev/ttyUSB0

# Or using screen
screen /dev/ttyUSB0 115200

# Or using minicom
minicom -D /dev/ttyUSB0 -b 115200
```

You should see "Hello world!" messages every 500ms.

## 🎯 Understanding the Hackathon Challenge

### What You Need to Build

**esp-csi-tui-rs** - A Terminal User Interface with:

1. **Device Interaction Module**
   - Serial communication with ESP32-C3
   - CSI configuration interface
   - Command/response protocol

2. **Data Visualization Module** (using Ratatui)
   - Live CSI plotting for subcarriers
   - Multiple plot types: 3D, 2D, Heatmap, Color Domain
   - Real-time updates

3. **Data Streaming Module**
   - Integration with rerun.io viewer
   - Remote streaming capability

4. **Data Storage Module**
   - .rrd format for Rerun playback
   - .csv format for raw data

5. **Bonus: Camera Module**
   - Live camera stream in TUI

### Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    esp-csi-tui-rs                       │
│                  (Your Laptop - Rust)                   │
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │   Ratatui    │  │    Serial    │  │   Rerun.io   │ │
│  │     TUI      │◄─┤ Communication│──┤   Streaming  │ │
│  └──────────────┘  └──────────────┘  └──────────────┘ │
│         │                  ▲                  │        │
│         │                  │                  │        │
│  ┌──────▼──────────────────┴──────────────────▼─────┐ │
│  │          Data Processing & Storage                │ │
│  │         (.csv / .rrd formats)                     │ │
│  └───────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
                           │
                    Serial (USB)
                           │
┌─────────────────────────▼───────────────────────────────┐
│                     ESP32-C3                            │
│                  (Embedded Rust)                        │
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │    WiFi      │  │  CSI Data    │  │    Serial    │ │
│  │   Radio      │──┤  Collection  │──┤     TX       │ │
│  └──────────────┘  └──────────────┘  └──────────────┘ │
└─────────────────────────────────────────────────────────┘
```

## 📝 Next Steps

### Phase 1: ESP32-C3 Firmware (Current Project)
- [x] Basic WiFi initialization (Already done!)
- [ ] Enable CSI collection in esp-wifi
- [ ] Create serial protocol for configuration
- [ ] Stream CSI data over serial

### Phase 2: TUI Application (New Project)
- [ ] Create new Rust project for TUI
- [ ] Implement serial communication
- [ ] Build Ratatui interface
- [ ] Add data visualization
- [ ] Integrate Rerun.io streaming
- [ ] Add data storage

## 🔧 Useful Commands

```bash
# Clean build
cargo clean

# Check code without building
cargo check

# Format code
cargo fmt

# Monitor without flashing
espmonitor /dev/ttyUSB0

# List connected USB devices
lsusb

# Reset ESP32
# Press the BOOT button while pressing RESET, then release RESET
```

## 🐛 Troubleshooting

### ESP32-C3 not detected
- Check USB cable (needs data pins, not just power)
- Try different USB ports
- Check with `lsusb` - should see "QinHeng Electronics" or similar
- Driver issues: `sudo apt install brltty` then `sudo systemctl disable brltty`

### Permission denied
```bash
sudo usermod -a -G dialout $USER
# Then logout and login
```

### Flash fails
- Hold BOOT button while connecting USB
- Try lower baud rate: `espflash flash --baud 115200 ...`
- Erase flash first: `espflash erase-flash`

### No output in monitor
- Check baud rate matches (115200)
- Try pressing RESET button on board
- Verify correct port

## 📚 Resources

- [esp-rs Book](https://esp-rs.github.io/book/)
- [esp-wifi Documentation](https://docs.rs/esp-wifi/)
- [Ratatui Tutorial](https://ratatui.rs/tutorials/)
- [Rerun.io Documentation](https://www.rerun.io/docs)
- [ESP32-C3 Technical Reference](https://www.espressif.com/sites/default/files/documentation/esp32-c3_technical_reference_manual_en.pdf)

## 🎓 Learning Path

1. **Week 1**: Get comfortable with ESP32-C3, build & flash successfully
2. **Week 2**: Understand CSI basics, modify firmware for CSI collection
3. **Week 3**: Build basic TUI with Ratatui, serial communication
4. **Week 4**: Add visualization, streaming, and storage features

Good luck with your hackathon! 🚀
