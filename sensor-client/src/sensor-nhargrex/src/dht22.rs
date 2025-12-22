//! This is a Rust API to obtain temperature and humidity measurements from a DHT22 connected to
//! a Raspberry Pi.
//!
//! This library is essentially a port of the 
//! [Adafruit_Python_DHT](https://github.com/adafruit/Adafruit_Python_DHT) library from C to Rust.  
//!
//! This library has been tesed on a DHT22 from Adafruit using a Raspberry Pi Module B+.
//!
extern crate rppal;
extern crate libc;

use std::ptr::read_volatile;
use std::ptr::write_volatile;

use std::thread::sleep;
use std::time::Duration;

use std::sync::Arc;
use std::sync::Mutex;

use rppal::gpio::Level;
use rppal::gpio::Mode;
use rppal::gpio::IoPin;

// A temperature and humidity reading from the DHT22.
#[derive(Debug, Clone, Copy)]
pub struct Reading {
    pub temperature: f32,
    pub humidity: f32
}

/// Errors that may occur when reading temperature.
#[derive(Debug)]
pub enum ReadingError {
    /// Occurs if a timeout occured reading the pin.
    Timeout,

    /// Occurs if the checksum value from the DHT22 is incorrect.
    Checksum,

    /// Occurs if there is a problem accessing gpio itself on the Raspberry PI.
    Gpio(())
}

impl From<rppal::gpio::Error> for ReadingError {
    fn from(_err: rppal::gpio::Error) -> ReadingError {
        ReadingError::Gpio(())
    }
}

const MAX_COUNT:usize = 32000;
const DHT_PULSES:usize = 41;

fn tiny_sleep() {
    let mut i = 0;
    unsafe {
        while read_volatile(&mut i) < 50 {
            write_volatile(&mut i, read_volatile(&mut i) + 1);
        }
    }
}

fn decode(arr:[usize; DHT_PULSES*2]) -> Result<Reading, ReadingError> {
    let mut threshold:usize = 0;

    let mut i = 2;
    while i < DHT_PULSES * 2 {
        threshold += arr[i];

        i += 2;
    }

    threshold /= DHT_PULSES - 1;

    let mut data = [0 as u8; 5];
    let mut i = 3;
    while i < DHT_PULSES * 2 {
        let index = (i-3) / 16;
        data[index] <<= 1;
        if arr[i] >= threshold {
            data[index] |= 1;
        } else {
            // else zero bit for short pulse
        }

        i += 2;
    }

    if data[4] != (data[0].wrapping_add(data[1]).wrapping_add(data[2]).wrapping_add(data[3]) & 0xFF) {
        return Result::Err(ReadingError::Checksum);
    }

    let h_dec = data[0] as u16 * 256 + data[1] as u16;
    let h = h_dec as f32 / 10.0f32;

    let t_dec = (data[2] & 0x7f) as u16 * 256 + data[3] as u16;
    let mut t = t_dec as f32 / 10.0f32;
    if (data[2] & 0x80) != 0 {
        t *= -1.0f32;
    }

    Result::Ok(Reading {
        temperature: t,
        humidity: h
    })
}

/// Read temperature and humidity from a DHT22 connected to a Gpio pin on a Raspberry Pi.
/// 
/// On a Raspberry Pi this is implemented using bit-banging which is very error-prone.  It will
/// fail 30% of the time.  You should write code to handle this.  In addition you should not
/// attempt a reading more frequently than once every 2 seconds because the DHT22 hardware does
/// not support that.
///
pub fn read_dht22(pin: &Arc<Mutex<IoPin>>) -> Result<Reading, ReadingError> {

    let mut gpio = pin.lock().unwrap();

    gpio.set_mode(Mode::Output); // changes mode in place

    let mut pulse_counts: [usize; DHT_PULSES*2] = [0; DHT_PULSES * 2];

    gpio.write(Level::High);
    sleep(Duration::from_millis(500));

    gpio.write(Level::Low);
    sleep(Duration::from_millis(20));

    gpio.set_mode(Mode::Input);

    // Sometimes the pin is briefly low.
    tiny_sleep();
    
    let mut count:usize = 0;

    while gpio.read() == Level::High {
        count = count + 1;

        if count > MAX_COUNT {
            return Result::Err(ReadingError::Timeout);
        }
    }

    for c in 0..DHT_PULSES {
        let i = c * 2;


        while gpio.read() == Level::Low {
            pulse_counts[i] = pulse_counts[i] + 1;

            if pulse_counts[i] > MAX_COUNT {
                return Result::Err(ReadingError::Timeout);
            }
        }

        while gpio.read() == Level::High {
            pulse_counts[i + 1] = pulse_counts[i + 1] + 1;

            if pulse_counts[i + 1] > MAX_COUNT {
                return Result::Err(ReadingError::Timeout);
            }
        }
    }

    decode(pulse_counts)
}
