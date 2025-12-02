#!/bin/bash
# Monitor ESP-IDF build progress

if pgrep -f "cargo build" > /dev/null; then
    echo "✓ Build is running..."
    echo ""
    echo "Last 10 lines of build output:"
    tail -10 build.log 2>/dev/null || echo "No log yet"
    echo ""
    echo "To watch continuously: tail -f build.log"
else
    echo "Build finished or not running"
    echo ""
    if grep -q "Finished" build.log 2>/dev/null; then
        echo "✓ BUILD SUCCESSFUL!"
        ls -lh target/riscv32imc-esp-espidf/release/my-esp32-project 2>/dev/null
    elif grep -q "error" build.log 2>/dev/null; then
        echo "✗ BUILD FAILED - Last errors:"
        grep -A 5 "error" build.log | tail -20
    fi
fi
