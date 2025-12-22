//
// sensor-nhargrex Rust program
// (c) 2024 Nicholas Hargreaves
//
// See README.md for details.
//
mod dht22;
use crate::dht22::{Reading, ReadingError, read_dht22};
use log::LevelFilter;
use simple_logging::{log_to_file};
use firestore::*;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration, interval};
use lazy_static::lazy_static;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use std::thread;
use chrono::{Utc, TimeZone};
use rppal::gpio::{Gpio, Trigger, InputPin, Mode, IoPin};
use pyo3::exceptions::PyValueError;
use pyo3::types::PyModule;
use pyo3::prelude::PyAnyMethods;
use pyo3::Python;
use pyo3::PyResult;
use anyhow::{Result};

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
    state: String,
    temp_f: f32,
    humidity: f32
}

lazy_static! {
    static ref USER_ID_ERROR: String = String::from("Couldn't get GOOGLE_USER_ID");
}

// Constants
const GPIO_PIN_17 : u8 = 17; // door sensor
const GPIO_PIN_18 : u8 = 18; // primary dht22
const GPIO_PIN_27 : u8 = 27; // secondary dht22
const SHOW_STATE : bool = false;
const DEBOUNCE_TIME : Duration = Duration::from_millis(500);
const POLLING_DURATION : Duration = Duration::from_millis(5000);
const SENSORS_REFRESH_REQUEST_COLLECTION: &str = "sensorsRefreshRequest";
const SENSORS_COLLECTION: &str = "sensors";
const SENSORS_REFRESH_REQUEST_DOCUMENT_ID: FirestoreListenerTarget = FirestoreListenerTarget::new(17_u32);
const REFRESH_REQUEST_TIMEWINDOW_SECONDS : i64 = -15;
const DHT22_TEMP_WARNING_F : f32 = 58.0;

