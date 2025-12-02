# Quick Start Checklist

## 🔌 Step 1: Connect ESP32-C3 (RIGHT NOW!)

```bash
# 1. Plug in your ESP32-C3 via USB

# 2. Find the device
ls /dev/ttyUSB* /dev/ttyACM* 2>/dev/null

# 3. If no devices found, check USB connection
lsusb  # Should see "QinHeng" or "Silicon Labs" or similar

# 4. Fix permissions if needed
sudo usermod -a -G dialout $USER
# Then logout/login OR:
sudo chmod 666 /dev/ttyUSB0  # Use your actual device
```

## 🏗️ Step 2: Build & Flash Current Code

```bash
cd /home/daniah/my-esp32-project

# Build
cargo build --release

# Flash and monitor (adjust port if needed)
espflash flash --monitor target/riscv32imc-unknown-none-elf/release/my-esp32-project

# Or specify port explicitly:
espflash flash --port /dev/ttyUSB0 --monitor target/riscv32imc-unknown-none-elf/release/my-esp32-project
```

**Expected Output**: You should see "Hello world!" messages every 500ms

## 📚 Step 3: Read Documentation

1. **GETTING_STARTED.md** - Comprehensive setup guide
2. **docs/CSI_IMPLEMENTATION.md** - CSI technical details
3. **docs/TUI_GUIDE.md** - How to build the TUI application

## 🎯 What You're Building

```
┌─────────────────────┐
│   ESP32-C3 Firmware │  ← You're here now
│   (Rust no_std)     │
│   - Collect CSI     │
│   - Serial output   │
└──────────┬──────────┘
           │ USB Serial
           │
┌──────────▼──────────┐
│  esp-csi-tui-rs     │  ← Next: Build this
│  (Rust std)         │
│  - Terminal UI      │
│  - Visualization    │
│  - Data storage     │
│  - Rerun streaming  │
└─────────────────────┘
```

## ⚠️ Important Notes

### ESP32-C3 Not Detected?
- **Check cable**: Must support data, not just power
- **Try different USB port**
- **Check drivers**: `sudo apt install brltty && sudo systemctl disable brltty`
- **Hard reset**: Unplug, wait 5 seconds, plug back in

### Build Fails?
```bash
# Clean and rebuild
cargo clean
cargo build --release

# Update toolchain
rustup update
rustup target add riscv32imc-unknown-none-elf
```

### Flash Fails?
```bash
# Hold BOOT button while connecting USB, then:
espflash flash --port /dev/ttyUSB0 target/riscv32imc-unknown-none-elf/release/my-esp32-project

# Or erase first:
espflash erase-flash --port /dev/ttyUSB0
```

## 🚀 Development Path

### Phase 1: ESP32 Firmware (Current)
- [x] Basic WiFi setup ✓
- [ ] Add CSI collection
- [ ] Implement serial protocol
- [ ] Stream CSI data

### Phase 2: TUI Application (Next)
- [ ] Create new Rust project
- [ ] Serial communication
- [ ] Ratatui interface
- [ ] Data visualization
- [ ] Rerun.io integration

## 📦 Dependencies Already Installed

✓ Rust toolchain
✓ espflash
✓ ESP32-C3 target (riscv32imc-unknown-none-elf)

## 🆘 Getting Help

### Check Logs
```bash
# ESP32 logs
RUST_LOG=debug espflash flash --monitor ...

# More verbose
RUST_LOG=trace espflash flash --monitor ...
```

### Common Issues

| Issue | Solution |
|-------|----------|
| No /dev/ttyUSB* | Check USB cable, try different port |
| Permission denied | `sudo chmod 666 /dev/ttyUSB0` |
| Flash timeout | Hold BOOT button, press RESET |
| Build fails | `cargo clean && cargo build --release` |
| Wrong chip | Verify it's ESP32-C3, not C2/S2/S3 |

### Test Commands
```bash
# Verify device
espflash board-info --port /dev/ttyUSB0

# Monitor without flashing
espmonitor /dev/ttyUSB0

# Check build artifacts
ls -lh target/riscv32imc-unknown-none-elf/release/
```

## 🎓 Learning Resources

- **ESP-RS Book**: https://esp-rs.github.io/book/
- **Ratatui Tutorial**: https://ratatui.rs/
- **Rerun Examples**: https://www.rerun.io/docs/getting-started/rust
- **CSI Research**: Google "WiFi CSI sensing"

## 🏁 First Goal

**Get "Hello world!" running on your ESP32-C3**

Once you see that, you've conquered the hardest part! The rest is incremental improvements.

---

**Current Status**: ⏳ Waiting for you to connect ESP32-C3

**Next Action**: Run the build & flash command above ☝️
