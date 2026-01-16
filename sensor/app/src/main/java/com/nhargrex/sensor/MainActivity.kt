package com.nhargrex.sensor

import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.util.Log
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.viewModels
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ShowChart
import androidx.compose.material.icons.filled.Cloud
import androidx.compose.material.icons.filled.Garage
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Thermostat
import androidx.compose.material.icons.filled.VerifiedUser
import androidx.compose.material.icons.filled.WaterDrop
import androidx.compose.material.icons.filled.SdCard
import androidx.compose.material3.BottomAppBar
import androidx.compose.material3.BottomAppBarDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.FloatingActionButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults.topAppBarColors
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.media3.common.MediaItem
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.ui.PlayerView
import androidx.room.Dao
import androidx.room.Database
import androidx.room.Entity
import androidx.room.Index
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.PrimaryKey
import androidx.room.Query
import androidx.room.Room
import androidx.room.RoomDatabase
import androidx.sqlite.db.SupportSQLiteDatabase
import com.firebase.ui.auth.AuthUI
import com.firebase.ui.auth.FirebaseAuthUIActivityResultContract
import com.google.firebase.Firebase
import com.google.firebase.auth.FirebaseAuth
import com.google.firebase.firestore.ListenerRegistration
import com.google.firebase.firestore.firestore
import com.google.firebase.messaging.Constants.MessageNotificationKeys.TAG
import com.google.firebase.storage.FirebaseStorage
import com.nhargrex.sensor.ui.theme.SensorTheme
import com.nhargrex.sensor.utils.timeAgo
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.tasks.await

data class MainUiState(
    val userId: String = "—",
    val email: String = "—",
    val state: String = "—",
    val online: Boolean = false,
    val humidity: Double = 0.0,
    val temp: Double = 0.0,
    val version: String = "1.2916",
    val isSignedIn: Boolean = false,
    val isLoading: Boolean = false,
    val timestamp: Double = 0.0,
    val sampleCount: Int = 0,
    val minTemp: Double? = 0.0,
    val maxTemp: Double? = 0.0
)
data class SensorData(
    val state: String,
    val temp: Double,
    val humidity: Double,
    val online: Boolean,
    val timestamp: Double
)

class MainRepository {
    private val auth = FirebaseAuth.getInstance()
    private val firestore = Firebase.firestore
    fun currentUser() = auth.currentUser
    suspend fun refreshOnline(userId: String): Boolean {
        return try {
            val result = firestore
                .collection("sensors")
                .document(userId)
                .get()
                .await()

            result.getBoolean("online") ?: false
        } catch (e: Exception) {
            Log.d(TAG, "Error ${e.toString()}")
            false
        }
    }
    suspend fun requestStateRefresh(userId: String) {

        // This will tell the device to update the Firebase document
        // with the current state; update of the document will then
        // trigger the listener `observeState` which will update the UI
        firestore.collection("sensorsRefreshRequest")
            .document(userId)
            .set(
                mapOf(
                    "r_ts" to (System.currentTimeMillis() / 1000),
                    "r_cmd" to 1
                )
            )
            .await()
    }
    suspend fun getDocument(userId: String): SensorData {
        val doc = firestore.collection("sensors")
            .document(userId)
            .get()
            .await()
        val sensorData = SensorData(
            doc.getString("state") ?: "—",
            doc.getDouble("temp_f") ?: 0.0,
            doc.getDouble("humidity") ?: 0.0,
            doc.getBoolean("online") ?: false,
            doc.getDouble("timestamp") ?: 0.0
        )
        return sensorData
    }

    fun subscribeToState(
        userId: String,
        onUpdate: (SensorData) -> Unit
    ): ListenerRegistration {

        return firestore.collection("sensors")
            .document(userId)
            .addSnapshotListener { snapshot, _ ->
                val state = snapshot?.getString("state") ?: "—"
                val temp = snapshot?.getDouble("temp_f") ?: 0.0
                val humidity = snapshot?.getDouble("humidity") ?: 0.0
                val timestamp = snapshot?.getDouble("timestamp") ?: 0.0

                onUpdate(
                    SensorData(
                        state = state,
                        temp = temp,
                        humidity = humidity,
                        online = snapshot?.getBoolean("online") ?: false,
                        timestamp = timestamp
                    )
                )
            }
    }

    fun signOut() = auth.signOut()

