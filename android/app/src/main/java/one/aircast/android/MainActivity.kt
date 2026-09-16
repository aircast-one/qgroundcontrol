package one.aircast.android

import android.content.Intent
import android.content.res.Configuration
import android.net.wifi.WifiManager
import android.os.Bundle
import android.view.Window
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Build
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material.icons.filled.Place
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.foundation.layout.Row
import one.aircast.android.ui.StatusReadingsInline
import one.aircast.android.ui.VehicleStateChip
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.zIndex
import androidx.compose.ui.unit.dp
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.view.WindowCompat
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.ui.hasVehicle
import one.aircast.mapspike.AircastTheme
import one.aircast.android.ui.AnalyzePage
import one.aircast.android.ui.AnalyzeScreen
import one.aircast.android.ui.CameraControlLayer
import one.aircast.android.ui.FlightActions
import one.aircast.android.ui.FollowMeReadout
import one.aircast.android.ui.ObstacleReadout
import one.aircast.android.ui.OrbitReadout
import one.aircast.android.ui.ParametersScreen
import one.aircast.android.ui.PlanTab
import one.aircast.android.ui.RcControlsLayer
import one.aircast.android.ui.SettingsScreen
import one.aircast.android.ui.SetupScreen
import one.aircast.android.ui.TrafficReadout
import one.aircast.android.ui.VehicleTitle
import one.aircast.mapspike.FlyMap
import one.aircast.android.ui.VideoSourceLayer
import one.aircast.android.ui.VideoSurface
import org.mavlink.qgroundcontrol.QGCBridge
import org.mavlink.qgroundcontrol.QGCUsbSerialManager
import org.qtproject.qt.android.QtQuickView
import org.qtproject.qt.android.QtRelaunchGuard

private val VIDEO_INSET_WIDTH = 200.dp
private val VIDEO_INSET_HEIGHT = 112.dp

private const val QML_URI = "qrc:/qml/QGroundControl/MainWindow/AndroidHost.qml"
private const val QML_LIBRARY = "AircastQGC"

private const val MULTICAST_LOCK_TAG = "Aircast"


internal fun reselectClearsAnalyze(current: Tab, tapped: Tab): Boolean =
    current == tapped && tapped == Tab.Analyze

enum class Tab(val label: String, val icon: ImageVector) {
    Fly("Fly", Icons.Default.Home),
    Plan("Plan", Icons.Default.Place),
    Setup("Setup", Icons.Default.Build),
    Analyze("Analyze", Icons.Default.Info),
    Settings("Settings", Icons.Default.Settings);

    companion object {
        fun from(destination: String) = when (destination.lowercase()) {
            "fly" -> Fly
            "plan" -> Plan
            "setup" -> Setup
            "analyze" -> Analyze
            else -> Settings
        }
    }
}

class MainActivity : ComponentActivity(), QGCBridge.Host {
    companion object {
        private var live: MainActivity? = null
    }

    private var multicastLock: WifiManager.MulticastLock? = null
    private lateinit var quickView: QtQuickView

    @Suppress("unused")
    fun hideSplashScreen(duration: Int) = Unit

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        live = this

        QGCBridge.setHost(this)
        Qgc.start()
        QGCUsbSerialManager.initialize(this)

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

    override fun getWindow(): Window =
        if (isDestroyed) live?.takeIf { it !== this }?.window ?: super.getWindow() else super.getWindow()

