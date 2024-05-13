package com.nhargrex.sensor

import android.content.ContentValues.TAG
import android.content.Context
import android.os.Bundle
import android.util.Log
import android.view.View
import android.widget.TextView
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.lifecycle.lifecycleScope
import com.auth0.android.jwt.JWT
import com.firebase.ui.auth.AuthUI
import com.firebase.ui.auth.FirebaseAuthUIActivityResultContract
import com.firebase.ui.auth.data.model.FirebaseAuthUIAuthenticationResult
import com.google.firebase.Firebase
import com.google.firebase.FirebaseApp
import com.google.firebase.auth.FirebaseAuth
import com.google.firebase.auth.FirebaseUser
import com.google.firebase.firestore.FieldValue
import com.google.firebase.firestore.ListenerRegistration
import com.google.firebase.firestore.firestore
import com.google.firebase.messaging.messaging
import kotlinx.coroutines.launch
import kotlinx.coroutines.tasks.await


class MainActivity : ComponentActivity() {

    private val versionNumber: String = "1.2916"
    private lateinit var auth: FirebaseAuth
    private lateinit var userId: String
    private lateinit var state: String
    private lateinit var email: String
    private lateinit var online: String
    private lateinit var updateSubscriber : ListenerRegistration

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        userId = getString(R.string.unknown_msg)
        state = getString(R.string.unknown_msg)
        online = false.toString()

        setContentView(R.layout.activity_main)

        versionNumber.setVersionNumber()

        FirebaseApp.initializeApp(this)

        auth = FirebaseAuth.getInstance()

        if (auth.currentUser == null) {
            // User is not signed in so:
            // set the state and user id to unknown and
            // the control button to sign in
            this.userId = getString(R.string.signed_out)
            setState(getString(R.string.unknown_msg))
            setUserId(this.userId)
            setEmail(this.email)
            setOnline(false)
        } else {
            // User is signed in so:
            // show the userId, get and store in firestore the FCM token
            // update the state from firebase document and
            // subscribe to real-time updates of state
            this.userId = getString(R.string.signed_in)
            getIdToken(auth.currentUser)
            updateState()
            setOnline(false)
            lifecycleScope.launch {
                // this is async/suspend operation
                updateOnline()
            }
            updateSubscriber = subscribeToStateUpdates()
        }

