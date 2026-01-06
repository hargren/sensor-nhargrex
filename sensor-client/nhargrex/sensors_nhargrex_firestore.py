""" Client side for sensor firestore update and notification via firebase cloud messaging
# Windows:
#   $env:GOOGLE_APPLICATION_CREDENTIALS="firebase-adminsdk.json"
# Linux:
#   export GOOGLE_APPLICATION_CREDENTIALS="/opt/.security/sensors-nhargrex-firebase-adminsdk-uev2w-11471882b8.json"
#   export GOOGLE_USER_ID="2U0..."
#
# Send notification entry point
# -- update_state_and_notify_user(user, state)
# -- >>> from sensors_nhargrex_firestore import update_state_and_notify_user
# -- >>> update_state_and_notify_user('2U0...', 'open')
# -- returns 0 if Ok
# -- returns 1 if Error
# Update temperature and humidity entry point
# -- update_temp_and_humidity(user, temp_f, humidity)
# -- >>> from sensors_nhargrex_firestore import update_temp_and_humidity
# -- >>> update_temp_and_humidity('2U0...', 72.5, 45.0)
# -- returns 0 if Ok
# -- returns 1 if Error
"""
import json
import logging
import os
import subprocess
import firebase_admin
import requests
import google.auth.transport.requests
from google.oauth2 import service_account
import firebase_admin
import re
import os
import time
import time
import pyrebase
from firebase_admin import firestore
from firebase_admin import credentials as firebase_credentials
from firebase_admin import firestore, storage
from picamera2 import Picamera2
from google.cloud import pubsub_v1

logging.basicConfig(level=logging.INFO)

PROJECT_ID = 'sensors-nhargrex'
BASE_URL = 'https://fcm.googleapis.com'
FCM_ENDPOINT = 'v1/projects/' + PROJECT_ID + '/messages:send'
FCM_URL = BASE_URL + '/' + FCM_ENDPOINT
SCOPES = ['https://www.googleapis.com/auth/firebase.messaging']

PUBSUB_TOPIC_ID = "sensor-data" # Replace with your desired Pub/Sub topic name
PUBSUB_PUBLISHER = pubsub_v1.PublisherClient()
PUBSUB_TOPIC_PATH = PUBSUB_PUBLISHER.topic_path(PROJECT_ID, PUBSUB_TOPIC_ID)

def _get_access_token(credentials_file):
# [START retrieve_access_token]
  credentials = service_account.Credentials.from_service_account_file(credentials_file, scopes=SCOPES)
  request = google.auth.transport.requests.Request()
  credentials.refresh(request)
  return credentials.token
# [END retrieve_access_token]

def _send_fcm_message(fcm_message):
  headers = {
    'Authorization': 'Bearer ' + _get_access_token(os.environ["GOOGLE_APPLICATION_CREDENTIALS"]),
    'Content-Type': 'application/json; UTF-8',
  }
  resp = requests.post(FCM_URL, data=json.dumps(fcm_message), headers=headers)

  if resp.status_code == 200:
    print('Message sent to Firebase for delivery, response:')
    print(resp.text)
  else:
    raise RuntimeError('Unable to send message to Firebase')

def _build_message(token, message_body):
  return {
    'message': {
      'token' : token,
      'notification': {
        'title': 'Sensor Notification',
        'body': message_body
      }
    }
  }

def _ensure_firebase_app():
    """
    Return the default firebase app, initializing it if necessary.
    Normalize FIREBASE_STORAGE_BUCKET env (strip 'gs://' if present).
    Raises a clear RuntimeError if initialization fails.
    """
    try:
        return firebase_admin.get_app()
    except ValueError:
        # Not initialized yet — try to initialize
        cred_path = os.environ.get("GOOGLE_APPLICATION_CREDENTIALS")
        raw_bucket = os.environ.get("FIREBASE_STORAGE_BUCKET")
        if not raw_bucket:
            bucket_name = f"{PROJECT_ID}.appspot.com"
            logging.info(f"FIREBASE_STORAGE_BUCKET not set; using default bucket {bucket_name!r}")
        else:
            bucket_name = raw_bucket[len("gs://"):] if raw_bucket.startswith("gs://") else raw_bucket
            logging.info(f"Using FIREBASE_STORAGE_BUCKET={bucket_name!r}")

        try:
            if cred_path:
                logging.info("Initializing firebase_admin with credentials at %r", cred_path)
                cred = firebase_credentials.Certificate(cred_path)
                app = firebase_admin.initialize_app(cred, options={"storageBucket": bucket_name})
            else:
                logging.info("Initializing firebase_admin without explicit credentials (will use ADC if available)")
                app = firebase_admin.initialize_app(options={"storageBucket": bucket_name})
            logging.info("Firebase app initialized: %r", app)
            return app
        except Exception as init_exc:
            logging.exception("Failed to initialize Firebase app")
            # Raise a clear error so callers see the root cause instead of a later ValueError
            raise RuntimeError("Failed to initialize Firebase app") from init_exc

