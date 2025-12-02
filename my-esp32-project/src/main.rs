use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::uart::*;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::wifi::{BlockingWifi, EspWifi, ClientConfiguration, Configuration};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_sys as _;
use core::ffi::c_void;

// Binary protocol constants
const MAGIC_HEADER: [u8; 2] = [0xC5, 0x1A];
const MSG_TYPE_CSI: u8 = 0x90;
const NUM_SUBCARRIERS: usize = 64;

// CSI data storage - using a larger buffer to avoid data loss
static mut CSI_BUFFER: [(i16, i16); NUM_SUBCARRIERS] = [(0, 0); NUM_SUBCARRIERS];
static mut CSI_READY: bool = false;
static mut CSI_RSSI: i8 = -50;
static mut CSI_TIMESTAMP: u32 = 0;

// Global UART handle for sending CSI data from callback
static mut UART_HANDLE: Option<*mut UartDriver> = None;

// CSI callback function that will be called from ESP-IDF
unsafe extern "C" fn csi_rx_callback(_ctx: *mut c_void, data: *mut esp_idf_sys::wifi_csi_info_t) {
    if data.is_null() {
        return;
    }
    
    let csi_info = &*data;
    
    // Check if we have valid CSI data
    if csi_info.buf.is_null() || csi_info.len == 0 {
        return;
    }
    
    // Extract RSSI from rx_ctrl
    let rssi = csi_info.rx_ctrl.rssi() as i8;
    
    // Parse CSI data - ESP32-C3 provides complex I/Q data
    let raw_data = core::slice::from_raw_parts(csi_info.buf as *const i8, csi_info.len as usize);
    
    // Copy available subcarriers (may be less than 64 depending on bandwidth)
    let num_subcarriers = (csi_info.len as usize / 2).min(NUM_SUBCARRIERS);
    for i in 0..num_subcarriers {
        if i * 2 + 1 < raw_data.len() {
            CSI_BUFFER[i] = (
                raw_data[i * 2] as i16,
                raw_data[i * 2 + 1] as i16
            );
        }
    }
    
    // Fill remaining subcarriers with zeros if needed
    for i in num_subcarriers..NUM_SUBCARRIERS {
        CSI_BUFFER[i] = (0, 0);
    }
    
    CSI_RSSI = rssi;
    CSI_TIMESTAMP = (esp_idf_sys::esp_timer_get_time() / 1000) as u32;
    CSI_READY = true;
    
    // Send CSI data immediately from callback if UART is available
    if let Some(uart_ptr) = UART_HANDLE {
        if !uart_ptr.is_null() {
            let _ = send_csi_data_raw(uart_ptr, &CSI_BUFFER, rssi, CSI_TIMESTAMP);
            CSI_READY = false; // Mark as sent
        }
    }
}

