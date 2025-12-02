#!/bin/bash

# Install udev rule for ESP32-C3 CSI device
# This script installs a udev rule that creates a consistent /dev/esp32-csi symlink

RULES_FILE="99-esp32-csi.rules"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Installing udev rule for ESP32-C3..."

# Copy the rule file
sudo cp "$SCRIPT_DIR/$RULES_FILE" /etc/udev/rules.d/

# Reload udev rules
sudo udevadm control --reload-rules
sudo udevadm trigger

echo "Done! Your ESP32 will now appear as /dev/esp32-csi"
echo "You may need to unplug and replug your device for this to take effect."