    fun getLatestVideoUrl(userId: String, onResult: (String?) -> Unit) {
        val storage = FirebaseStorage.getInstance()
        val folderRef = storage.getReference("videos/$userId")

        folderRef.listAll()
            .addOnSuccessListener { list ->
                val latest = list.items
                    .map { it.name }
                    .filter { it.startsWith("security-") && it.endsWith(".mp4") }
                    .maxByOrNull { name ->
                        // Extract timestamp: security-YYYYMMDD-HHMMSS.mp4
                        val ts = name.removePrefix("security-").removeSuffix(".mp4")
                        ts.replace("-", "")
                    }

                if (latest == null) {
                    onResult(null)
                    return@addOnSuccessListener
                }

                folderRef.child(latest).downloadUrl
                    .addOnSuccessListener { uri -> onResult(uri.toString()) }
                    .addOnFailureListener { onResult(null) }
            }
            .addOnFailureListener { onResult(null) }
    }
}

class MainViewModel(
    private val dao: SensorHistoryDao,
    private val repo: MainRepository = MainRepository()
) : ViewModel() {

    private val _uiState = MutableStateFlow(MainUiState())
    val uiState: StateFlow<MainUiState> = _uiState.asStateFlow()

    private var stateListener: ListenerRegistration? = null

    init {
        loadInitialState()
        observeDatabaseStats()
    }

    private fun observeDatabaseStats() {
        viewModelScope.launch {
            dao.getCountFlow().collect { count ->
                _uiState.update { it.copy(sampleCount = count) }
            }
        }
        viewModelScope.launch {
            dao.getMinTempFlow().collect { min ->
                _uiState.update { it.copy(minTemp = min) }
            }
        }
        viewModelScope.launch {
            dao.getMaxTempFlow().collect { max ->
                _uiState.update { it.copy(maxTemp = max) }
            }
        }
    }

    fun loadInitialState() {
        val user = repo.currentUser()

        if (user == null) {
            _uiState.value = MainUiState(userId = "Signed out", isSignedIn = false)
            return
        }

        _uiState.update {
            it.copy(
                userId = user.uid,
                email = user.email ?: "—",
                isSignedIn = true
            )
        }

        observeState(user.uid)
        refreshState()
    }

    fun refreshState() {
        viewModelScope.launch {

            _uiState.update { it.copy(
                isLoading= true
            ) }

            val userId = repo.currentUser()?.uid ?: return@launch

            val online = repo.refreshOnline(userId)

            _uiState.update { it.copy(online = online) }

            // In MainViewModel inside refreshState()
            if (online) {
                repo.requestStateRefresh(userId)
                val d = repo.getDocument(userId)

                // Only update the Firestore data here.
                // The Room stats (count, min, max) will update
                // automatically via the collectors above.
                _uiState.update {
                    it.copy(
                        online = d.online,
                        state = d.state,
                        temp = d.temp,
                        humidity = d.humidity,
                        timestamp = d.timestamp
                    )
                }
            }
            _uiState.update { it.copy(isLoading = false) }
        }
    }

    // Inside MainViewModel
    override fun onCleared() {
        super.onCleared()
        stateListener?.remove() // Cleanup to prevent memory leaks
    }

    private fun observeState(userId: String) {
        stateListener?.remove()
        stateListener = repo.subscribeToState(userId) { u ->
            viewModelScope.launch {
                dao.insert(SensorHistoryEntity(
                    timestamp = u.timestamp,
                    temp = u.temp,
                    humidity = u.humidity
                ))
                _uiState.update {
                    it.copy(
                        online = u.online,
                        state = u.state,
                        temp = u.temp,
                        humidity = u.humidity,
                        timestamp = u.timestamp
                    )
                }
            }
        }
    }
    fun signOut() {
        stateListener?.remove()
        repo.signOut()

        _uiState.update {
            MainUiState(
                userId = "Signed out",
                email = "—",
                state = "—",
                temp = 0.0,
                humidity = 0.0,
                online = false,
                isSignedIn = false,
                isLoading = false,
                timestamp = 0.0,
                sampleCount = 0,
                minTemp = null,
                maxTemp = null
            )
        }
    }

    fun fetchLatestVideoUrl(onUrlReady: (String?) -> Unit) {
        val userId = _uiState.value.userId
        if (userId == "Signed out") {
            onUrlReady(null)
            return
        }

        repo.getLatestVideoUrl(userId) { url ->
            onUrlReady(url)
        }
    }
}

class MainActivity : ComponentActivity() {

    private val viewModel: MainViewModel by viewModels {
        val app = application as SensorApp
        MainViewModelFactory(
            dao = app.database.sensorHistoryDao(),
            repo = MainRepository()
        )
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        setContent {
            SensorTheme {
                val ui by viewModel.uiState.collectAsState()

                MessageCard(
                    ui = ui,
                    onRefresh = { viewModel.refreshState() },
                    onSignIn = { signInLauncher.launch(signInIntent) },
                    onSignOut = { viewModel.signOut() },
                    onPlayLatestVideo = { playLatestVideo() },
                    onShowHistory = { openHistoryChart() },
                    sampleStatsContent = {
                        SampleStatsCard(
                            count = ui.sampleCount,
                            minTemp = ui.minTemp,
                            maxTemp = ui.maxTemp
                        )
                    }
                )
            }
        }
    }

