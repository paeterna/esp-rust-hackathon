#!/bin/bash

# ESP32 CSI Viewer Auto-reconnect Script
# This script monitors the serial port and automatically restarts the TUI when the device reconnects

TUI_DIR="/home/daniah/esp-csi-tui-rs"

echo "ESP32 CSI Viewer - Auto-reconnect mode"
echo "Press Ctrl+C to exit"
echo ""

# Function to find the first available USB serial device
find_serial_port() {
    # Prefer the persistent device name if it exists
    if [ -e "/dev/esp32-csi" ]; then
        echo "/dev/esp32-csi"
        return 0
    fi
    
    # Fall back to dynamic USB device names
    for port in /dev/ttyUSB* /dev/ttyACM*; do
        if [ -e "$port" ]; then
            echo "$port"
            return 0
        fi
    done
    return 1
}

# Function to check if device is connected
device_exists() {
    SERIAL_PORT=$(find_serial_port)
    [ -n "$SERIAL_PORT" ]
}

# Function to wait for device
wait_for_device() {
    echo "Waiting for ESP32..."
    while ! device_exists; do
        sleep 1
    done
    SERIAL_PORT=$(find_serial_port)
    echo "Device detected on $SERIAL_PORT!"
    sleep 2  # Give device time to initialize
}

# Function to run TUI
run_tui() {
    cd "$TUI_DIR"
    cargo run --release -- --port "$SERIAL_PORT"
    EXIT_CODE=$?
    return $EXIT_CODE
}

# Main loop
while true; do
    # Wait for device to be connected
    wait_for_device
    
    # Run the TUI
    echo "Starting CSI Viewer..."
    run_tui
    
    # If we get here, TUI exited
    echo ""
    echo "TUI stopped. Checking device status..."
    
    # If device is still connected, user probably quit intentionally
    if device_exists; then
        echo "Device still connected. Press Enter to restart, or Ctrl+C to exit."
        read -t 5 || echo "Auto-restarting..."
    else
        echo "Device disconnected. Waiting for reconnection..."
    fi
    
    sleep 1
done
