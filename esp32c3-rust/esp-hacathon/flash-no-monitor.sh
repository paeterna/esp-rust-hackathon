#!/bin/bash
# Flash ESP32-C3 without keeping the monitor open
cargo build --release && espflash flash --chip esp32c3 target/riscv32imc-unknown-none-elf/release/esp-hacathon
