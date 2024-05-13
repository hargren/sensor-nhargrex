//
// sensor-nhargrex Rust program
// (c) 2024 Nicholas Hargreaves
//
// Requires:
// export GOOGLE_APPLICATION_CREDENTIALS="/opt/.security/sensors-nhargrex-firebase-adminsdk-uev2w-11471882b8.json"
// export GOOGLE_USER_ID="2U0LR6A8LER430Tq4tmdfAdl4iu2"
// export GOOGLE_PROJECT_ID=sensors-nhargrex
// sudo rm /tmp/sensor-nhargrex.log && cargo build && cargo run
// cargo build --release
// sudo systemctl daemon-reload && sudo systemctl start sensor-nhargrex && sudo systemctl status sensor-nhargrex
//
// To kill:
// ps -eaf | grep sensor | grep nhargrex |  grep -Pio1 'nhargre1\s+\d+' | sed -r s/nhargre1// | xargs kill -9
//
// GND         --> 5
// GPIO PIN 17 --> 6 -- used for interrupt on thread 1
// GPIO PIN 18 --> ? -- used for on document change on thread 2
//
use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use std::env;
use lazy_static::lazy_static;
use rppal::gpio::{Gpio, Trigger};
use std::{thread, time::Duration};
use chrono::{TimeZone, Utc};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use log::LevelFilter;
use simple_logging::{log_to_file};
use firestore::*;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum State {
    Open,
    Closed
}

// Sensor Request Firestore Document
#[derive(Debug, Clone, Deserialize, Serialize)]
struct SensorRefreshRequestObject {
    #[serde(alias = "_firestore_id")]
    doc_id: String,
    r_ts: u64,
    r_cmd: i32
}

// Sensor Request Firestore Document
#[derive(Debug, Clone, Deserialize, Serialize)]
struct SensorObject {
    online: bool,
    state: String
}

lazy_static! {
    static ref USER_ID_ERROR: String = String::from("Couldn't get GOOGLE_USER_ID");
}

// Constants
const GPIO_PIN_17 : u8 = 17;
const GPIO_PIN_18 : u8 = 18;
const SHOW_STATE : bool = false;
const DEBOUNCE_TIME : Duration = Duration::from_millis(500);
const POLLING_DURATION : Duration = Duration::from_millis(5000);
const SENSORS_REFRESH_REQUEST_COLLECTION: &str = "sensorsRefreshRequest";
const SENSORS_COLLECTION: &str = "sensors";
const SENSORS_REFRESH_REQUEST_DOCUMENT_ID: FirestoreListenerTarget = FirestoreListenerTarget::new(17_u32);
const REFRESH_REQUEST_TIMEWINDOW_SECONDS : i64 = -15;

