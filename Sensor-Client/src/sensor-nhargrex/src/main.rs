//
// sensor-nhargrex Rust program
// (c) 2024 Nicholas Hargreaves
//
// Requires:
// export GOOGLE_APPLICATION_CREDENTIALS="/opt/.security/sensors-nhargrex-firebase-adminsdk-uev2w-11471882b8.json"
// export GOOGLE_USER_ID="2U0LR6A8LER430Tq4tmdfAdl4iu2" && cargo build && cargo run
//
// To kill:
// ps -eaf | grep sensor | grep nhargrex |  grep -Pio1 'nhargre1\s+\d+' | sed -r s/nhargre1// | xargs kill -9
//
// GND         --> 5
// GPIO PIN 17 --> 6
//
use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use std::env;
use lazy_static::lazy_static;
use rppal::gpio::{Gpio, Trigger};
use std::{thread, time::Duration};
use chrono::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub enum State {
    Open,
    Closed
}

lazy_static! {
    static ref USER_ID_ERROR: String = String::from("Couldn't get GOOGLE_USER_ID");
}

// Constants
const GPIO_PIN : u8 = 17;
const SHOW_STATE : bool = false;
const DEBOUNCE_TIME : Duration = Duration::from_millis(500);
const POLLING_DURATION : Duration = Duration::from_millis(1000);

// Main
fn main() -> Result<(),  Box<dyn std::error::Error>> {

    // initialize atomic reference count mutex to now()
    let interrupt_counter = Arc::new(Mutex::new(SystemTime::now().duration_since(UNIX_EPOCH)?));

    // check user environment variable is set
    let user = get_user_from_env();
    if user == *USER_ID_ERROR { return Err("Couldn't get GOOGLE_USER_ID")? };

    // get gpio pin as input
    let mut sensor_door_pin = Gpio::new()?.get(GPIO_PIN)?.into_input_pullup();

    // create interrupt on gpio pin change
    sensor_door_pin.set_async_interrupt(Trigger::Both, move |level| {

        // obtain Arc mutex
        let mut last_interrupt_time = interrupt_counter.lock().unwrap();

        // get now() for releasing Arc Mutex
        let time_of_interrupt = SystemTime::now().duration_since(UNIX_EPOCH).expect("REASON");

        // calculate the time since last interrupt
        let time_since_last_interrupt = time_of_interrupt.checked_sub(*last_interrupt_time).expect("REASON");

        // process interrupt if time since last interrupt > debounce time (e.g. 500ms)                        
        if time_since_last_interrupt > DEBOUNCE_TIME {
            // get state
            let state : State = if level == rppal::gpio::Level::High {
                State::Open
            } else {
                State::Closed
            };

            // print state
            println!("Door State Change {:?} --> Distance={:?}", state, time_since_last_interrupt);

            // update (cloud) state and notify (Android) user
            if let Err(error) = update_state_and_notify_user(user.clone(), state) {
                panic!("Error: {:?}", error);
            }
        }

        // Update Arc with now and release mutex by dropping out of local scope
        *last_interrupt_time = time_of_interrupt;
    })?;

    println!("Monitoring pin {} (Press <ctrl-c> to exit):", GPIO_PIN.to_string());

    loop {
        thread::sleep(POLLING_DURATION);
        if SHOW_STATE == true {
            println!("{} State {:?}", Utc::now().timestamp(), get_state(&sensor_door_pin));
        }
    }
}

fn get_state(pin : &rppal::gpio::InputPin) -> State {
    if pin.read() == rppal::gpio::Level::High {
        State::Open
    }
    else {
        State::Closed
    }
}

fn update_state_and_notify_user(user: String, state: State) -> PyResult<()> {

    let s : String = match state {
        State::Open => "OPEN".to_lowercase().to_string(),
        State::Closed => "CLOSED".to_lowercase().to_string(),
    };

    Python::with_gil(|py| {
        let firebase = PyModule::import_bound(py, "sensors_nhargrex_firestore")?;
        // python getattr =
        //  test_python_integration
        //  update_state_and_notify_user
        let result: i32 = firebase
            .getattr("update_state_and_notify_user")?
            .call1((user, s,))?
            .extract()?;

        if result > 0 { return Err(PyValueError::new_err("Unexpected error")) };
        
        Ok(())
    })
}

fn get_user_from_env() -> String {
    match env::var("GOOGLE_USER_ID") {
        Ok(user_id) => {
            user_id.to_string()
        }
        Err(_) => String::from("Couldn't get GOOGLE_USER_ID")
    }
}