    fun playLatestVideo() {
        // Ask the ViewModel for the URL
        viewModel.fetchLatestVideoUrl { url ->
            if (url != null) {
                val intent = Intent(this, VideoPlayerActivity::class.java)
                intent.putExtra("videoUrl", url)
                startActivity(intent)
            } else {
                Toast.makeText(this, "No video found", Toast.LENGTH_SHORT).show()
            }
        }
    }

    fun openHistoryChart() {
        val intent = Intent(this, HistoryChartActivity::class.java)
        startActivity(intent)
    }

    private val signInLauncher = registerForActivityResult(
        FirebaseAuthUIActivityResultContract()
    ) { _ ->
        viewModel.loadInitialState()
    }

    private val providers = arrayListOf(
        AuthUI.IdpConfig.GoogleBuilder().build()
    )

    private val signInIntent = AuthUI.getInstance()
        .createSignInIntentBuilder()
        .setAvailableProviders(providers)
        .build()
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MessageCard(
    ui: MainUiState,
    onRefresh: () -> Unit,
    onSignIn: () -> Unit,
    onSignOut: () -> Unit,
    onPlayLatestVideo: () -> Unit,
    onShowHistory: () -> Unit,
    sampleStatsContent: @Composable () -> Unit
) {

    Scaffold(
        topBar = {
            var menuExpanded by remember { mutableStateOf(false) }

            TopAppBar(
                colors = topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer,
                    titleContentColor = MaterialTheme.colorScheme.primary,
                ),
                title = {
                    Text(stringResource(id = R.string.app_name))
                },
                actions = {
                    IconButton(onClick = { menuExpanded = true }) {
                        Icon(
                            imageVector = Icons.Default.MoreVert,
                            contentDescription = "Menu"
                        )
                    }

                    DropdownMenu(
                        expanded = menuExpanded,
                        onDismissRequest = { menuExpanded = false }
                    ) {
                        DropdownMenuItem(
                            text = {
                                Text(if (ui.isSignedIn) "Sign Out" else "Sign In")
                            },
                            onClick = {
                                if (ui.isSignedIn) onSignOut() else onSignIn()
                                menuExpanded = false
                            }
                        )
                    }
                }
            )
        },
        bottomBar = {
            BottomAppBar(
                actions = {
                    FloatingActionButton(
                        onClick = onRefresh,
                        containerColor = BottomAppBarDefaults.bottomAppBarFabColor,
                        elevation = FloatingActionButtonDefaults.bottomAppBarFabElevation()
                    ) {
                        Icon(Icons.Filled.Refresh, "Refresh")
                    }

                    Spacer(modifier = Modifier.width(8.dp))

                    FloatingActionButton(
                        onClick = onPlayLatestVideo,
                        containerColor = BottomAppBarDefaults.bottomAppBarFabColor,
                        elevation = FloatingActionButtonDefaults.bottomAppBarFabElevation()
                    ) {
                        Icon(Icons.Filled.PlayArrow, "Play Latest Video")
                    }

                    Spacer(modifier = Modifier.width(8.dp))

                    FloatingActionButton(
                        onClick = onShowHistory,
                        containerColor = BottomAppBarDefaults.bottomAppBarFabColor,
                        elevation = FloatingActionButtonDefaults.bottomAppBarFabElevation()
                    ) {
                        Icon(Icons.AutoMirrored.Filled.ShowChart, "History")
                    }

                    Spacer(modifier = Modifier.width(8.dp))

                    Text(
                        text = timeAgo(ui.timestamp),
                        style = MaterialTheme.typography.labelSmall,
                        modifier = Modifier
                            .padding(start = 8.dp)
                            .align(Alignment.CenterVertically)
                    )

                }
            )
        },
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .padding(innerPadding)
                .verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {

            Spacer(modifier = Modifier.width(8.dp))

            SensorCard(
                icon  = Icons.Default.VerifiedUser,
                label = stringResource(id = R.string.user),
                value = ui.email
            )

            SensorCard(
                icon = Icons.Default.Cloud,
                label = "Connected State",
                value = if (ui.online) "online" else "offline"
            )

            SensorCard(
                icon = Icons.Default.Garage,
                label = "State",
                value = ui.state
            )

            SensorCard(
                icon  = Icons.Default.Thermostat,
                label = "Temperature (°F)",
                value = if (ui.temp == 0.0) "—" else "%.2f".format(ui.temp)
            )

            SensorCard(
                icon = Icons.Default.WaterDrop,
                label = "Humidity (%)",
                value = if (ui.humidity == 0.0) "—" else "%.2f".format(ui.humidity)
            )

            // show sample stats card
            sampleStatsContent()
        }
    }
}