def _firestore_upload_video_to_storage(user, filename):
    try:
        app = _ensure_firebase_app()
        bucket = storage.bucket(app=app)
        logging.info(f"Uploading video {os.path.basename(filename)!r} to Storage bucket {bucket.name!r} for user {user!r}")
        blob = bucket.blob(f'videos/{user}/{os.path.basename(filename)}')

        # Ensure correct content-type so browsers / console can download properly
        blob.upload_from_filename(filename, content_type='video/mp4')

        # Reload metadata and log useful fields
        blob.reload()
        logging.info("Upload complete: %r -> gs://%s/%s (size=%s content_type=%s)",
                     filename, bucket.name, blob.name, blob.size, blob.content_type)
    except Exception:
        logging.exception("Failed to upload video to Storage")

def _firestore_add_data(state, user, temp_f=None, humidity=None):
    db = firestore.client()

    # [START add_data]
    doc_ref = db.collection("sensors").document(user)

    if (temp_f is not None and humidity is not None):
      doc_ref.set({
          "state": state,
          "online" : True,
          "temp_f": temp_f,
          "humidity": humidity,
          "timestamp": time.time()
      })
    else:
      doc_ref.set({
          "state": state,
          "online" : True,
      })
    # [END add_data]

def _firestore_read_data(user):
    db = firestore.client()

    # [START read_data]
    doc_ref = db.collection('fcmTokens').document(user)
    doc = doc_ref.get()
    if doc.exists:     
        return doc.to_dict()
    raise Exception('Document does not exist.') 
    # [END read_data]

def _firestore_read_state(user):
    db = firestore.client()

    # [START read_data]
    doc_ref = db.collection('sensors').document(user)
    doc = doc_ref.get()
    if doc.exists:     
        return doc.to_dict()
    raise Exception('Document does not exist.') 
    # [END read_data]

def _validate_user(user):
  pattern = re.compile('[A-Za-z0-9]+')
  if (pattern.match(user) == None or len(user) != 28):
    raise ValueError('Invalid user: user must match validation pattern.') 

def _validate_state(state):
  match state:
    case "open":
      state = "open"
    case "closed":
      state = "closed"
    case _:
      raise ValueError('Invalid state: valid states are [open|closed].')    

def _validate_temp(temp_f):
  if (type(temp_f) is not float or temp_f < -40 or temp_f > 125):
    raise ValueError('Invalid temperature: valid range is -40 to 125 F.') 

def _validate_humidity(humidity):
  if (type(humidity) is not float or humidity < 0.0 or humidity > 100.0):
    logging.info(f"_validate_humidity: invalid humidity value: {humidity!r}")
    raise ValueError('Invalid humidity: valid range is 0 to 100%.]') 
  
def _validate_timestamp(timestamp):
  if (type(timestamp) is not int or timestamp < 0 ):
    logging.info(f"_validate_timestamp: invalid timestamp: {timestamp!r}")
    raise ValueError('Invalid timestamp: valid range is 0 and up.') 

