use color_eyre::Result;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, stdin};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio_serial::SerialPortBuilderExt;
use crate::csi::frame::CsiFrame;
use crate::csi::parser::CsiParser;

pub struct CsiReader {
    device_path: String,
    frame_tx: mpsc::UnboundedSender<CsiFrame>,
}

impl CsiReader {
    pub fn new(device_path: String, frame_tx: mpsc::UnboundedSender<CsiFrame>) -> Self {
        Self {
            device_path,
            frame_tx,
        }
    }

    /// Start reading CSI data from stdin (for piped input) or ESP32 via espflash monitor
    pub async fn start(&self) -> Result<()> {
        let frame_tx = self.frame_tx.clone();

        // Check if we should read from stdin (for piped input)
        if self.device_path == "stdin" || self.device_path == "-" {
            // Read from stdin (when user pipes espflash output)
            tokio::spawn(async move {
                let mut reader = BufReader::new(stdin()).lines();
                let mut parser = CsiParser::new();
                let mut line_count = 0;

                eprintln!("DEBUG: Reading CSI data from stdin...");
                loop {
                    match reader.next_line().await {
                        Ok(Some(line)) => {
                            line_count += 1;
                            if line_count <= 10 {
                                eprintln!("DEBUG: Line {}: {}", line_count, line);
                            } else if line_count % 100 == 0 {
                                eprintln!("DEBUG: Received {} lines so far", line_count);
                            }
                            if let Some(frame) = parser.parse_line(&line) {
                                eprintln!("DEBUG: Parsed frame with {} amplitudes", frame.amplitudes.len());
                                if frame_tx.send(frame).is_err() {
                                    eprintln!("DEBUG: Channel closed, stopping reader");
                                    break;
                                }
                            }
                        }
                        Ok(None) => {
                            eprintln!("DEBUG: stdin EOF after {} lines", line_count);
                            break;
                        }
                        Err(e) => {
                            eprintln!("DEBUG: Read error after {} lines: {}", line_count, e);
                            break;
                        }
                    }
                }
                eprintln!("DEBUG: stdin reader task ended");
            });

            // Keep the main task alive
            tokio::signal::ctrl_c().await?;
            return Ok(());
        }

        // Otherwise, connect directly to the serial port
        eprintln!("DEBUG: Opening serial port: {}", self.device_path);

        let mut port = tokio_serial::new(&self.device_path, 115200)
            .data_bits(tokio_serial::DataBits::Eight)
            .parity(tokio_serial::Parity::None)
            .stop_bits(tokio_serial::StopBits::One)
            .flow_control(tokio_serial::FlowControl::None)
            .open_native_async()?;

        eprintln!("DEBUG: Serial port opened successfully");

        // Note: Automatic DTR/RTS reset is handled by opening the port
        // The ESP32 should reset automatically when the port opens
        eprintln!("DEBUG: ESP32 should reset automatically on port open");

        // Split the port into read and write halves
        let (reader, writer) = tokio::io::split(port);

        // Create a channel to signal when the prompt is ready
        let (prompt_tx, mut prompt_rx) = mpsc::unbounded_channel::<()>();

        // Read from serial port and look for the prompt
        tokio::spawn(async move {
            let mut buf_reader = BufReader::new(reader);
            let mut parser = CsiParser::new();
            let mut line_count = 0;
            let mut byte_count = 0;
            let mut prompt_sent = false;

            eprintln!("DEBUG: Serial reader task started, waiting for prompt");

            loop {
                let mut line_bytes = Vec::new();
                match buf_reader.read_until(b'\n', &mut line_bytes).await {
                    Ok(0) => {
                        eprintln!("DEBUG: Serial EOF after {} lines, {} bytes", line_count, byte_count);
                        break;
                    }
                    Ok(n) => {
                        byte_count += n;
                        line_count += 1;

                        // Try to convert to UTF-8, replacing invalid sequences
                        let line = String::from_utf8_lossy(&line_bytes).to_string();

                        // Look for the prompt symbol to know when we can send commands
                        if !prompt_sent && line.contains('>') && line_count > 40 {
                            eprintln!("DEBUG: Found prompt at line {}! Signaling to send commands", line_count);
                            let _ = prompt_tx.send(());
                            prompt_sent = true;
                        }

                        if byte_count < 2000 || line_count % 50 == 0 {
                            eprintln!("DEBUG: Line {}: {}", line_count, line.trim());
                        }

                        // Skip lines that look like command echoes
                        if line.starts_with("set-") || line.starts_with("start") {
                            eprintln!("DEBUG: Skipping command echo: {}", line.trim());
                            continue;
                        }

                        if let Some(frame) = parser.parse_line(&line) {
                            eprintln!("DEBUG: Parsed CSI frame with {} amplitudes", frame.amplitudes.len());
                            if frame_tx.send(frame).is_err() {
                                eprintln!("DEBUG: Channel send failed");
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("DEBUG: Serial read error: {}", e);
                        // Don't break on errors, just continue
                        continue;
                    }
                }
            }
            eprintln!("DEBUG: Serial reader task ended");
        });

        // Spawn a task to send initialization commands AFTER prompt appears
        tokio::spawn(async move {
            eprintln!("DEBUG: Waiting for prompt signal...");

            // Wait for the prompt signal
            if prompt_rx.recv().await.is_none() {
                eprintln!("DEBUG: Prompt channel closed, not sending commands");
                return;
            }

            eprintln!("DEBUG: Prompt received! Sending initialization commands...");

            let mut writer = writer;
            let commands = vec![
                "set-wifi --mode=sniffing\n",
                "set-csi\n",
                "start\n",
            ];

            for cmd in commands {
                eprintln!("DEBUG: Sending command: {}", cmd.trim());
                if let Err(e) = writer.write_all(cmd.as_bytes()).await {
                    eprintln!("DEBUG: Failed to send command: {}", e);
                    break;
                }
                if let Err(e) = writer.flush().await {
                    eprintln!("DEBUG: Failed to flush: {}", e);
                    break;
                }
                // Delay between commands
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            eprintln!("DEBUG: All initialization commands sent");
        });

        // Keep the main task alive
        tokio::signal::ctrl_c().await?;

        Ok(())
    }
}
