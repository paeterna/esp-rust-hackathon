# ESP32-C3 WiFi CSI Hackathon Project

**Hackathon Goal**: Build `esp-csi-tui-rs` - A terminal user interface for collecting and visualizing WiFi Channel State Information (CSI) from ESP32 devices using Rust.

## 🚀 Quick Start

**NEW TO THIS PROJECT? START HERE!**

1. **Connect your ESP32-C3** via USB to your laptop
2. **Run diagnostics**: `./scripts/diagnose.sh`
3. **Read**: `QUICKSTART.md` for immediate next steps
4. **Read**: `STATUS.md` for complete project overview

## 📚 Documentation Guide

| File | When to Read | Purpose |
|------|--------------|---------|
| **STATUS.md** | 👈 **Start here!** | Complete project status, what's done, what's next |
| **QUICKSTART.md** | When connecting ESP32 | Quick commands & troubleshooting |
| **GETTING_STARTED.md** | After first success | Comprehensive setup & hackathon details |
| **docs/CSI_IMPLEMENTATION.md** | When coding CSI | Technical CSI implementation guide |
| **docs/SERIAL_PROTOCOL.md** | When coding protocol | Serial communication specification |
| **docs/TUI_GUIDE.md** | When building TUI | Step-by-step TUI development |
| **docs/README.md** | Reference | Original project template docs |

## 🛠️ Quick Commands

```bash
# Check if ESP32-C3 is connected
./scripts/diagnose.sh

# Build firmware
cargo build --release

# Flash to ESP32-C3 (replace /dev/ttyUSB0 with your port)
espflash flash --monitor --port /dev/ttyUSB0 \
  target/riscv32imc-unknown-none-elf/release/my-esp32-project

# Or use the script
./scripts/flash.sh release
```

## 🎯 What You're Building

### Two-Part System

```
┌─────────────────────────────────┐
│   ESP32-C3 Firmware (Part 1)    │ ← You are here
│   ─────────────────────────      │
│   • Collect WiFi CSI data        │
│   • Serial protocol handler      │
│   • Stream data to laptop        │
└────────────┬────────────────────┘
             │ USB Serial
┌────────────▼────────────────────┐
│   esp-csi-tui-rs (Part 2)       │ ← Build next
│   ─────────────────────          │
│   • Terminal UI (Ratatui)       │
│   • Data visualization           │
│   • Rerun.io streaming           │
│   • CSV/RRD export               │
│   • (Bonus) Camera integration   │
└─────────────────────────────────┘
```

### Part 1: ESP32-C3 Firmware (This Repository)
**Technology**: Embedded Rust (no_std)  
**Features**:
- WiFi CSI data collection
- Serial communication protocol
- Real-time data streaming
- Configuration management

**Status**: Basic WiFi setup ✓, CSI collection pending

### Part 2: TUI Application (Next Project)
**Technology**: Desktop Rust with Ratatui  
**Features**:
- Terminal-based user interface
- Multiple visualization modes (2D, 3D, heatmap, color domain)
- Remote streaming to Rerun.io viewer
- Data export (CSV and RRD formats)
- Device configuration interface
- Optional camera integration

**Status**: Not started (comprehensive guide ready in `docs/TUI_GUIDE.md`)

## ✅ System Check

Run `./scripts/diagnose.sh` to verify:
- ✓ Rust toolchain installed
- ✓ espflash available
- ✓ ESP32-C3 target configured
- ✓ User permissions correct
- ⚠️ ESP32-C3 connected (needs your action!)

## 📁 Project Structure

```
my-esp32-project/
├── src/
│   ├── main.rs                    # Current basic firmware
│   └── main_csi_template.rs       # Template for CSI features
├── scripts/
│   ├── build.sh                   # Build script
│   ├── flash.sh                   # Flash script
│   └── diagnose.sh                # Connection diagnostics ⭐
├── docs/
│   ├── README.md                  # Original template docs
│   ├── CSI_IMPLEMENTATION.md      # CSI technical guide
│   └── TUI_GUIDE.md               # TUI development guide
├── STATUS.md                      # Project status & overview ⭐
├── QUICKSTART.md                  # Quick reference ⭐
├── GETTING_STARTED.md             # Comprehensive guide ⭐
├── Cargo.toml                     # Rust dependencies
└── README.md                      # This file
```

## 🎓 Learning Path

### Week 1: Foundation (Current)
- [x] Project setup
- [ ] Connect ESP32-C3 ← **Do this now!**
- [ ] First successful flash
- [ ] Understand CSI basics

### Week 2: ESP32 Development
- [ ] Implement CSI collection
- [ ] Build serial protocol
- [ ] Test data streaming
- [ ] Optimize performance

### Week 3: TUI Basics
- [ ] Create TUI project
- [ ] Serial communication
- [ ] Basic Ratatui interface
- [ ] Simple visualization

### Week 4: Advanced Features
- [ ] All visualization modes
- [ ] Rerun.io integration
- [ ] Data storage (CSV/RRD)
- [ ] Polish & testing
- [ ] Bonus: Camera feature

## 🆘 Troubleshooting

### ESP32 Not Detected
```bash
# Run diagnostics first
./scripts/diagnose.sh

# Check USB connection
lsusb

# Try different cable/port
# Press RESET button on ESP32
```

### Can't Flash
```bash
# Hold BOOT button while connecting, or:
espflash erase-flash --port /dev/ttyUSB0

# Then try again
espflash flash --port /dev/ttyUSB0 ...
```

### Build Issues
```bash
cargo clean
cargo build --release
```

## 📞 Resources

- **ESP-RS Book**: https://esp-rs.github.io/book/
- **esp-wifi Docs**: https://docs.rs/esp-wifi/
- **Ratatui**: https://ratatui.rs/
- **Rerun.io**: https://www.rerun.io/docs
- **CSI Research**: Search "WiFi CSI sensing" on Google Scholar

## 🎉 Ready to Start?

**Your Next Action**:

1. 🔌 **Plug in your ESP32-C3**
2. ⚙️ **Run**: `./scripts/diagnose.sh`
3. 📖 **Read**: `STATUS.md` for complete overview
4. 🚀 **Flash**: Follow commands in `QUICKSTART.md`

Good luck with your hackathon! 🚀

---

*Setup completed by GitHub Copilot - All documentation and scripts are ready to use!*