// Main
#[tokio::main]
#[allow(dependency_on_unit_never_type_fallback)]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    
    // sendor door pin
    let sensor_door_pin = Arc::new(Mutex::new(Gpio::new()?.get(GPIO_PIN_17)?.into_input_pullup()));

    // temp sensor pin
    let sensor_primary_temp_pin = Arc::new(Mutex::new(Gpio::new()?.get(GPIO_PIN_18)?.into_io(Mode::Output)));

    // temp sensor pin
    let sensor_secondary_temp_pin = Arc::new(Mutex::new(Gpio::new()?.get(GPIO_PIN_27)?.into_io(Mode::Output)));

    // clone for worker/polling threads
    let sensor_door_pin_for_callback = sensor_door_pin.clone();
    let sensor_door_pin_for_command = sensor_door_pin.clone();
    let sendor_door_pin_for_startup = sensor_door_pin.clone();
    let worker_sensor_state_pin = sensor_door_pin.clone();
    let worker_sensor_primary_temp_pin = sensor_primary_temp_pin.clone();
    let init_sensor_primary_temp_pin = sensor_primary_temp_pin.clone();
    let sensor_primary_temp_pin_for_command = sensor_primary_temp_pin.clone();
    let sensor_secondary_temp_pin_for_startup = sensor_secondary_temp_pin.clone();
    let sensor_pin_for_command = sensor_door_pin_for_command.clone();
    let sensor_pin_for_temp_monitor = sensor_door_pin.clone();

    // statup log
    log_to_file("/tmp/sensor-nhargrex.log", LevelFilter::Info).unwrap();
    log::info!("Normal start");

    log::info!("Wait to start (for network)");
    thread::sleep(POLLING_DURATION * 6);
    log::info!("Continuing");

    // initialize firestore
    let project_id = config_env_var("GOOGLE_PROJECT_ID")?.to_string();
    let key_file = config_env_var("GOOGLE_APPLICATION_CREDENTIALS")?.to_string().into();
    let db = init_firestore_with_retry(project_id, key_file, 10)
        .await
        .map_err(|e| {
            log::error!("Firestore initialization failed: {:?}", e);
            e
        })?;

    log::info!("Firestore DB initialized");

    let mut listener = db
    .create_listener(
        FirestoreMemListenStateStorage::new(),
    )
    .await?;
    log::info!("Firestore DB listener created");

    // add target for listener
    db.fluent()
    .select()
    .from(SENSORS_REFRESH_REQUEST_COLLECTION)
    .listen()
    .add_target(SENSORS_REFRESH_REQUEST_DOCUMENT_ID, &mut listener)?;
    
    // initialize atomic reference count mutex to now()
    let interrupt_counter = Arc::new(Mutex::new(SystemTime::now().duration_since(UNIX_EPOCH)?));

    // check user environment variable is set
    let user = config_env_var("GOOGLE_USER_ID")?.to_string();
    let monitor_user = user.clone();
    let startup_user = user.clone();
    let command_user = user.clone();

    // used to hold initial temp/humidity reading moved into worker thread
    let mut inital_temp_f: f32 = 0.0;
    let mut inital_humidity: f32 = 0.0;

    // initialize temp sensor and get initial reading
    const MAX_RETRIES: u8 = 10;
    const INITIAL_DELAY_SECS: u64 = 1;

    for attempt in 1..=MAX_RETRIES {
        match read_dht22(&init_sensor_primary_temp_pin) {
            Ok(Reading { temperature, humidity }) => {
                let temp_f = temperature * 9.0 / 5.0 + 32.0;
                log::info!("Initial DHT22 Reading: Temp: {:.2} °F, Humidity: {:.2} %", temp_f, humidity);
                inital_temp_f = temp_f;
                inital_humidity = humidity;
                break;
            },
            Err(e) => {
                log::warn!("Firestore/DHT sync failed, attempt {}/{}: {:?}", attempt, MAX_RETRIES, e);
                
                if attempt == MAX_RETRIES {
                    log::error!("Failed to initialize after {} attempts", MAX_RETRIES);
                    panic!("Initialization failed");
                }

                // Exponential Backoff: 1s, 2s, 4s, 8s, 16s...
                let backoff_duration = Duration::from_secs(INITIAL_DELAY_SECS * 2u64.pow(attempt as u32 - 1));
                
                log::info!("Waiting {:?} before next retry...", backoff_duration);
                sleep(backoff_duration).await;
            }
        }
    }

    // create a channel and worker thread to handle potentially blocking work
    let (tx, rx) = std::sync::mpsc::channel::<rppal::gpio::Level>();

    // worker thread that handles debounced sensor door pin async interrupt
    // read door state and dht22 and send to cloud 
    std::thread::spawn(move || {
        log::info!("GPIO worker thread started");
        let mut last_good_temp_f: f32 = inital_temp_f.clone();
        let mut last_good_humidity: f32 = inital_humidity.clone();

        for level in rx {
            let state: State = if level == rppal::gpio::Level::High {
                State::Open
            } else {
                State::Closed
            };

            // immediate visibility that worker got the event
            log::info!("GPIO worker received event: {:?}", state);

            // read temp from temp_sensor_secondary_pin
            let worker_sensor_primary_temp_pin = Arc::clone(&worker_sensor_primary_temp_pin);
            let worker_user = user.clone();

            match read_dht22(&worker_sensor_primary_temp_pin) {
                Ok(Reading {temperature, humidity}) => {
                    let temp_f = temperature * 9.0 / 5.0 + 32.0;
                    log::info!("GPIO worker DHT22 Reading: Temp: {:.2} °F, Humidity: {:.2} %", temp_f, humidity);

                    // Quick sanity check before calling the Python updater
                    if !( (-40.0..=125.0).contains(&temp_f) && (0.0..=100.0).contains(&humidity) ) {
                        log::warn!("GPIO worker DHT22 reading out of range, skipping update: {:.2}°F, {:.2}%", temp_f, humidity);
                        continue;
                    }

                    // Skip if reading is obviously invalid
                    if temp_f == 32.0 && humidity == 0.0 {
                        log::warn!("GPIO worker DHT22 invalid, skipping update");
                        continue;
                    }

                    if let Err(error) = update_state_temp_f_humidity_and_notify_user(worker_user, read_shared_state(&worker_sensor_state_pin), Some(temp_f), Some(humidity), Some(false)) {
                        log::error!("update_state_temp_f_humidity_and_notify_user {:?}", error);
                    }

                    // cache last know good reading for use if reading fails next time
                    last_good_temp_f = temp_f;
                    last_good_humidity = humidity;
                },
                Err(_) => {
                    log::warn!("GPIO worker DHT22 reading failed, sending state update previous temp/humidity: {:.2}°F, {:.2}%", last_good_temp_f, last_good_humidity);
                    if let Err(error) = update_state_temp_f_humidity_and_notify_user(worker_user, read_shared_state(&worker_sensor_state_pin), Some(last_good_temp_f), Some(last_good_humidity), Some(false)) {
                        log::error!("update_state_temp_f_humidity_and_notify_user {:?}", error);
                    }
                }
            }
        }
        log::info!("GPIO worker thread exiting");
    });

    // async interrupt on GPIO sensor door pin
    // sends mspc message to worker thread.
    {
        let tx_int = tx.clone();
        let interrupt_counter = interrupt_counter.clone();
        let pin_for_cb = sensor_door_pin_for_callback.clone();

        // Note: set_async_interrupt needs &mut InputPin. We can lock the mutex to get a &mut guard,
        // then call set_async_interrupt on that guarded mutable reference.
        let mut guard = pin_for_cb.lock().unwrap();
        guard.set_async_interrupt(Trigger::Both, move |level| {
            log::debug!("GPIO interrupt callback fired: level={:?}", level);

            let mut last_interrupt_time = interrupt_counter.lock().unwrap();
            let time_of_interrupt = SystemTime::now().duration_since(UNIX_EPOCH).expect("REASON");
            let time_since_last_interrupt = time_of_interrupt.checked_sub(*last_interrupt_time).expect("REASON");

            if time_since_last_interrupt > DEBOUNCE_TIME {
                log::debug!("GPIO interrupt callback: level={:?} debounce_ok distance={:?}", level, time_since_last_interrupt);
                if let Err(e) = tx_int.send(level) {
                    log::error!("Failed to send GPIO event to worker: {:?}", e);
                }
            }

            *last_interrupt_time = time_of_interrupt;
        })?;
        // guard is dropped here (releases lock)
    }
    log::info!("GPIO sensor door interrupt installed OK");
    
    // start listener thread for document change (refresh request)
    let fs_listener = listener
        .start(move |event| {
            // clone again for each invocation (cheap) so the inner async block owns its Arc
            let door_pin = sensor_pin_for_command.clone();
            let temp_pin = sensor_primary_temp_pin_for_command.clone();
            let v_user = command_user.clone();
            async move {
                log::info!("Firestore DB listener event received");
                match event {
                    FirestoreListenEvent::DocumentChange(ref doc_change) => {
                        if let Some(doc) = &doc_change.document {
                            let sensor_refresh_request: SensorRefreshRequestObject = FirestoreDb::deserialize_doc_to::<SensorRefreshRequestObject>(doc).expect("Deserialized object");
                            log::info!("Recevied: {sensor_refresh_request:?} r_ts={}, r_cmd={}", sensor_refresh_request.r_ts, sensor_refresh_request.r_cmd);
                            let delta_ts = (Utc.timestamp_opt(sensor_refresh_request.r_ts as i64, 0).unwrap() - Utc::now()).num_seconds();
                            log::info!("Time delta of refresh request: delta={}s", delta_ts);
                            // only process the change if it was recently in the past or now
                            if delta_ts <= 0 && delta_ts > REFRESH_REQUEST_TIMEWINDOW_SECONDS {
                                let command = sensor_refresh_request.r_cmd;
                                match command {
                                    0 => {
                                        // cmd => refresh
                                        log::info!("Command: refresh");

                                        // get gpio pin as input and read state
                                        let state = read_shared_state(&door_pin);

                                        // get temp and humidity
                                        let t : f32;
                                        let h : f32;

                                        match read_dht22_once(&temp_pin) {
                                            Ok(Reading {  temperature, humidity }) => {
                                                t = temperature;
                                                h = humidity;    
                                            }
                                            Err(_) => {
                                                t = 0.0;
                                                h = 0.0;                                                  
                                            }
                                        }

                                        log::info!("State {:?}, Temp: {:.2}°F, Humidity: {:.2}%", state, t, h);

                                        // (force) update (cloud) state and notify (Android) user
                                        if let Err(error) = update_state_temp_f_humidity_and_notify_user(v_user, state, Some(t), Some(h), Some(true)) {
                                            log::error!("Error on update_state_temp_f_humidity_and_notify_user {:?} - continuing", error);
                                            // continue without this update
                                        }
                                    }
                                    1 => {
                                        // cmd => status
                                        log::info!("Command: status");

                                        // setup firestore
                                        let db = FirestoreDb::with_options_service_account_key_file(
                                            FirestoreDbOptions::new(config_env_var("GOOGLE_PROJECT_ID")?.to_string()),
                                            config_env_var("GOOGLE_APPLICATION_CREDENTIALS")?.to_string().into()
                                        ).await?;
                                        log::info!("Firestore DB initialized");

                                        // read temp and humidity
                                        let t : f32;
                                        let h : f32;

                                        match read_dht22_once(&temp_pin) {
                                            Ok(Reading {temperature, humidity}) => {
                                                t = temperature;
                                                h = humidity;    
                                            }
                                            Err(_) => {
                                                t = 0.0;
                                                h = 0.0;                                                  
                                            }
                                        }

                                        // Update sensor document with online = true
                                        db.fluent()
                                        .update()
                                        .in_col(SENSORS_COLLECTION)
                                        .document_id(config_env_var("GOOGLE_USER_ID")?.to_string().clone())
                                        .object(&SensorObject {
                                            online: true,
                                            state: match read_shared_state(&door_pin) {
                                                State::Open => "OPEN".to_lowercase().to_string(),
                                                State::Closed => "CLOSED".to_lowercase().to_string(),
                                            },
                                            temp_f: t,
                                            humidity: h
                                        })
                                        .execute::<()>()
                                        .await?;
                                        log::info!("Status and temperature updated to current");
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
                        log::info!("Received a listen response - dropped");
                    }
                }
                Ok(())
            }
        });

    // listen for refresh requests
    fs_listener.await?;

    // secondary dht22 reading (using rust lib)
    // poll and alert on low temp
    tokio::spawn({
        let sensor_secondary_temp_pin = Arc::clone(&sensor_secondary_temp_pin);
        let monitor_user = monitor_user.clone();
        async move {
            sleep(Duration::from_secs(10)).await;
            log::info!("Starting Low Temp Warning Monitor periodic DHT22 read task");
            let mut iv = interval(Duration::from_secs(5)); // every 5 seconds
            loop {
                iv.tick().await;
                match read_dht22(&sensor_secondary_temp_pin) {
                    Ok(Reading {temperature, humidity}) => {
                        let temp_f = temperature * 9.0 / 5.0 + 32.0;
                        log::debug!("(Secondary) DHT22 Reading: Temp: {:.2} °F, Humidity: {:.2} %", temp_f, humidity);

                        // Quick sanity check before calling the Python updater
                        if !( (-40.0..=125.0).contains(&temp_f) && (0.0..=100.0).contains(&humidity) ) {
                            log::warn!("(Secondary) DHT22 reading out of range, skipping update: {:.2}°F, {:.2}%", temp_f, humidity);
                            continue;
                        }

                        if (temp_f == 32.0) && (humidity == 0.0) {
                            log::warn!("(Secondary) DHT22 reading invalid (32.0°F, 0.0%), skipping update");
                            continue;
                        }   

                        // now check to see if temp_f is below warning temp
                        if temp_f < DHT22_TEMP_WARNING_F && temperature != 0.0 {
                            let force_notify = true;
                            log::warn!("(Secondary) DHT22 temperature below warning temp {:.2}: {:.2} °F", DHT22_TEMP_WARNING_F, temp_f);

                            if let Err(error) = update_state_temp_f_humidity_and_notify_user(monitor_user.clone(), read_shared_state(&sensor_pin_for_temp_monitor), Some(temp_f), Some(humidity), Some(force_notify)) {
                                log::error!("Panic on update_state_temp_f_humidity_and_notify_user {:?}", error);
                                //panic!("Error: {:?}", error);
                            }

                            // Wait 8 hours before polling again
                            log::warn!("Waiting for 8 hours before checking again...");
                            tokio::time::sleep(Duration::from_secs(60 * 60 * 8)).await;
                        }
                    },
                    Err(ReadingError::Timeout) => {
                        log::debug!("(Secondary) DHT22 timeout (GPIO18)");
                    },
                    Err(ReadingError::Checksum) => {
                        log::debug!("(Secondary) DHT22 checksum error(GPIO18)");
                    },
                    Err(ReadingError::Gpio(_)) => {
                        log::debug!("(Secondary) DHT22 GPIO error (GPIO18)");
                    }
                }
            }
        }
    });

    // since we are starting up, and sensor state may have changed on device power-off
    // make a one time update and notify
    start_update_sensor_read_and_user_update_and_notitfy(
        startup_user.clone(),
        &sensor_secondary_temp_pin_for_startup,
        &sendor_door_pin_for_startup)
        .await;

    // main loop to keep everything alive, should never exit
    log::info!("Monitoring pin {} (Press <ctrl-c> to exit):", GPIO_PIN_17.to_string());
    loop {
        thread::sleep(POLLING_DURATION);
        if SHOW_STATE == true {
            log::info!("{} State {:?}", Utc::now().timestamp(), read_shared_state(&sensor_door_pin));
        }
    }
}

pub fn config_env_var(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|e| format!("{}: {}", name, e))
}

// helper to read shared pin state for door open/closed sensor
pub fn read_shared_state(pin: &Arc<Mutex<InputPin>>) -> State {
    let guard = pin.lock().unwrap();
    if guard.read() == rppal::gpio::Level::High {
        State::Open
    } else {
        State::Closed
    }
}

pub fn update_state_temp_f_humidity_and_notify_user(user: String, state: State, temp_f: Option<f32>, humidity: Option<f32>, force_notify: Option<bool>) -> PyResult<()> {

    let s : String = match state {
        State::Open => "OPEN".to_lowercase().to_string(),
        State::Closed => "CLOSED".to_lowercase().to_string(),
    };

    let t = match temp_f {
        Some(t) => t,
        None => 0.0
    };

    let h = match humidity {
        Some(h) => h,
        None => 0.0
    };

    let f = match force_notify {
        Some(f) => f,
        None => false
    };

    if (t == 0.0) || (h == 0.0) {
        log::warn!("update_state_temp_f_humidity_and_notify_user called with invalid temp/humidity: t={}, h={}", t, h);
        return Ok(());
    }

    Python::with_gil(|py| {
        let firebase = PyModule::import_bound(py, "sensors_nhargrex_firestore")?;
        //  update_state_and_notify_user
        let result: i32 = firebase
            .getattr("update_state_and_notify_user")?
            .call1((user, s, t, h, f,))?
            .extract()?;

        if result > 0 { return Err(PyValueError::new_err("Unexpected error")) };
        
        Ok(())
    })
}

pub fn read_dht22_once(sensor_temp_pin: &Arc<Mutex<IoPin>>) -> Result<Reading, ReadingError> {
    match read_dht22(sensor_temp_pin) {
        Ok(Reading { temperature, humidity }) => {
            let temp_f = temperature * 9.0 / 5.0 + 32.0;
            return Result::Ok(Reading {
                temperature: temp_f,
                humidity: humidity
            });
        }
        Err(_) => {
            log::warn!("DHT22 reading failed");
            return Result::Err(ReadingError::Timeout);
        }
    }
}

/// Initialize Firestore with retries and exponential backoff
pub async fn init_firestore_with_retry(
    project_id: String,
    key_file: String,
    max_attempts: usize,
) -> Result<FirestoreDb> {
    let mut attempt = 0;
    let mut delay = Duration::from_secs(2);

    loop {
        attempt += 1;
        log::info!("Attempt {} to initialize Firestore DB", attempt);

        // Wrap in timeout so we don't hang forever
        let result = tokio::time::timeout(
            Duration::from_secs(20),
            FirestoreDb::with_options_service_account_key_file(
                FirestoreDbOptions::new(project_id.clone()),
                key_file.clone().into(),
            ),
        )
        .await;

        match result {
            Ok(Ok(db)) => {
                log::info!("Firestore DB initialized successfully on attempt {}", attempt);
                return Ok(db);
            }
            Ok(Err(e)) => {
                log::warn!("Firestore init failed: {:?}", e);
            }
            Err(_) => {
                log::warn!("Firestore init timed out after 20s");
            }
        }

        if attempt >= max_attempts {
            return Err(anyhow::anyhow!(
                "Failed to initialize Firestore after {} attempts",
                attempt
            ));
        }

        log::info!("Retrying in {:?}...", delay);
        sleep(delay).await;
        delay *= 2; // exponential backoff
    }
}


pub async fn start_update_sensor_read_and_user_update_and_notitfy(
    startup_user: String,
    sensor_secondary_temp_pin: &Arc<Mutex<IoPin>>,
    sendor_door_pin_for_startup: &Arc<Mutex<InputPin>>
) {
    const MAX_RETRIES: u8 = 3;
    const RETRY_DELAY: Duration = Duration::from_secs(5);

    for attempt in 1..=MAX_RETRIES {
        match read_dht22(sensor_secondary_temp_pin) {
            Ok(Reading { temperature, humidity }) => {
                let temp_f = temperature * 9.0 / 5.0 + 32.0;
                log::info!(
                    "(Startup) DHT22 Reading: Temp: {:.2} °F, Humidity: {:.2} %",
                    temp_f,
                    humidity
                );

                // Sanity check
                if !( (-40.0..=125.0).contains(&temp_f) && (0.0..=100.0).contains(&humidity) ) {
                    log::warn!(
                        "(Startup) DHT22 reading out of range, skipping update: {:.2}°F, {:.2}%",
                        temp_f,
                        humidity
                    );
                }
                else if temp_f == 32.0 && humidity == 0.0 {
                    log::warn!(
                        "(Startup) DHT22 reading invalid (32.0°F, 0.0%), skipping update"
                    );  
                }
                else {
                    if let Err(error) = update_state_temp_f_humidity_and_notify_user(
                        startup_user.clone(),
                        read_shared_state(&sendor_door_pin_for_startup),
                        Some(temp_f),
                        Some(humidity),
                        Some(false)
                    ) {
                        log::error!(
                            "Panic on update_state_temp_f_humidity_and_notify_user {:?}",
                            error
                        );
                        panic!("Error: {:?}", error);
                    }
                }
                break;
            }

            Err(ReadingError::Timeout) => {
                log::warn!(
                    "(Startup) DHT22 timeout (GPIO18) — attempt {}/{}",
                    attempt,
                    MAX_RETRIES
                );
            }
            Err(ReadingError::Checksum) => {
                log::warn!(
                    "(Startup) DHT22 checksum error (GPIO18) — attempt {}/{}",
                    attempt,
                    MAX_RETRIES
                );
            }
            Err(ReadingError::Gpio(_e)) => {
                log::warn!(
                    "(Startup) DHT22 GPIO error (GPIO18): — attempt {}/{}",
                    attempt,
                    MAX_RETRIES
                );
            }
        }

        // Wait before retrying unless it's the last attempt
        if attempt < MAX_RETRIES {
            sleep(RETRY_DELAY).await;
        }
    }
}
