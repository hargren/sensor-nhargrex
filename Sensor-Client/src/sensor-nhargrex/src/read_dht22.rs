// Simple Rust wrapper that runs your existing Python DHT22 reader and forwards its output.
// This is the safest quick port: it reuses the working Python driver (adafruit_dht) but
// gives you a Rust program to run and control the process.

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

fn main() -> anyhow::Result<()> {
    // Path to your existing Python script
    let script = "/home/nhargre1/sensor-nhargrex/Sensor-Client/nhargrex/read_dht22.py";

    // Spawn the python script and capture stdout/stderr
    let mut child = Command::new("python3")
        .arg(script)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().expect("failed to capture stdout");
    let stderr = child.stderr.take().expect("failed to capture stderr");

    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    // Forward stderr in background
    let err_handle = thread::spawn(move || {
        for line in stderr_reader.lines().flatten() {
            eprintln!("{}", line);
        }
    });

    // Move child into Arc for ctrl-c handler
    let child_arc = Arc::new(Mutex::new(child));
    {
        let child_for_handler = child_arc.clone();
        ctrlc::set_handler(move || {
            if let Ok(mut ch) = child_for_handler.lock() {
                let _ = ch.kill();
            }
            std::process::exit(0);
        })?;
    }

    // Read stdout lines here and parse "temp,humidity" formatted responses
    for line_res in stdout_reader.lines() {
        match line_res {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let parts: Vec<&str> = trimmed.split(',').collect();
                if parts.len() >= 2 {
                    let temp_opt = parts[0].trim().parse::<f64>().ok();
                    let hum_opt = parts[1].trim().parse::<f64>().ok();
                    if let (Some(temp), Some(hum)) = (temp_opt, hum_opt) {
                        println!("Temp: {temp:.1}°F | Humidity: {hum:.1}%");
                        continue;
                    }
                }

                // Fallback: print raw line if it doesn't match expected format
                println!("{}", line);
            }
            Err(e) => {
                eprintln!("Error reading stdout: {}", e);
            }
        }
    }

    // Wait for stderr forwarding thread to finish
    err_handle.join().ok();

    Ok(())
}
