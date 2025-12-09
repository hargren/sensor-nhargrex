# sensor-nhargrex

sensor-nhargrex Rust program
(c) 2024 Nicholas Hargreaves

## Requires
```
export GOOGLE_APPLICATION_CREDENTIALS="/opt/.security/sensors-nhargrex-firebase-adminsdk-uev2w-11471882b8.json"
export GOOGLE_USER_ID="2U0LR6A8LER430Tq4tmdfAdl4iu2"
export GOOGLE_PROJECT_ID=sensors-nhargrex
export export FIREBASE_STORAGE_BUCKET="sensors-nhargrex.appspot.com"
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
```
## GCloud Untilites
```
gcloud init
gsutil ls gs://sensors-nhargrex.appspot.com/videos/2U0LR6A8LER430Tq4tmdfAdl4iu2 
gsutil cp gs://sensors-nhargrex.appspot.com/videos/2U0LR6A8LER430Tq4tmdfAdl4iu2/security-20251130-175019.mp4 C:\temp\FirebaseVideos\security-20251130-175019.mp4
```