fn calculate_crc8(data: &[u8]) -> u8 {
    let mut crc: u8 = 0x00;  // Match TUI: initial value is 0x00
    for &byte in data {
        crc ^= byte;
        for _ in 0..8 {
            if crc & 0x80 != 0 {
                crc = (crc << 1) ^ 0x07;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

fn send_csi_data(uart: &mut UartDriver, csi_data: &[(i16, i16); NUM_SUBCARRIERS], rssi: i8) -> anyhow::Result<()> {
    let timestamp = unsafe { esp_idf_sys::esp_timer_get_time() / 1000 } as u32;
    unsafe { send_csi_data_raw(uart as *mut UartDriver, csi_data, rssi, timestamp) }
}

// Raw function that can be called from callback with raw pointer
unsafe fn send_csi_data_raw(uart: *mut UartDriver, csi_data: &[(i16, i16); NUM_SUBCARRIERS], rssi: i8, timestamp: u32) -> anyhow::Result<()> {
    // Payload: timestamp(4) + rssi(1) + rate(1) + channel(1) + mac(6) + num_subcarriers(2) + csi_data(64*4)
    let payload_size = 4 + 1 + 1 + 1 + 6 + 2 + (NUM_SUBCARRIERS * 4);
    let mut frame = vec![0u8; 2 + 1 + 2 + payload_size + 1]; // header + type + length + payload + crc
    let mut offset = 0;

    // Magic header
    frame[offset..offset + 2].copy_from_slice(&MAGIC_HEADER);
    offset += 2;

    // Message type
    frame[offset] = MSG_TYPE_CSI;
    offset += 1;

    // Payload length (little-endian)
    let length = payload_size as u16;
    frame[offset..offset + 2].copy_from_slice(&length.to_le_bytes());
    offset += 2;

    // Timestamp (milliseconds since boot)
    frame[offset..offset + 4].copy_from_slice(&timestamp.to_le_bytes());
    offset += 4;

    // RSSI
    frame[offset] = rssi as u8;
    offset += 1;

    // Rate (placeholder)
    frame[offset] = 0x0B; // 11 Mbps placeholder
    offset += 1;

    // Channel (placeholder)
    frame[offset] = 1; // Channel 1 placeholder
    offset += 1;

    // MAC address (placeholder)
    frame[offset..offset + 6].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
    offset += 6;

    // Number of subcarriers
    frame[offset..offset + 2].copy_from_slice(&(NUM_SUBCARRIERS as u16).to_le_bytes());
    offset += 2;

    // CSI data (64 subcarriers, each with I and Q as i16)
    for &(i, q) in csi_data.iter() {
        frame[offset..offset + 2].copy_from_slice(&i.to_le_bytes());
        offset += 2;
        frame[offset..offset + 2].copy_from_slice(&q.to_le_bytes());
        offset += 2;
    }

    // Calculate CRC over type + length + payload (matching TUI: frame[2..frame_size-1])
    let crc_start = 2; // Start after magic header
    let crc_end = offset; // End before CRC byte
    let crc = calculate_crc8(&frame[crc_start..crc_end]);
    frame[offset] = crc;

    // Send frame
    if !uart.is_null() {
        (*uart).write(&frame)?;
    }
    
    Ok(())
}

fn main() -> anyhow::Result<()> {
    // Initialize ESP-IDF services
    esp_idf_svc::sys::link_patches();
    
    log::info!("Starting ESP32 CSI application...");
    
    let peripherals = Peripherals::take()?;
    log::info!("Peripherals initialized");
    log::info!("Peripherals initialized");
    
    let sys_loop = EspSystemEventLoop::take()?;
    log::info!("Event loop created");
    
    let nvs = EspDefaultNvsPartition::take()?;
    log::info!("NVS initialized");

    // Configure UART0 for CSI data transmission
    let config = config::Config::new().baudrate(Hertz(115_200));
    let mut uart = UartDriver::new(
        peripherals.uart0,
        peripherals.pins.gpio21,  // TX
        peripherals.pins.gpio20,  // RX
        Option::<gpio::Gpio0>::None,
        Option::<gpio::Gpio1>::None,
        &config,
    )?;
    log::info!("UART configured");

    // Store UART handle globally for callback access
    unsafe {
        UART_HANDLE = Some(&mut uart as *mut UartDriver);
    }

    log::info!("Initializing WiFi...");
    
    // Initialize WiFi in station mode
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;
    
    log::info!("WiFi initialized successfully");

    log::info!("WiFi initialized successfully");

    // Configure WiFi for promiscuous mode (receive all packets)
    let wifi_config = Configuration::Client(ClientConfiguration {
        ssid: "ESP32-CSI".try_into().unwrap(), // Placeholder SSID
        password: "".try_into().unwrap(),
        ..Default::default()
    });
    
    log::info!("Setting WiFi configuration...");
    wifi.set_configuration(&wifi_config)?;
    
    log::info!("Starting WiFi...");
    wifi.start()?;
    
    log::info!("WiFi started successfully");

    log::info!("WiFi started successfully");

    // Enable CSI
    log::info!("Enabling CSI...");
    unsafe {
        // Configure CSI parameters
        let mut csi_config = esp_idf_sys::wifi_csi_config_t {
            lltf_en: true,           // Enable Long Training Field
            htltf_en: true,          // Enable HT Long Training Field
            stbc_htltf2_en: true,    // Enable STBC HT-LTF2
            ltf_merge_en: true,      // Enable LTF merging
            channel_filter_en: false, // Disable channel filter
            manu_scale: true,        // Manual scaling
            shift: 0,                // No shift
            dump_ack_en: true,       // Enable ACK frame dumping
        };

        // Register CSI callback
        esp_idf_sys::esp_wifi_set_csi_rx_cb(Some(csi_rx_callback), core::ptr::null_mut());
        
        // Enable CSI
        esp_idf_sys::esp_wifi_set_csi_config(&mut csi_config);
        esp_idf_sys::esp_wifi_set_csi(true);

        // Set WiFi to promiscuous mode to capture all packets
        esp_idf_sys::esp_wifi_set_promiscuous(true);
    }
    
    log::info!("CSI enabled, entering main loop...");

    // Main loop - the CSI callback will handle data transmission
    loop {
        // Just keep the system running - CSI data is sent from callback
        FreeRtos::delay_ms(100);
        
        // Optional: Send any buffered CSI data that wasn't sent from callback
        unsafe {
            if CSI_READY {
                let _ = send_csi_data(&mut uart, &CSI_BUFFER, CSI_RSSI);
                CSI_READY = false;
            }
        }
    }
}
