# Testing RRD Storage

## Usage Examples:

### 1. CSV Export Only (existing feature)
```bash
cargo run --release -- --port /dev/ttyUSB2 --csv output.csv
```

### 2. RRD Export Only (new feature)
```bash
cargo run --release -- --port /dev/ttyUSB2 --rrd csi_data.rrd
```

### 3. Both CSV and RRD Export
```bash
cargo run --release -- --port /dev/ttyUSB2 --csv output.csv --rrd csi_data.rrd
```

### 4. Live Rerun Streaming (no file save)
```bash
cargo run --release -- --port /dev/ttyUSB2 --rerun-live
```

### 5. Live Streaming + RRD Recording
```bash
cargo run --release -- --port /dev/ttyUSB2 --rrd csi_data.rrd --rerun-live
```

### 6. Mock Mode with RRD Export (for testing)
```bash
cargo run --release -- --mock --rrd test_data.rrd
# Press 'q' after collecting some data to save the RRD file
```

## Playback RRD Files

After recording, play back the data in Rerun viewer:
```bash
rerun csi_data.rrd
```

Or stream to a remote viewer:
```bash
rerun --web-viewer csi_data.rrd
```

## What Gets Recorded

The RRD file contains:
- CSI amplitude spectrum (all 64 subcarriers)
- CSI phase spectrum (all 64 subcarriers)
- RSSI values over time
- Timestamps for synchronization

## Status Display

Press 'S' in the TUI to see:
- RRD Recording status (Enabled/Disabled)
- RRD file path
- Number of samples recorded