@Composable
fun SensorCard(
    icon: ImageVector,
    label: String,
    value: String,
    modifier: Modifier = Modifier
) {
    Card(
        modifier = modifier
            .fillMaxWidth()
            .padding(horizontal = 12.dp),
        shape = RoundedCornerShape(12.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 4.dp)
    ) {
        Row(
            modifier = Modifier
                .padding(16.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                imageVector = icon,
                contentDescription = label,
                tint = MaterialTheme.colorScheme.primary
            )
            Spacer(modifier = Modifier.width(12.dp))
            Column {
                Text(
                    text = label,
                    style = MaterialTheme.typography.titleMedium
                )
                Text(
                    text = value,
                    style = MaterialTheme.typography.bodyLarge,
                    fontWeight = FontWeight.Bold
                )
            }
        }
    }
}

@Composable
fun SampleStatsCard(
    count: Int,
    minTemp: Double?,
    maxTemp: Double?,
    modifier: Modifier = Modifier
) {
    Card(
        modifier = modifier
            .fillMaxWidth()
            .padding(horizontal = 12.dp),
        shape = RoundedCornerShape(12.dp),
        elevation = CardDefaults.cardElevation(defaultElevation = 4.dp),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.secondaryContainer
        )
    ) {
        Row (
            modifier = Modifier
            .padding(16.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                imageVector = Icons.Default.SdCard,
                contentDescription = "stats",
                tint = MaterialTheme.colorScheme.primary
            )
            Spacer(modifier = Modifier.width(12.dp))
            Column {
                Text(
                    text = "n: $count"
                )

                Text(text = "Min: ${minTemp?.let { "%.1f".format(it) } ?: "—"}°F")
                Text(text = "Max: ${maxTemp?.let { "%.1f".format(it) } ?: "—"}°F")
            }
        }
    }
}


@Composable
fun ExoVideoPlayer(videoUrl: String) {
    val context = LocalContext.current

    val exoPlayer = remember {
        ExoPlayer.Builder(context).build().apply {
            val mediaItem = MediaItem.fromUri(videoUrl)
            setMediaItem(mediaItem)
            prepare()
            playWhenReady = true
        }
    }

    DisposableEffect(Unit) {
        onDispose {
            exoPlayer.release()
        }
    }

    AndroidView(
        modifier = Modifier.fillMaxSize(),
        factory = { ctx ->
            PlayerView(ctx).apply {
                player = exoPlayer
                useController = true
            }
        }
    )
}

@Entity(
    tableName = "sensor_history",
    indices = [
        Index(value = ["timestamp"]),
        Index(value = ["temp"])
    ]
)
data class SensorHistoryEntity(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val timestamp: Double,
    val temp: Double,
    val humidity: Double
)

@Dao
interface SensorHistoryDao {

    @Query("SELECT COUNT(*) FROM sensor_history")
    fun getCountFlow(): Flow<Int>

    @Query("SELECT MIN(temp) FROM sensor_history")
    fun getMinTempFlow(): Flow<Double?>

    @Query("SELECT MAX(temp) FROM sensor_history")
    fun getMaxTempFlow(): Flow<Double?>

    @Query("""
        SELECT * FROM sensor_history
        ORDER BY timestamp DESC
        LIMIT 1024
    """)
    suspend fun getLatest1024(): List<SensorHistoryEntity>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insert(entity: SensorHistoryEntity)
}


@Database(
    entities = [SensorHistoryEntity::class],
    version = 1
)
abstract class AppDatabase : RoomDatabase() {

    abstract fun sensorHistoryDao(): SensorHistoryDao

    companion object {
        fun build(context: Context): AppDatabase {
            return Room.databaseBuilder(
                context,
                AppDatabase::class.java,
                "sensor.db"
            )
                .addCallback(object : Callback() {
                    override fun onCreate(db: SupportSQLiteDatabase) {
                        super.onCreate(db)

                        db.execSQL("""
                        CREATE TRIGGER IF NOT EXISTS prune_history_trigger
                        AFTER INSERT ON sensor_history
                        BEGIN
                            DELETE FROM sensor_history
                            WHERE id NOT IN (
                                SELECT id FROM sensor_history
                                ORDER BY timestamp DESC
                                LIMIT 1024
                            );
                        END;
                    """.trimIndent())
                    }
                })
                .build()
        }
    }
}

