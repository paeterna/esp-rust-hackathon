#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::delay::Delay;

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal::main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    esp_println::println!("ESP32-C3 Started!");
    esp_println::println!("=====================");
    esp_println::println!("Ready for CSI hackathon!");
    esp_println::println!("MAC: 48:f6:ee:99:c9:38");

    let mut counter = 0u32;
    loop {
        esp_println::println!("Heartbeat #{} - System OK", counter);
        counter += 1;
        delay.delay_millis(1000);
    }
}