    override fun onDestroy() {
        if (live === this) live = null
        if (isChangingConfigurations) QtRelaunchGuard.forgetActivity(this)
        runCatching { QGCUsbSerialManager.cleanup(this) }
        multicastLock?.takeIf { it.isHeld }?.release()
        super.onDestroy()
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
    var controlsExpanded by remember { mutableStateOf(true) }
    var actionsHeightPx by remember { mutableIntStateOf(0) }
    var analyzePage by remember { mutableStateOf<AnalyzePage?>(null) }
    var popEpoch by remember { mutableIntStateOf(0) }
    var videoExpanded by remember { mutableStateOf(true) }

    val notices by one.aircast.android.bridge.qgcPath("host")
    val snackbars = remember { SnackbarHostState() }
    var acknowledgedThrough by remember { mutableLongStateOf(-1L) }
    val noticeScope = rememberCoroutineScope()
    var shownAt by remember { mutableStateOf(emptyMap<String, Long>()) }

    BackHandler(enabled = tab != Tab.Fly) { tab = Tab.Fly }

    LaunchedEffect(notices) {
        val queued = one.aircast.android.ui.noticesAfter(
            one.aircast.android.ui.hostNotices(notices),
            acknowledgedThrough,
        )
        if (queued.isEmpty()) return@LaunchedEffect
        val through = queued.last().id
        acknowledgedThrough = through
        one.aircast.android.ui.noticeDestination(queued)?.let { tab = Tab.from(it) }
        val now = System.currentTimeMillis()
        val banners = one.aircast.android.ui.bannersToShow(queued, shownAt, now)
        shownAt = shownAt + banners.associateWith { now }
        noticeScope.launch {
            withContext(Dispatchers.Default) {
                Qgc.invoke("host.acknowledgeThrough", through)
            }
            banners.forEach { snackbars.showSnackbar(it) }
        }
    }

    val vehiclesJson by one.aircast.android.bridge.qgcPath(one.aircast.mapspike.VEHICLES_VIEW)
    var lastVehicles by remember {
        mutableStateOf<one.aircast.mapspike.VehicleChoices?>(null)
    }

    LaunchedEffect(vehiclesJson) {
        val now = one.aircast.mapspike.vehicleChoices(vehiclesJson)
        val notice = one.aircast.mapspike.handoverNotice(lastVehicles, now, one.aircast.mapspike.VehicleBridge.lastAsked)
        if (one.aircast.mapspike.activeChanged(lastVehicles, now)) {
            one.aircast.mapspike.VehicleBridge.forget()
        }
        lastVehicles = one.aircast.mapspike.rememberedChoices(lastVehicles, now)
        notice?.let { said -> noticeScope.launch { snackbars.showSnackbar(said) } }
    }

    LaunchedEffect(Unit) {
        withContext(Dispatchers.Default) {
            Qgc.invoke("video.setNativeRendering", true)
            Qgc.invoke("video.initNative")
        }
    }

    AircastTheme {
        Scaffold(
            snackbarHost = { SnackbarHost(snackbars) },
            topBar = {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 12.dp, vertical = 6.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    VehicleStateChip()
                    if (tab == Tab.Fly) {
                        StatusReadingsInline(Modifier.weight(1f))
                    }
                }
            },
            bottomBar = {
                NavigationBar {
                    Tab.entries.forEach { entry ->
                        NavigationBarItem(
                            selected = tab == entry,
                            onClick = {
                                if (tab == entry) {
                                    if (reselectClearsAnalyze(tab, entry)) analyzePage = null
                                    popEpoch++
                                } else {
                                    tab = entry
                                }
                            },
                            icon = { Icon(entry.icon, entry.label) },
                            label = { Text(entry.label) },
                        )
                    }
                }
            },
        ) { padding ->
            Box(Modifier.padding(padding).fillMaxSize()) {
                AndroidView(factory = { quickView }, modifier = Modifier.fillMaxSize())

                if (tab == Tab.Fly) {
                    Box(
                        if (videoExpanded) {
                            Modifier
                                .align(Alignment.TopEnd)
                                .padding(12.dp)
                                .size(width = VIDEO_INSET_WIDTH, height = VIDEO_INSET_HEIGHT)
                                .zIndex(1f)
                        } else {
                            Modifier.fillMaxSize()
                        },
                    ) {
                        FlyMap(
                            modifier = Modifier.fillMaxSize(),
                            cameraBottomPx = if (videoExpanded || !controlsExpanded) 0 else actionsHeightPx,
                        )
                        if (videoExpanded) {
                            Box(
                                Modifier
                                    .matchParentSize()
                                    .clickable { videoExpanded = false },
                            )
                        }
                    }
                }

                VideoSurface(
                    modifier = if (videoExpanded) {
                        Modifier.fillMaxSize()
                    } else {
                        Modifier
                            .align(Alignment.TopEnd)
                            .padding(12.dp)
                            .size(width = VIDEO_INSET_WIDTH, height = VIDEO_INSET_HEIGHT)
                    },
                    expanded = videoExpanded,
                    onClick = { videoExpanded = !videoExpanded },
                )

                key(popEpoch) {
                    when (tab) {
                        Tab.Settings -> Surface(Modifier.fillMaxSize()) { SettingsScreen() }
                        Tab.Setup -> Surface(Modifier.fillMaxSize()) { SetupScreen() }
                        Tab.Plan -> Surface(Modifier.fillMaxSize()) { PlanTab() }
                        Tab.Analyze -> AnalyzeScreen(
                            page = analyzePage,
                            onSelect = { analyzePage = it },
                            modifier = Modifier.fillMaxSize(),
                        )
                        else -> Unit
                    }
                }

                if (tab == Tab.Fly) {
                    Column(
                        Modifier
                            .align(Alignment.TopStart)
                            .padding(12.dp)
                            .padding(top = VIDEO_INSET_HEIGHT + 12.dp),
                        verticalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        ObstacleReadout()
                        OrbitReadout()
                        FollowMeReadout()
                        TrafficReadout()
                        VideoSourceLayer()
                        CameraControlLayer()
                        RcControlsLayer()
                    }
                }

                if (tab == Tab.Fly) {
                    val controllable = hasVehicle()
                    Column(Modifier.align(Alignment.BottomCenter)) {
                        if (controllable) Surface(
                            Modifier.align(Alignment.CenterHorizontally),
                            color = MaterialTheme.colorScheme.surface.copy(alpha = 0.92f),
                            shape = MaterialTheme.shapes.small,
                        ) {
                            IconButton(onClick = { controlsExpanded = !controlsExpanded }) {
                                Icon(
                                    if (controlsExpanded) {
                                        Icons.Default.KeyboardArrowDown
                                    } else {
                                        Icons.Default.KeyboardArrowUp
                                    },
                                    if (controlsExpanded) "Hide flight controls" else "Show flight controls",
                                )
                            }
                        }
                        AnimatedVisibility(visible = controlsExpanded || !controllable) {
                            Surface(
                                Modifier.fillMaxWidth().onSizeChanged { actionsHeightPx = it.height },
                                color = MaterialTheme.colorScheme.surface.copy(alpha = 0.92f),
                            ) { FlightActions() }
                        }
                    }
                }
            }
        }
    }
}
