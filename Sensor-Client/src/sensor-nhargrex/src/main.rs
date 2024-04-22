//
// sensor-nhargrex rust program
// (c) 2024 Nicholas Hargreaves
//
// Requires:
// export GOOGLE_APPLICATION_CREDENTIALS="/opt/.security/sensors-nhargrex-firebase-adminsdk-uev2w-11471882b8.json"
// export GOOGLE_USER_ID="2U0LR6A8LER430Tq4tmdfAdl4iu2"
//
//use pyo3::prelude::*;
//use pyo3::exceptions::PyValueError;
use std::env;
use lazy_static::lazy_static;
use rppal::gpio::{Gpio, Trigger};
//use std::{thread, time::Duration};
use chrono::prelude::*;
use std::io;

#[derive(Debug)]
pub enum State {
    Open,
    Closed
}

lazy_static! {
    static ref USER_ID_ERROR: String = String::from("Couldn't get GOOGLE_USER_ID");
}

const GPIO_PIN : u8 = 17;

//fn main() -> Result<(), PyErr> {
fn main() -> Result<(),  Box<dyn std::error::Error>> {

    let _state : State = State::Open;

    let user : String = get_user_from_env();

    //let interval = Duration::from_secs(1); // Set your desired delay (e.g., 1 second)

    //if user == *USER_ID_ERROR { return Err(PyValueError::new_err("Couldn't get GOOGLE_USER_ID")) };
    if user == *USER_ID_ERROR { return Err("Couldn't get GOOGLE_USER_ID")? };

    let gpio = Gpio::new()?;

    let mut sensor_door_pin = gpio.get(GPIO_PIN)?.into_input();

    sensor_door_pin.set_reset_on_drop(true);

    // create interrupt on gpio pin change
    sensor_door_pin.set_async_interrupt(Trigger::Both, |level| {
        let door_status : State = if level == rppal::gpio::Level::High {
            State::Open
        } else {
            State::Closed
        };
        println!("Door {:?}: {:?} {}", door_status, level, Utc::now().timestamp());
    })?;

    println!("Monitoring pin {} (Press <enter> to exit):", GPIO_PIN.to_string());

    let _ = io::stdin().read_line(&mut String::new());

    sensor_door_pin.clear_async_interrupt()?;

    Ok(())

    // start loop to monitor state here...
    // update_state_and_notify_user(user, state)
}

/*
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
*/
fn get_user_from_env() -> String {
    match env::var("GOOGLE_USER_ID") {
        Ok(user_id) => {
            user_id.to_string()
        }
        Err(_) => String::from("Couldn't get GOOGLE_USER_ID")
    }
}