// Main
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    // statup log
    log_to_file("/tmp/sensor-nhargrex.log", LevelFilter::Info).unwrap();
    log::info!("Normal start");

    log::info!("Wait to start");
    thread::sleep(POLLING_DURATION * 6);
    log::info!("Continuing");
    
    // setup firestore
    let db = FirestoreDb::with_options_service_account_key_file(
        FirestoreDbOptions::new(config_env_var("GOOGLE_PROJECT_ID")?.to_string()),
        config_env_var("GOOGLE_APPLICATION_CREDENTIALS")?.to_string().into()
      ).await?;
    log::info!("Firestore DB initialized");

    let mut listener = db
    .create_listener(
        FirestoreMemListenStateStorage::new(),
    )
    .await?;
    log::info!("Firestore DB listener created");

    // add target
    db.fluent()
    .select()
    .from(SENSORS_REFRESH_REQUEST_COLLECTION)
    .listen()
    .add_target(SENSORS_REFRESH_REQUEST_DOCUMENT_ID, &mut listener)?;
    
    // initialize atomic reference count mutex to now()
    let interrupt_counter = Arc::new(Mutex::new(SystemTime::now().duration_since(UNIX_EPOCH)?));

    // check user environment variable is set
    let user = get_user_from_env();
    if user == *USER_ID_ERROR { return Err("Couldn't get GOOGLE_USER_ID")? };

    // get gpio pin as input
    let mut sensor_door_pin = Gpio::new()?.get(GPIO_PIN_17)?.into_input_pullup();

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
            log::info!("Door State Change {:?} --> Distance={:?}", state, time_since_last_interrupt);

            // update (cloud) state and notify (Android) user
            // will only notify if state is different to the one in the cloud
            if let Err(error) = update_state_and_notify_user(user.clone(), state) {
                log::error!("Panic on update_state_and_notify_user {:?}", error);
                panic!("Error: {:?}", error);
            }
        }

        // Update Arc with now and release mutex by dropping out of local scope
        *last_interrupt_time = time_of_interrupt;
    })?;
    
    // start listener thread for document change (refresh request)
    let fs_listener = listener
        .start(|event| async move {
            log::info!("Firestore DB listener event received");
            match event {
                FirestoreListenEvent::DocumentChange(ref doc_change) => {
                    if let Some(doc) = &doc_change.document {
                        let sensor_refresh_request: SensorRefreshRequestObject = FirestoreDb::deserialize_doc_to::<SensorRefreshRequestObject>(doc).expect("Deserialized object");
                        log::info!("Recevied: {sensor_refresh_request:?} r_ts={}, r_cmd={}", sensor_refresh_request.r_ts, sensor_refresh_request.r_cmd);
                        let delta_ts = (Utc.timestamp_opt(sensor_refresh_request.r_ts as i64, 0).unwrap() - Utc::now()).num_seconds();
                        log::info!("Time delta of refresh request: delta={}s", delta_ts);
                        if delta_ts <= 0 && delta_ts > REFRESH_REQUEST_TIMEWINDOW_SECONDS {
                            let command = sensor_refresh_request.r_cmd;
                            match command {
                                0 => {
                                    // cmd => refresh

                                    // get gpio pin as input and read state
                                    let state = get_state(&Gpio::new()?.get(GPIO_PIN_18)?.into_input_pullup());
                                    log::info!("{} Refresh state {:?}", Utc::now().timestamp(), state);

                                    // check user environment variable is set
                                    let user : String = get_user_from_env();

                                    // update (cloud) state and notify (Android) user
                                    // will only notify if state is different to the one in the cloud
                                    if let Err(error) = update_state_and_notify_user(user, state) {
                                        log::error!("Panic on update_state_and_notify_user {:?}", error);
                                        panic!("Error: {:?}", error);
                                    }
                                }
                                1 => {
                                    // cmd => status
                                    log::info!("Refresh online state command request");

                                    // setup firestore
                                    let db = FirestoreDb::with_options_service_account_key_file(
                                        FirestoreDbOptions::new(config_env_var("GOOGLE_PROJECT_ID")?.to_string()),
                                        config_env_var("GOOGLE_APPLICATION_CREDENTIALS")?.to_string().into()
                                    ).await?;
                                    log::info!("Firestore DB initialized");

                                    // Update sensor document with online = true
                                    db.fluent()
                                    .update()
                                    .in_col(SENSORS_COLLECTION)
                                    .document_id(get_user_from_env().clone())
                                    .object(&SensorObject {
                                        online: true,
                                        state: match get_state(&Gpio::new()?.get(GPIO_PIN_18)?.into_input_pullup()) {
                                            State::Open => "OPEN".to_lowercase().to_string(),
                                            State::Closed => "CLOSED".to_lowercase().to_string(),
                                        }
                                    })
                                    .execute()
                                    .await?;
                                    log::info!("Online state set to true");
                                }
                                _ => {
                                    // not recognized
                                    log::info!("Unknown command recevied!");
                                }    
                            }
                        }
                    }
                }
                _ => {
                    log::info!("Received a listen response to handle!");
                }
            }
            Ok(())
        });

    // listen for refresh requests
    fs_listener.await?;
    
    // since we are starting up, and sensor state may have changed on device power-off
    // make a one time update and notify
    if let Err(error) = update_state_and_notify_user(get_user_from_env(), get_state(&sensor_door_pin)) {
        log::error!("Panic on update_state_and_notify_user {:?}", error);
        panic!("Error: {:?}", error);
    }

    // main loop to keep everything ticking
    log::info!("Monitoring pin {} (Press <ctrl-c> to exit):", GPIO_PIN_17.to_string());
    loop {
        thread::sleep(POLLING_DURATION);
        if SHOW_STATE == true {
            log::info!("{} State {:?}", Utc::now().timestamp(), get_state(&sensor_door_pin));
        }
    }
}

pub fn config_env_var(name: &str) -> Result<String, String> {
    log::info!("Try to get environment variable from {}", name);   
    std::env::var(name).map_err(|e| format!("{}: {}", name, e))
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
