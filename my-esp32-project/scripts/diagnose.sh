#!/bin/bash

# ESP32-C3 Connection Diagnostics Script
# Run this to diagnose connection issues

echo "═══════════════════════════════════════════════"
echo "  ESP32-C3 Connection Diagnostics"
echo "═══════════════════════════════════════════════"
echo ""

# Color codes
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check 1: USB devices
echo "1. Checking USB devices..."
if lsusb | grep -i "qinheng\|ch340\|cp210\|ftdi\|silicon labs"; then
    echo -e "${GREEN}✓${NC} USB device found:"
    lsusb | grep -i "qinheng\|ch340\|cp210\|ftdi\|silicon labs"
else
    echo -e "${RED}✗${NC} No ESP32 USB device found"
    echo "   → Check USB cable (must support data transfer)"
    echo "   → Try different USB ports"
    echo "   → Press RESET button on ESP32"
fi
echo ""

# Check 2: Serial ports
echo "2. Checking serial ports..."
if ls /dev/ttyUSB* /dev/ttyACM* 2>/dev/null; then
    echo -e "${GREEN}✓${NC} Serial ports found:"
    ls -l /dev/ttyUSB* /dev/ttyACM* 2>/dev/null
else
    echo -e "${RED}✗${NC} No serial ports found"
    echo "   → ESP32 may not be connected properly"
    echo "   → Check dmesg: dmesg | tail -30"
fi
echo ""

# Check 3: Permissions
echo "3. Checking permissions..."
if groups | grep -q dialout; then
    echo -e "${GREEN}✓${NC} User is in dialout group"
else
    echo -e "${YELLOW}!${NC} User NOT in dialout group"
    echo "   → Run: sudo usermod -a -G dialout \$USER"
    echo "   → Then logout and login"
fi
echo ""

# Check 4: Rust toolchain
echo "4. Checking Rust toolchain..."
if command -v rustc &> /dev/null; then
    echo -e "${GREEN}✓${NC} Rust installed: $(rustc --version)"
else
    echo -e "${RED}✗${NC} Rust not found"
    echo "   → Install from https://rustup.rs"
fi
echo ""

# Check 5: ESP32 target
echo "5. Checking ESP32-C3 target..."
if rustup target list --installed | grep -q "riscv32imc-unknown-none-elf"; then
    echo -e "${GREEN}✓${NC} riscv32imc-unknown-none-elf target installed"
else
    echo -e "${RED}✗${NC} ESP32-C3 target not installed"
    echo "   → Run: rustup target add riscv32imc-unknown-none-elf"
fi
echo ""

# Check 6: espflash
echo "6. Checking espflash..."
if command -v espflash &> /dev/null; then
    echo -e "${GREEN}✓${NC} espflash installed: $(espflash --version)"
else
    echo -e "${RED}✗${NC} espflash not found"
    echo "   → Run: cargo install espflash"
fi
echo ""

# Check 7: Try to detect ESP32
ESP32_DETECTED=04
echo "7. Attempting to detect ESP32-C3..."
if command -v espflash &> /dev/null; then
    for port in /dev/ttyUSB* /dev/ttyACM*; do
        if [ -e "$port" ]; then
            echo "   Trying $port..."
            if timeout 3s espflash board-info --port "$port" 2>/dev/null; then
                echo -e "${GREEN}✓${NC} ESP32 detected on $port"
                ESP32_DETECTED=1
                ESP32_PORT=$port
                break
            else
                echo -e "${YELLOW}!${NC} No ESP32 on $port (or permission denied)"
            fi
        fi
    done
else
    echo -e "${YELLOW}!${NC} espflash not available, skipping detection"
fi
echo ""

# Check 8: dmesg for recent USB events
echo "8. Recent USB connection events:"
echo "   (Last 10 lines with 'usb' or 'tty')"
dmesg | grep -i "usb\|tty" | tail -10
echo ""

# Summary and recommendations
echo "═══════════════════════════════════════════════"
echo "  Summary & Recommendations"
echo "═══════════════════════════════════════════════"
echo ""

# Detect if ESP32 is likely connected
if [ "$ESP32_DETECTED" -eq 1 ]; then
    echo -e "${GREEN}ESP32-C3 appears to be connected!${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Build your project:"
    echo "     cd /home/daniah/my-esp32-project"
    echo "     cargo build --release"
    echo ""
    echo "  2. Flash to ESP32:"
    echo "     espflash flash --monitor --port $ESP32_PORT target/riscv32imc-unknown-none-elf/release/my-esp32-project"
else
    echo -e "${RED}ESP32-C3 does NOT appear to be connected${NC}"
    echo ""
    echo "Troubleshooting steps:"
    echo "  1. Connect ESP32-C3 via USB cable"
    echo "  2. Check cable supports data (not just charging)"
    echo "  3. Try different USB ports"
    echo "  4. Press RESET button on board"
    echo "  5. Check: dmesg | tail -30"
    echo "  6. Fix permissions: sudo usermod -a -G dialout \$USER"
fi
echo ""

echo "For more help, see:"
echo "  - QUICKSTART.md"
echo "  - GETTING_STARTED.md"
echo "  - https://esp-rs.github.io/book/"
