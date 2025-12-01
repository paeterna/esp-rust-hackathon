# CSI-TUI

A Terminal User Interface (TUI) application for visualizing ESP32 Channel State Information (CSI) data.

## Features

- Interactive terminal-based UI built with Ratatui
- Real-time CSI data visualization
- Clean and responsive interface

## Dependencies

- [Ratatui](https://github.com/ratatui/ratatui) - Terminal UI framework
- [Crossterm](https://github.com/crossterm-rs/crossterm) - Cross-platform terminal manipulation
- [Color-eyre](https://github.com/eyre-rs/color-eyre) - Error handling and reporting

## Building

```bash
cargo build --release
```

## Running

```bash
cargo run
```

## Usage

- Press `q` or `Esc` to quit the application

## Project Structure

```
csi-tui/
├── Cargo.toml          # Project dependencies
├── src/
│   └── main.rs         # Main application code
└── README.md           # This file
```

## Next Steps

- Add serial port communication to read CSI data from ESP32
- Implement data visualization widgets (graphs, charts)
- Add data logging and export functionality
- Create configuration options for display settings
