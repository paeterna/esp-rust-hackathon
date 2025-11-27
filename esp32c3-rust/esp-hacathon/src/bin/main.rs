#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use esp_hal::clock::CpuClock;
use esp_hal::main;
use esp_hal::time::{Duration, Instant};
use esp_hal::timer::timg::TimerGroup;
use esp_hal::uart::Uart;
use esp_radio::ble::controller::BleConnector;
use esp_radio::wifi::ScanConfig;
use core::fmt::Write;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    // generator version: 1.0.1

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 66320);
    // COEX needs more RAM - so we've added some more
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);
    let radio_init = esp_radio::init().expect("Failed to initialize Wi-Fi/BLE controller");
    let (mut wifi_controller, _interfaces) =
        esp_radio::wifi::new(&radio_init, peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");
    let _connector = BleConnector::new(&radio_init, peripherals.BT, Default::default());

    // Initialize UART for USB serial communication
    let mut uart0 = Uart::new(peripherals.UART0, Default::default()).unwrap();

    // Start WiFi in station mode for scanning
    wifi_controller.start().expect("Failed to start WiFi");

    let mut counter = 0u32;
    let mut last_scan = Instant::now();

    // Baseline RSSI values for motion detection (track up to 10 APs)
    let mut baseline_rssi: [i8; 10] = [-100; 10];
    let mut baseline_ssid: [Option<heapless::String<32>>; 10] = [const { None }; 10];

    let _ = writeln!(uart0, "{{\"status\":\"WiFi Motion Sensor Started\"}}");

    loop {
        let now = Instant::now();

        // Scan for WiFi networks every 1 second
        if last_scan.elapsed() >= Duration::from_millis(1000) {
            // Create scan config
            let scan_config = ScanConfig::default();

            // Perform WiFi scan
            match wifi_controller.scan_with_config(scan_config) {
                Ok(scan_results) => {
                    let mut motion_detected = false;
                    let mut rssi_values = heapless::Vec::<_, 10>::new();

                    // Process up to 10 strongest access points
                    for (idx, ap) in scan_results.iter().take(10).enumerate() {
                        let rssi = ap.signal_strength;
                        let ssid_str = ap.ssid.as_str();
                        let ssid_owned: heapless::String<32> =
                            heapless::String::try_from(ssid_str).unwrap_or_default();

                        // Find matching baseline by SSID
                        let mut baseline_idx = None;
                        for (bidx, bssid) in baseline_ssid.iter().enumerate() {
                            if let Some(bs) = bssid {
                                if bs == &ssid_owned {
                                    baseline_idx = Some(bidx);
                                    break;
                                }
                            }
                        }

                        // If not found, add to baseline
                        if baseline_idx.is_none() && idx < 10 {
                            if baseline_ssid[idx].is_none() {
                                baseline_ssid[idx] = Some(ssid_owned.clone());
                                baseline_rssi[idx] = rssi;
                                baseline_idx = Some(idx);
                            }
                        }

                        // Check for motion (RSSI change > 4 dBm)
                        if let Some(bidx) = baseline_idx {
                            let rssi_delta = (rssi - baseline_rssi[bidx]).abs();
                            if rssi_delta > 4 && counter > 5 {
                                motion_detected = true;
                            }

                            // Update baseline with exponential moving average
                            baseline_rssi[bidx] =
                                ((baseline_rssi[bidx] as i16 * 90 + rssi as i16 * 10) / 100) as i8;
                        }

                        // Collect RSSI data for output
                        let _ = rssi_values.push((ssid_str, rssi, ap.channel));
                    }

                    // Send data via UART
                    let _ = write!(uart0, "{{\"counter\":{},\"motion\":{},\"aps\":[",
                        counter,
                        if motion_detected { 1 } else { 0 }
                    );

                    for (idx, (ssid, rssi, channel)) in rssi_values.iter().enumerate() {
                        if idx > 0 {
                            let _ = write!(uart0, ",");
                        }
                        let _ = write!(uart0, "{{\"ssid\":\"{}\",\"rssi\":{},\"ch\":{}}}",
                            ssid, rssi, channel
                        );
                    }

                    let _ = writeln!(uart0, "]}}");

                    counter = counter.wrapping_add(1);
                }
                Err(_) => {
                    let _ = writeln!(uart0, "{{\"error\":\"scan_failed\"}}");
                }
            }

            last_scan = now;
        }

        // Small delay to prevent busy-waiting
        let delay_start = Instant::now();
        while delay_start.elapsed() < Duration::from_millis(50) {}
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0/examples/src/bin
}
