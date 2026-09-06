package one.aircast.android

import android.content.Intent
import android.content.res.Configuration
import android.net.wifi.WifiManager
import android.os.Bundle
import android.os.PowerManager
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Build
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.List
import androidx.compose.material.icons.filled.Place
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.view.WindowCompat
import one.aircast.android.bridge.Qgc
import one.aircast.android.ui.FlightActions
import one.aircast.android.ui.ParametersScreen
import one.aircast.android.ui.SettingsScreen
import one.aircast.android.ui.StatusStrip
import one.aircast.android.ui.VehicleTitle
import org.mavlink.qgroundcontrol.QGCBridge
import org.mavlink.qgroundcontrol.QGCUsbSerialManager
import org.qtproject.qt.android.QtQmlStatus
import org.qtproject.qt.android.QtQuickView

private const val QML_URI = "qrc:/qml/QGroundControl/MainWindow/AndroidHost.qml"
private const val QML_LIBRARY = "AircastQGC"
private const val WAKE_LOCK_TAG = "Aircast:screen"
private const val MULTICAST_LOCK_TAG = "Aircast"

private const val SETUP_QML = "qrc:/qml/QGroundControl/VehicleSetup/SetupView.qml"
private const val ANALYZE_QML = "qrc:/qml/QGroundControl/AnalyzeView/AnalyzeView.qml"

enum class Tab(val label: String, val icon: ImageVector, val page: String, val tool: String) {
    Fly("Fly", Icons.Default.Home, "fly", ""),
    Plan("Plan", Icons.Default.Place, "plan", ""),
    Setup("Setup", Icons.Default.Build, "fly", SETUP_QML),
    Params("Params", Icons.Default.List, "fly", ""),
    Analyze("Analyze", Icons.Default.Info, "fly", ANALYZE_QML),
    Settings("Settings", Icons.Default.Settings, "fly", "");

    companion object {
        fun from(destination: String) = when (destination.lowercase()) {
            "fly" -> Fly
            "plan" -> Plan
            "setup" -> Setup
            "parameters" -> Params
            "analyze" -> Analyze
            else -> Settings
        }
    }
}

class MainActivity : ComponentActivity(), QGCBridge.Host {
    private var wakeLock: PowerManager.WakeLock? = null
    private var multicastLock: WifiManager.MulticastLock? = null
    private lateinit var quickView: QtQuickView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        QGCBridge.setHost(this)
        Qgc.start()
        QGCUsbSerialManager.initialize(this)

        acquireWakeLock()
        acquireMulticastLock()
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)

        quickView = QtQuickView(this, QML_URI, QML_LIBRARY, arrayOf("qrc:/qml"))

        QGCBridge.notifyFontScale(resources.configuration.fontScale)
        QGCBridge.notifySafeAreaInsets(0, 0, 0, 0)
        intent?.data?.let { QGCBridge.notifyDeepLink(it.toString()) }

        setContent { AircastShell(quickView) }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        if (intent.action == Intent.ACTION_VIEW) {
            intent.data?.let { QGCBridge.notifyDeepLink(it.toString()) }
        }
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        QGCBridge.notifyFontScale(newConfig.fontScale)
    }

    override fun setSystemBarAppearance(lightBars: Boolean) {
        WindowCompat.getInsetsController(window, window.decorView).apply {
            isAppearanceLightStatusBars = lightBars
            isAppearanceLightNavigationBars = lightBars
        }
    }

    override fun onDestroy() {
        runCatching { QGCUsbSerialManager.cleanup(this) }
        multicastLock?.takeIf { it.isHeld }?.release()
        wakeLock?.takeIf { it.isHeld }?.release()
        super.onDestroy()
    }

    private fun acquireWakeLock() {
        val manager = getSystemService(PowerManager::class.java) ?: return
        wakeLock = manager.newWakeLock(PowerManager.SCREEN_BRIGHT_WAKE_LOCK, WAKE_LOCK_TAG)
            .also { runCatching { it.acquire() } }
    }

    private fun acquireMulticastLock() {
        val manager = applicationContext.getSystemService(WifiManager::class.java) ?: return
        multicastLock = manager.createMulticastLock(MULTICAST_LOCK_TAG).apply {
            setReferenceCounted(true)
            runCatching { acquire() }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AircastShell(quickView: QtQuickView) {
    var tab by remember { mutableStateOf(Tab.Fly) }
    var qmlReady by remember { mutableStateOf(false) }
    var controlsExpanded by remember { mutableStateOf(true) }

    DisposableEffect(quickView) {
        val listeners = mutableListOf<Int>()
        quickView.setStatusChangeListener { status ->
            qmlReady = status == QtQmlStatus.READY
            if (qmlReady && listeners.isEmpty()) {
                listeners += quickView.connectSignalListener(
                    "navigateRequest",
                    String::class.java,
                ) { _, destination -> tab = Tab.from(destination ?: "") }
            }
        }
        onDispose { listeners.forEach { quickView.disconnectSignalListener(it) } }
    }

    LaunchedEffect(tab, qmlReady) {
        if (!qmlReady) return@LaunchedEffect
        quickView.setProperty("toolSource", tab.tool)
        quickView.setProperty("page", tab.page)
    }

    MaterialTheme(colorScheme = darkColorScheme()) {
        Scaffold(
            topBar = {
              Column {
                TopAppBar(
                    title = { VehicleTitle() },
                    actions = {
                        if (tab == Tab.Fly) {
                            IconButton(onClick = { controlsExpanded = !controlsExpanded }) {
                                Icon(Icons.Default.Build, "Toggle flight controls")
                            }
                        }
                    },
                )
                StatusStrip()
              }
            },
            bottomBar = {
                NavigationBar {
                    Tab.entries.forEach { entry ->
                        NavigationBarItem(
                            selected = tab == entry,
                            onClick = { tab = entry },
                            icon = { Icon(entry.icon, entry.label) },
                            label = { Text(entry.label) },
                        )
                    }
                }
            },
        ) { padding ->
            Box(Modifier.padding(padding).fillMaxSize()) {
                AndroidView(factory = { quickView }, modifier = Modifier.fillMaxSize())

                when (tab) {
                    Tab.Settings -> Surface(Modifier.fillMaxSize()) { SettingsScreen() }
                    Tab.Params -> Surface(Modifier.fillMaxSize()) { ParametersScreen() }
                    else -> Unit
                }

                AnimatedVisibility(
                    visible = tab == Tab.Fly && controlsExpanded,
                    modifier = Modifier.align(Alignment.BottomCenter),
                ) {
                    Surface(
                        Modifier.fillMaxWidth(),
                        color = MaterialTheme.colorScheme.surface.copy(alpha = 0.92f),
                    ) { FlightActions() }
                }
            }
        }
    }
}
