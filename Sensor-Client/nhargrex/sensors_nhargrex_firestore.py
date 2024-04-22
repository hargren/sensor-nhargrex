""" Client side for sensor firestore update and notification via firebase cloud messaging
# Windows:
#   $env:GOOGLE_APPLICATION_CREDENTIALS="firebase-adminsdk.json"
# Linux:
#   export GOOGLE_APPLICATION_CREDENTIALS="/opt/.security/sensors-nhargrex-firebase-adminsdk-uev2w-11471882b8.json"
#   export GOOGLE_USER_ID="2U0LR6A8LER430Tq4tmdfAdl4iu2"
#
# Send notification entry point
# -- update_state_and_notify_user(user, state)
# -- >>> from sensors_nhargrex_firestore import update_state_and_notify_user
# -- >>> update_state_and_notify_user('X2U0LR6A8LER430Tq4tmdfAdl4iu2', 'open')
# -- returns 0 if Ok
# -- returns 1 if Error
"""
import json
import requests
import google.auth.transport.requests
import firebase_admin
import re
import os
from firebase_admin import firestore
from google.oauth2 import service_account
from firebase_admin import firestore

PROJECT_ID = 'sensors-nhargrex'
BASE_URL = 'https://fcm.googleapis.com'
FCM_ENDPOINT = 'v1/projects/' + PROJECT_ID + '/messages:send'
FCM_URL = BASE_URL + '/' + FCM_ENDPOINT
SCOPES = ['https://www.googleapis.com/auth/firebase.messaging']


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

def _build_message(token, state):
  return {
    'message': {
      'token' : token,
      'notification': {
        'title': 'Sensor Notification',
        'body': state
      }
    }
  }

def _firestore_add_data(state):
    db = firestore.client()

    # [START add_data]
    doc_ref = db.collection("state").document("garage")
    doc_ref.set({
        "state": state,
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

#
# Send notification entry point
# -- update_state_and_notify_user(user, state)
# -- >>> from sensors_nhargrex_firestore import updateStateAndNotifyUser
# -- >>> update_state_and_notify_user('2U0LR6A8LER430Tq4tmdfAdl4iu2', 'open')
# -- returns 0 if Ok
# -- returns 1 if Error
def update_state_and_notify_user(user, state):
  try:
    _validate_user(user)
    _validate_state(state)
    app = firebase_admin.initialize_app()
    _firestore_add_data(state)
    _send_fcm_message(_build_message(_firestore_read_data(user)["token"], state))
    firebase_admin.delete_app(app)
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