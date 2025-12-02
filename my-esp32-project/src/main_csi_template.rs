#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{delay::Delay, prelude::*, uart::Uart, gpio::Io};
use esp_hal::uart::config::Config as UartConfig;

extern crate alloc;
use core::mem::MaybeUninit;
use core::fmt::Write;

fn init_heap() {
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

    unsafe {
        esp_alloc::HEAP.add_region(esp_alloc::HeapRegion::new(
            HEAP.as_mut_ptr() as *mut u8,
            HEAP_SIZE,
            esp_alloc::MemoryCapability::Internal.into(),
        ));
    }
}

#[entry]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();
    init_heap();

    esp_println::logger::init_logger_from_env();
    
    log::info!("ESP32-C3 CSI Data Collection Firmware");
    log::info!("======================================");

    // Initialize WiFi
    let timg0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
    let _init = esp_wifi::init(
        esp_wifi::EspWifiInitFor::Wifi,
        timg0.timer0,
        esp_hal::rng::Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
    )
    .unwrap();

    log::info!("WiFi initialized successfully");

    // Setup UART for communication with host
    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    
    // USB Serial JTAG for communication
    // Note: ESP32-C3 has built-in USB-Serial-JTAG on GPIO18/GPIO19
    log::info!("UART configured for host communication");
    
    // TODO: Initialize CSI collection
    // This requires esp-wifi CSI support or esp-idf bindings
    log::warn!("CSI collection not yet implemented - requires esp-wifi CSI API");
    
    let mut counter = 0u32;
    
    loop {
        log::info!("Heartbeat #{} - Ready for CSI collection", counter);
        
        // TODO: Main loop should:
        // 1. Check for commands from host (via UART)
        // 2. Process CSI configuration commands
        // 3. Collect and send CSI data
        // 4. Handle status requests
        
        counter += 1;
        delay.delay(1000.millis());
    }
}

// Protocol definitions for future implementation
#[allow(dead_code)]
mod protocol {
    /// Commands from host to ESP32
    #[derive(Debug)]
    pub enum Command {
        StartCsi,
        StopCsi,
        ConfigCsi { /* config params */ },
        GetStatus,
        SetChannel { channel: u8 },
    }

    /// Responses from ESP32 to host
    #[derive(Debug)]
    pub enum Response {
        Ack,
        CsiData { /* CSI data */ },
        Status { /* status info */ },
        Error { code: u8 },
    }

    /// CSI configuration
    #[derive(Debug, Clone, Copy)]
    pub struct CsiConfig {
        pub lltf_en: bool,
        pub htltf_en: bool,
        pub stbc_htltf2_en: bool,
        pub ltf_merge_en: bool,
        pub channel_filter_en: bool,
        pub manu_scale: bool,
    }

    impl Default for CsiConfig {
        fn default() -> Self {
            Self {
                lltf_en: true,
                htltf_en: true,
                stbc_htltf2_en: false,
                ltf_merge_en: true,
                channel_filter_en: true,
                manu_scale: false,
            }
        }
    }

    /// CSI data packet header
    #[repr(C, packed)]
    pub struct CsiHeader {
        pub magic: u16,      // 0xC51
        pub length: u16,
        pub timestamp: u32,
        pub rssi: i8,
        pub rate: u8,
        pub channel: u8,
        pub mac: [u8; 6],
    }
}
