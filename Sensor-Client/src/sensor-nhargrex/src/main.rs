//
// sensor-nhargrex rust program
// (c) 2024 Nicholas Hargreaves
//
// Requires:
// export GOOGLE_APPLICATION_CREDENTIALS="/opt/security/sensors-nhargrex-firebase-adminsdk-uev2w-11471882b8.json"
// export GOOGLE_USER_ID="2U0LR6A8LER430Tq4tmdfAdl4iu2"
//
use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use std::env;
use lazy_static::lazy_static;

pub enum State {
    Open,
    Closed
}

lazy_static! {
    static ref USER_ID_ERROR: String = String::from("Couldn't get GOOGLE_USER_ID");
}

fn main() -> Result<(), PyErr> {

    let state : State = State::Open;

    let user : String = get_user_from_env();

    if user == *USER_ID_ERROR { return Err(PyValueError::new_err("Couldn't get GOOGLE_USER_ID")) };

    // start loop to monitor state here...

    update_state_and_notify_user(user, state)
}

fn update_state_and_notify_user(user: String, state: State) -> PyResult<()> {

    let s : String = match state {
        State::Open => "OPEN".to_lowercase().to_string(),
        State::Closed => "CLOSED".to_lowercase().to_string(),
    };

    Python::with_gil(|py| {
        let firebase = PyModule::import_bound(py, "sensors_nhargrex_firestore")?;
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
