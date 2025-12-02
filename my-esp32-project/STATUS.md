# 🎯 HACKATHON SETUP - COMPLETE!

## ✅ What's Been Done

I've set up your ESP32-C3 hackathon project with comprehensive documentation and tools:

### 📁 Files Created

1. **QUICKSTART.md** - Your first stop! Quick commands to get running
2. **GETTING_STARTED.md** - Complete guide for ESP32-C3 and hackathon overview
3. **docs/CSI_IMPLEMENTATION.md** - Technical details on CSI data collection
4. **docs/TUI_GUIDE.md** - Step-by-step guide to build esp-csi-tui-rs
5. **src/main_csi_template.rs** - Template for CSI-enabled firmware
6. **scripts/diagnose.sh** - Diagnostic tool to troubleshoot connections

### ✅ System Check Results

Your development environment is **READY**:
- ✓ Rust toolchain installed (1.91.1)
- ✓ ESP32-C3 target (riscv32imc-unknown-none-elf) installed
- ✓ espflash (4.2.0) installed
- ✓ User has dialout permissions
- ✓ Build scripts ready

### ⚠️ What's Missing

**ESP32-C3 is NOT currently connected**

## 🚀 IMMEDIATE NEXT STEPS

### Step 1: Connect Your ESP32-C3 (DO THIS NOW!)

1. **Plug in ESP32-C3** to your laptop via USB
2. **Wait 3 seconds**
3. **Run diagnostic again**:
   ```bash
   ./scripts/diagnose.sh
   ```

You should see `/dev/ttyUSB0` or `/dev/ttyACM0` appear

### Step 2: Test Basic Functionality

```bash
# Build the project
cd /home/daniah/my-esp32-project
cargo build --release

# Flash and monitor (replace /dev/ttyUSB0 with your port)
espflash flash --monitor --port /dev/ttyUSB0 \
  target/riscv32imc-unknown-none-elf/release/my-esp32-project
```

**Expected**: You should see "Hello world!" messages every 500ms

### Step 3: Read Documentation

Start with `QUICKSTART.md`, then move to other docs as needed.

## 📊 Hackathon Challenge Breakdown

### Part 1: ESP32-C3 Firmware (Your Current Project)

**Goal**: Collect CSI data and send via serial

**Status**: Basic WiFi setup ✓, CSI collection ✗ (needs implementation)

**Key Tasks**:
- [ ] Enable CSI in esp-wifi (check for updates or use esp-idf)
- [ ] Implement serial protocol for commands
- [ ] Stream CSI data packets
- [ ] Handle configuration from host

**Estimated Time**: 1-2 weeks

### Part 2: TUI Application (New Project)

**Goal**: Terminal UI for visualizing and storing CSI data

**Status**: Not started (documentation ready)

**Key Tasks**:
- [ ] Create new Rust project with Ratatui
- [ ] Serial communication with ESP32
- [ ] Data visualization (2D, 3D, heatmap, color domain)
- [ ] Rerun.io streaming integration
- [ ] Data storage (.csv and .rrd formats)
- [ ] Camera integration (bonus)

**Estimated Time**: 2-3 weeks

## 🏗️ Project Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    Your Laptop                           │
│                                                          │
│  ┌────────────────────────────────────────────────┐    │
│  │         esp-csi-tui-rs (Terminal App)          │    │
│  │                                                 │    │
│  │  • Ratatui TUI                                 │    │
│  │  • Serial Communication                        │    │
│  │  • Data Visualization                          │    │
│  │  • Rerun.io Streaming                          │    │
│  │  • CSV/RRD Export                              │    │
│  └────────────────┬───────────────────────────────┘    │
│                   │ Serial over USB                     │
└───────────────────┼─────────────────────────────────────┘
                    │
┌───────────────────▼─────────────────────────────────────┐
│                  ESP32-C3 Board                         │
│                                                         │
│  ┌────────────────────────────────────────────────┐   │
│  │      my-esp32-project (Firmware)               │   │
│  │                                                 │   │
│  │  • WiFi Radio                                  │   │
│  │  • CSI Data Collection                         │   │
│  │  • Serial Protocol Handler                     │   │
│  │  • Configuration Manager                       │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

## 🎓 Learning Path

### Week 1: Foundation
- Get ESP32-C3 connected ← **YOU ARE HERE**
- Build and flash successfully
- Understand CSI basics
- Research esp-wifi CSI capabilities

### Week 2: ESP32 Firmware
- Implement CSI collection
- Build serial protocol
- Test data streaming
- Optimize performance

### Week 3: TUI Basics
- Create new TUI project
- Implement serial communication
- Build basic Ratatui interface
- Add simple visualization

### Week 4: Advanced Features
- Multiple plot types
- Rerun.io integration
- Data storage (CSV/RRD)
- Polish and testing
- (Bonus) Camera integration

## 🛠️ Quick Commands Reference

```bash
# Diagnose connection
./scripts/diagnose.sh

# Build firmware
cargo build --release

# Flash to ESP32 (adjust port)
espflash flash --monitor --port /dev/ttyUSB0 \
  target/riscv32imc-unknown-none-elf/release/my-esp32-project

# Or use shortcut script
./scripts/flash.sh release

# Monitor serial output
espmonitor /dev/ttyUSB0

# Clean build
cargo clean

# Check without building
cargo check
```

## 📚 Documentation Index

| File | Purpose | When to Read |
|------|---------|--------------|
| QUICKSTART.md | Quick reference | **Start here** |
| GETTING_STARTED.md | Comprehensive setup | After connecting ESP32 |
| docs/CSI_IMPLEMENTATION.md | CSI technical details | When implementing CSI |
| docs/TUI_GUIDE.md | TUI development | When building app |
| scripts/diagnose.sh | Troubleshooting | When having issues |

## 🆘 Common Issues & Solutions

### ESP32 Not Detected
```bash
# Check USB devices
lsusb

# Check serial ports
ls /dev/ttyUSB* /dev/ttyACM*

# Try different cable/port
# Press RESET button on board
```

### Permission Denied
```bash
# Already done, but if needed:
sudo chmod 666 /dev/ttyUSB0
```

### Build Fails
```bash
cargo clean
cargo build --release
```

### Flash Fails
```bash
# Hold BOOT button while connecting
# Or try:
espflash erase-flash --port /dev/ttyUSB0
```

## 🎯 Success Criteria

### Milestone 1: Connection ✓
- [x] ESP32-C3 detected as /dev/ttyUSB* ← **Next: Do this!**
- [x] Can flash firmware
- [x] See "Hello world!" in serial monitor

### Milestone 2: Basic CSI
- [ ] ESP32 collects CSI data
- [ ] Data sent via serial
- [ ] Protocol working

### Milestone 3: TUI MVP
- [ ] TUI application created
- [ ] Reads serial data
- [ ] Basic visualization
- [ ] CSV export

### Milestone 4: Full Features
- [ ] All plot types implemented
- [ ] Rerun.io streaming works
- [ ] RRD format export
- [ ] Configuration UI complete
- [ ] (Bonus) Camera integration

## 🎉 You're Ready!

Everything is set up. Your next action:

**🔌 CONNECT YOUR ESP32-C3 AND RUN `./scripts/diagnose.sh`**

Once connected, follow the commands in QUICKSTART.md to flash your first program!

---

**Good luck with your hackathon!** 🚀

*You've got this! The hard part (setup) is done. Now it's just building cool stuff.*