        setSignInButtonText(auth.currentUser).setOnClickListener { _: View? ->
            if (auth.currentUser != null) {
                AuthUI.getInstance()
                    .signOut(this)
                    .addOnCompleteListener {
                        setSignInButtonText(auth.currentUser)
                        setUserId(getString(R.string.signed_out))
                        setEmail(getString(R.string.unknown_msg))
                        setState(getString(R.string.unknown_msg))
                        setOnline(false)
                        updateSubscriber.remove()
                    }
            } else {
                signInLauncher.launch(signInIntent)
            }
        }
    }

    private val signInLauncher = registerForActivityResult(
        FirebaseAuthUIActivityResultContract()
    ) { res ->
        this.onSignInResult(res)
    }

    private val providers = arrayListOf(
        AuthUI.IdpConfig.GoogleBuilder().build()
    )

    // Create and launch sign-in intent
    private val signInIntent = AuthUI.getInstance()
        .createSignInIntentBuilder()
        .setAvailableProviders(providers)
        .build()

    private fun onSignInResult(result: FirebaseAuthUIAuthenticationResult) {
        val response = result.idpResponse
        if (result.resultCode == RESULT_OK) {
            // Successfully signed in
            setSignInButtonText(auth.currentUser)
            if (auth.currentUser != null) {
                getIdToken(auth.currentUser)
                updateState()
                lifecycleScope.launch {
                    // this is async/suspend operation
                    updateOnline()
                }
                updateSubscriber = subscribeToStateUpdates()
            }
        } else {
            // Sign in failed. If response is null the user canceled the
            // sign-in flow using the back button. Otherwise check
            // response.getError().getErrorCode() and handle the error.
            this.userId = getString(R.string.signed_out)
            setState(getString(R.string.unknown_msg))
            setEmail(getString(R.string.unknown_msg))
            setOnline(false)
            setUserId(this.userId)
            Log.i(TAG, "Login failed - $response.getError().getErrorCode()")
        }
    }

    private fun setSignInButtonText(user: FirebaseUser?): TextView {
        val signIn = findViewById<View>(R.id.signIn) as TextView
        if (user == null) signIn.text = getString(R.string.sign_in) else signIn.text = getString(R.string.sign_out)
        return signIn
    }

    private fun updateState() {

        val currentUser = FirebaseAuth.getInstance().currentUser

        currentUser?.let {
            val userId = currentUser.uid
            Firebase.firestore
                .collection("sensors")
                .document(userId)
                .get()
                .addOnSuccessListener { document ->
                    setState((document.data?.get("state") ?: "--").toString())
                }
        }
    }

    private suspend fun updateOnline() {

        val currentUser = FirebaseAuth.getInstance().currentUser

        currentUser?.let {
            val userId = currentUser.uid

            // get the sensor document
            val sensorDocument = Firebase.firestore
                .collection("sensors")
                .document(userId)
                .get()
                .await()

            // set online to false
            sensorDocument.data?.set("online", false)

            val updatedSensorDocument = hashMapOf(
                "online" to false,
                "state" to sensorDocument.data?.get("state")
            )

            // write the document
            Firebase.firestore
                .collection("sensors")
                .document(userId)
                .set(updatedSensorDocument)
                .await()

            // send command to device to request it to update online state as device might be offline
            val sensorRefreshRequest = hashMapOf(
                "r_ts" to (System.currentTimeMillis() / 1000),
                "r_cmd" to 1
            )

            Log.i(TAG, "Sensor Refresh Request Document - $sensorRefreshRequest")

            Firebase.firestore
                .collection("sensorsRefreshRequest")
                .document(userId)
                .set(sensorRefreshRequest)
                .await()

            // wait for device to update the online state - if it doesn't we will say it is offline
            // TODO: make this a listen for change
            Thread.sleep(5000)

            val sensor = Firebase.firestore
                .collection("sensors")
                .document(userId)
                .get()
                .await()

            val online = sensor.data?.get("online")

            Log.i(TAG, "Sensor Data Document - online=$online")

            setOnline(online as Boolean?)
        }
    }

    private fun subscribeToStateUpdates() : ListenerRegistration {
        val db = Firebase.firestore
        val path = auth.currentUser?.uid

        val docRef = db.collection("sensors").document(path!!)

        val unsubscribeToUpdates =
            docRef.addSnapshotListener { querySnapshot, firebaseFirestoreException ->
                firebaseFirestoreException?.let {
                    Toast.makeText(this, it.message, Toast.LENGTH_SHORT).show()
                    return@addSnapshotListener
                }
                querySnapshot?.let {
                    setState((querySnapshot.data?.get("state") ?: "--").toString())
                }
            }
        return unsubscribeToUpdates

    }

    private fun getIdToken(user: FirebaseUser?) {
        user?.getIdToken(true)?.addOnSuccessListener { res ->
            val idToken = res.token!!
            val jwt = JWT(idToken)
            setUserId(jwt.getClaim("user_id").asString())
            setEmail(jwt.getClaim("email").asString())
            lifecycleScope.launch {
                // Get new FCM registration token
                val token = getAndStoreRegistrationToken()
                val msg = getString(R.string.msg_token_fmt, token)
                Toast.makeText(baseContext, msg, Toast.LENGTH_SHORT).show()
            }
        }
    }

    private fun String.setVersionNumber() {
        val textView: TextView = findViewById<View>(R.id.version) as TextView
        textView.text = getString(R.string.version, this)
    }

    private fun setUserId(userId: String?) {
        this.userId = userId ?: "--"
        val textView = findViewById<View>(R.id.user) as TextView
        textView.text = getString(R.string.user, this.userId)
    }

    private fun setEmail(email: String?) {
        this.email = email ?: "--"
        val textView = findViewById<View>(R.id.email) as TextView
        textView.text = getString(R.string.email, this.email)
    }

    private fun setOnline(online: Boolean?) {
        this.online = online.toString()
        val textView = findViewById<View>(R.id.online) as TextView
        textView.text = getString(R.string.online, this.online)
    }

    private fun setState(state: String?) {
        this.state = state ?: getString(R.string.unknown_msg)
        val textView = findViewById<View>(R.id.state) as TextView
        textView.text = getString(R.string.state, this.state)
    }

    private suspend fun getAndStoreRegistrationToken(): String {
        // [START log_reg_token]
        val token = Firebase.messaging.token.await()

        // Check whether the retrieved token matches the one on your server for this user's device
        val preferences = this.getPreferences(Context.MODE_PRIVATE)
        val tokenStored = preferences.getString("deviceToken", "")
        lifecycleScope.launch {
            if ((tokenStored == "" || tokenStored != token) && (userId != "--" || userId != getString(R.string.signed_out)))
            {
                // Add token and timestamp to Firestore for this user
                val deviceToken = hashMapOf(
                    "token" to token,
                    "timestamp" to FieldValue.serverTimestamp(),
                )

                // Get user ID from Firebase Auth or your own server
                Firebase.firestore
                    .collection("fcmTokens")
                    .document(userId)
                    .set(deviceToken)
                    .await()
            }
        }
        // [END log_reg_token]

        return token
    }
}


