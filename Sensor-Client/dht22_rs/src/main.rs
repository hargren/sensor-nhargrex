// cargo build --release
// sudo setcap 'cap_sys_nice=eip' target/release/dht22
// ./target/release/dht22

use log::LevelFilter;
use std::{time::Duration};
use simple_logging::{log_to_file};
use dht22_pi::{read, Reading, ReadingError};

//const GPIO_PIN_18 : u8 = 18;
const GPIO_PIN_27 : u8 = 27;

pub fn main() {
    log_to_file("dht22.log", LevelFilter::Info).unwrap();
    log::info!("+main");
    loop {
        match read(GPIO_PIN_27) {
            Ok(Reading {temperature, humidity}) => {
                let temp_f = temperature * 9.0 / 5.0 + 32.0;
                log::info!("DHT22 Reading: Temp: {:.2} °F, Humidity: {:.2} %", temp_f, humidity);
            },
            Err(ReadingError::Timeout) => {
                log::warn!("DHT22 timeout");
            },
            Err(ReadingError::Checksum) => {
                log::warn!("DHT22 checksum error");
            },
            Err(ReadingError::Gpio(_)) => {
                log::warn!("DHT22 GPIO error");
            }
        }
        std::thread::sleep(Duration::from_millis(5000));
    }
}

