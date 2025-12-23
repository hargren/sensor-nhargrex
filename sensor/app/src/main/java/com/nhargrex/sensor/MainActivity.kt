package com.nhargrex.sensor

import android.content.ContentValues.TAG
import android.content.Context
import android.os.Bundle
import android.util.Log
import android.view.View
import android.widget.Button
import android.widget.TextView
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
import kotlin.properties.Delegates

data class MainUiState(
    val userId: String = "--",
    val email: String = "--",
    val state: String = "--",
    val online: Boolean = false,
    val version: String = "1.2916",
    val isSignedIn: Boolean = false,
    val loading: Boolean = false
)

class MainRepository {

    private val auth = FirebaseAuth.getInstance()
    private val firestore = Firebase.firestore

    fun currentUser() = auth.currentUser

    suspend fun refreshOnline(): Boolean {
        return try {
            val result = firestore
                .collection("online")
                .document("status")
                .get()
                .await()

            result.getBoolean("online") ?: false
        } catch (e: Exception) {
            false
        }
    }

    suspend fun requestStateRefresh(userId: String) {
        firestore.collection("sensorsRefreshRequest")
            .document(userId)
            .set(
                mapOf(
                    "r_ts" to (System.currentTimeMillis() / 1000),
                    "r_cmd" to 0
                )
            )
            .await()
    }

    suspend fun getState(userId: String): String {
        val doc = firestore.collection("sensors")
            .document(userId)
            .get()
            .await()

        return doc.getString("state") ?: "--"
    }

    fun subscribeToState(userId: String, onUpdate: (String) -> Unit): ListenerRegistration {
        return firestore.collection("sensors")
            .document(userId)
            .addSnapshotListener { snapshot, _ ->
                val state = snapshot?.getString("state") ?: "--"
                onUpdate(state)
            }
    }

    fun signOut() = auth.signOut()
}

class MainViewModel(
    private val repo: MainRepository = MainRepository()
) : ViewModel() {

    private val _uiState = MutableStateFlow(MainUiState())
    val uiState: StateFlow<MainUiState> = _uiState.asStateFlow()

    private var stateListener: ListenerRegistration? = null

    init {
        loadInitialState()
    }

    fun loadInitialState() {
        val user = repo.currentUser()

        if (user == null) {
            _uiState.update {
                it.copy(
                    userId = "Signed out",
                    email = "--",
                    state = "--",
                    online = false,
                    isSignedIn = false
                )
            }
            return
        }

        _uiState.update {
            it.copy(
                userId = user.uid,
                email = user.email ?: "--",
                isSignedIn = true
            )
        }

        observeState(user.uid)
        refreshState()
    }

    fun refreshState() {
        viewModelScope.launch {
            _uiState.update { it.copy(loading = true, state = "--") }

            val online = repo.refreshOnline()

            _uiState.update { it.copy(online = online) }

            if (online) {
                val userId = repo.currentUser()?.uid ?: return@launch
                repo.requestStateRefresh(userId)
                delay(3000)
                val newState = repo.getState(userId)
                _uiState.update { it.copy(state = newState) }
            }

            _uiState.update { it.copy(loading = false) }
        }
    }

    private fun observeState(userId: String) {
        stateListener?.remove()
        stateListener = repo.subscribeToState(userId) { newState ->
            _uiState.update { it.copy(state = newState) }
        }
    }

    fun signOut() {
        stateListener?.remove()
        repo.signOut()

        _uiState.update {
            MainUiState(
                userId = "Signed out",
                email = "--",
                state = "--",
                online = false,
                isSignedIn = false
            )
        }
    }
}