def _capture_video(filename_base, capture_time):
    """
    Record to a temporary .h264 file, remux to .mp4 (without re-encoding),
    remove the temporary file and return the final .mp4 path.
    `filename_base` is the path *without* extension (e.g. '/tmp/security-20251130-163439').
    """
    tmp_h264 = filename_base + ".h264"
    out_mp4 = filename_base + ".mp4"

    try:
        picam2 = Picamera2()
        picam2.start_and_record_video(tmp_h264, duration=capture_time)
        picam2.close()
    except Exception:
        logging.exception("Failed to capture video with Picamera2")
        try:
            if os.path.exists(tmp_h264):
                os.remove(tmp_h264)
        except Exception:
            logging.exception("Failed to remove tmp file after capture failure")
        raise

    try:
        completed = subprocess.run(
            ["ffmpeg", "-y", "-i", tmp_h264, "-c:v", "copy", out_mp4],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        logging.info("Remuxed video to %r", out_mp4)
    except subprocess.CalledProcessError as e:
        logging.error("ffmpeg remux failed: returncode=%s", e.returncode)
        logging.error("ffmpeg stdout: %s", e.stdout)
        logging.error("ffmpeg stderr: %s", e.stderr)
        # keep tmp_h264 for inspection, but re-raise for caller to handle
        raise
    finally:
        try:
            if os.path.exists(tmp_h264):
                os.remove(tmp_h264)
        except Exception:
            logging.exception("Failed to remove tmp file %r", tmp_h264)

    return out_mp4

#
# Send notification entry point
# -- update_state_and_notify_user(user, state)
# -- >>> from sensors_nhargrex_firestore import updateStateAndNotifyUser
# -- >>> update_state_and_notify_user('2U0...', 'open', '71.0', '41.0', True)
# -- returns 0 if Ok
# -- returns 1 if Error
def update_state_and_notify_user(user, state, temp_f=None, humidity=None, force_notify=None):
    logging.info(f"update_state_and_notify_user called: user={user!r}, state={state!r}, temp={temp_f!r}, humidity={humidity!r}, force={force_notify!r}")
    logging.info(f"ENV GOOGLE_APPLICATION_CREDENTIALS={os.environ.get('GOOGLE_APPLICATION_CREDENTIALS')!r}, GOOGLE_USER_ID={os.environ.get('GOOGLE_USER_ID')!r}")
    try:
        if (temp_f is None) or (humidity is None):
            logging.info("No temp/humidity provided; skipping update")
            return 0
        else:
            _validate_temp(temp_f)
            _validate_humidity(humidity) 
        
        _validate_user(user)
        _validate_state(state)
        _ensure_firebase_app()

        # read current state and other fields
        firestore_state = _firestore_read_state(user)
        state_in_cloud = firestore_state.get("state")

        if (state != state_in_cloud or (force_notify is not None and force_notify == True)) and (temp_f is not None and humidity is not None):
            if state == "open":
              logging.info("State changed to 'open'; capturing video")
              timestr = time.strftime("%Y%m%d-%H%M%S")
              # cleanup old files
              for f in os.listdir("/tmp"):
                  if f.startswith("security-") and f.endswith(".mp4"):
                      try:
                          os.remove(os.path.join("/tmp", f))
                      except Exception:
                          logging.exception("Failed to remove old tmp file")

              filename_base = os.path.join("/tmp", f"security-{timestr}")
              # capture returns the final .mp4 path
              filename_mp4 = _capture_video(filename_base, capture_time=5)
              logging.info(f"Captured video to {filename_mp4!r}")

              try:
                  _firestore_upload_video_to_storage(user, filename_mp4)
              except Exception:
                  logging.exception("Video upload failed (continuing)")
            if (force_notify is not None and force_notify == True):
              logging.info("Force notify is True; updating Firestore and sending notification")
            else:
              logging.info(f"State changed from {state_in_cloud!r} to {state!r}; updating Firestore and sending notification")
            _firestore_add_data(state, user, temp_f, humidity)
            message_string = f"Door: {state}, Temp: {round(temp_f)}\u00B0F, Humidity: {round(humidity)}%"
            _send_fcm_message(_build_message(_firestore_read_data(user)["token"], message_string))
        else:
          logging.info("State unchanged; skipping update and notification")
        return 0

    except Exception as e:
        # log the full exception and re-raise so callers (Rust/pyo3) get the traceback
        logging.exception("update_state_and_notify_user error")
        raise

def update_temp_and_humidity(user, temp_f, humidity):
  try:
    _validate_user(user)
    _validate_temp(temp_f)
    _validate_humidity(humidity)
    _ensure_firebase_app()
    _firestore_add_data(_firestore_read_state(user)["state"], user, temp_f, humidity)
  except ValueError as e:
    return 1
  except RuntimeError as e:
    return 1
  return 0

def publish_temp_and_humidity(user, temp_f, humidity):
  logging.info(f"publish_temp_and_humidity called: user={user!r}, temp={temp_f!r}, humidity={humidity!r}")
  try:
    _validate_user(user)
    _validate_temp(temp_f)
    _validate_humidity(humidity)
    _ensure_firebase_app()
    message_data = {
        "user": user,
        "temp_f": temp_f,
        "humidity": humidity,
        "timestamp": time.time() # Add a timestamp for time-series analysis
    }
    data_bytes = json.dumps(message_data).encode("utf-8")
    future = PUBSUB_PUBLISHER.publish(PUBSUB_TOPIC_PATH, data_bytes)
    message_id = future.result() # This will block until the message is published
    logging.info(f"Published message with ID: {message_id} to topic: {PUBSUB_TOPIC_ID}")
  except ValueError as e:
    return 1
  except RuntimeError as e:
    return 1
  return 0


def test_python_integration(user, state):
  try:
    _validate_user(user)
    _validate_state(state)
  except ValueError as e:
    return 1
  except RuntimeError as e:
    return 1
  return 0

