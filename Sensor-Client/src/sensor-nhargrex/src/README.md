# sensor-nhargrex

sensor-nhargrex Rust program
(c) 2024 Nicholas Hargreaves

## Requires
```
export GOOGLE_APPLICATION_CREDENTIALS="/opt/.security/<firebase-security.json>"
export GOOGLE_USER_ID="<userId>"
export GOOGLE_PROJECT_ID=<projectId>
export export FIREBASE_STORAGE_BUCKET="<projectId>.appspot.com"
```
## Build
```
sudo rm -f /tmp/sensor-nhargrex.log && cargo build && cargo run

tail -f /tmp/sensor-nhargrex.log
```
## Install
```
sudo systemctl stop sensor-nhargrex && \
sudo rm -f /tmp/sensor-nhargrex.log && \
cargo build --release && \
sudo systemctl daemon-reload && \
sudo systemctl start sensor-nhargrex && \
sudo systemctl status sensor-nhargrex
```
## To kill
```
ps -eaf | grep sensor | grep nhargrex |  grep -Pio1 'nhargre1\s+\d+' | sed -r s/nhargre1// | xargs kill -9
```
## Hardware
```
GND         --> 5
GPIO PIN 17 --> 6 -- used for interrupt on thread 1 and commands on thread 2
GPIO PIN 27 --> 7 -- used for DHT22 data pin
GPIO PIN 28 --> 8 -- used for DHT22 data pin
```
## GCloud Untilites
```
gcloud init
gsutil ls gs://sensors-nhargrex.appspot.com/videos/<userId> 
gsutil cp gs://sensors-nhargrex.appspot.com/videos/<userId>/security-20251130-175019.mp4
```