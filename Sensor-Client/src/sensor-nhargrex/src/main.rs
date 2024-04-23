//
// sensor-nhargrex Rust program
// (c) 2024 Nicholas Hargreaves
//
// Requires:
// export GOOGLE_APPLICATION_CREDENTIALS="/opt/.security/sensors-nhargrex-firebase-adminsdk-uev2w-11471882b8.json"
// export GOOGLE_USER_ID="2U0LR6A8LER430Tq4tmdfAdl4iu2"
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
use std::sync::Mutex;

#[derive(Debug)]
pub enum State {
    Open,
    Closed
}

lazy_static! {
    static ref USER_ID_ERROR: String = String::from("Couldn't get GOOGLE_USER_ID");
}

const GPIO_PIN : u8 = 17;

fn main() -> Result<(),  Box<dyn std::error::Error>> {

    let duration = Duration::from_secs(1);

    let interrupt_mutex = Mutex::new(()); // Initialize the mutex

    // check user environment variable is set
    let user = get_user_from_env();
    if user == *USER_ID_ERROR { return Err("Couldn't get GOOGLE_USER_ID")? };

    // get gpio pin as input
    let mut sensor_door_pin = Gpio::new()?.get(GPIO_PIN)?.into_input();

    // create interrupt on gpio pin change
    sensor_door_pin.set_async_interrupt(Trigger::Both, move |level| {
        
        // get mutex
        let _lock = interrupt_mutex.lock().unwrap();

        let u : String = user.clone();

        // get state
        let state : State = if level == rppal::gpio::Level::High {
            State::Open
        } else {
            State::Closed
        };

        println!("Door State Change {:?}: {:?} {}", state, level, Utc::now().timestamp());

        // update (cloud) state and notify (Android) user
        if let Err(error) = update_state_and_notify_user(u, state) {
            panic!("Error: {:?}", error);
        }
    })?;

    println!("Monitoring pin {} (Press <ctrl-c> to exit):", GPIO_PIN.to_string());

    loop {
        thread::sleep(duration);
        println!("Current State {:?}", get_state(&sensor_door_pin));
    }
}

fn get_state(pin : &rppal::gpio::InputPin) -> State {
    if pin.read() == rppal::gpio::Level::High {
        State::Closed
    }
    else {
        State::Open
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
            .getattr("test_python_integration")?
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
