// Simple serial port test
use tokio_serial::SerialPortBuilderExt;
use tokio::io::{AsyncBufReadExt, BufReader};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port_name = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/dev/tty.usbserial-110".to_string());

    println!("Opening serial port: {}", port_name);

    let port = tokio_serial::new(&port_name, 115200)
        .data_bits(tokio_serial::DataBits::Eight)
        .parity(tokio_serial::Parity::None)
        .stop_bits(tokio_serial::StopBits::One)
        .flow_control(tokio_serial::FlowControl::None)
        .open_native_async()?;

    println!("Port opened! Reading lines...");
    println!("Press the RESET button on your ESP32 now!");
    println!("----------------------------------------");

    let mut reader = BufReader::new(port);
    let mut line = String::new();
    let mut count = 0;

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                println!("EOF");
                break;
            }
            Ok(n) => {
                count += 1;
                println!("[{}] ({} bytes) {}", count, n, line.trim_end());
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }
    }

    Ok(())
}
