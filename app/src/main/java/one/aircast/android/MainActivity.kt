package one.aircast.android

import android.content.Intent
import android.content.res.Configuration
import android.net.wifi.WifiManager
import android.os.Bundle
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
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
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
import one.aircast.android.ui.AnalyzePage
import one.aircast.android.ui.AnalyzeScreen
import one.aircast.android.ui.CameraControlLayer
import one.aircast.android.ui.FlightActions
import one.aircast.android.ui.ObstacleReadout
import one.aircast.android.ui.ParametersScreen
import one.aircast.android.ui.PlanTab
import one.aircast.android.ui.RcControlsLayer
import one.aircast.android.ui.SettingsScreen
import one.aircast.android.ui.SetupScreen
import one.aircast.android.ui.StatusStrip
import one.aircast.android.ui.VehicleTitle
import one.aircast.mapspike.FlyMap
import one.aircast.android.ui.VideoSourceLayer
import one.aircast.android.ui.VideoSurface
import org.mavlink.qgroundcontrol.QGCBridge
import org.mavlink.qgroundcontrol.QGCUsbSerialManager
import org.qtproject.qt.android.QtQmlStatus
import org.qtproject.qt.android.QtQuickView

private val VIDEO_INSET_WIDTH = 200.dp
private val VIDEO_INSET_HEIGHT = 112.dp

private const val QML_URI = "qrc:/qml/QGroundControl/MainWindow/AndroidHost.qml"
private const val QML_LIBRARY = "AircastQGC"
private const val QML_DEFAULT_PAGE = "fly"

private const val MULTICAST_LOCK_TAG = "Aircast"


enum class Tab(val label: String, val icon: ImageVector, val page: String) {
    Fly("Fly", Icons.Default.Home, "fly"),
    Plan("Plan", Icons.Default.Place, "plan"),
    Setup("Setup", Icons.Default.Build, "fly"),
    Analyze("Analyze", Icons.Default.Info, "fly"),
    Settings("Settings", Icons.Default.Settings, "fly");

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
    private var multicastLock: WifiManager.MulticastLock? = null
    private lateinit var quickView: QtQuickView

    @Suppress("unused")
    fun hideSplashScreen(duration: Int) = Unit

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

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

    override fun onDestroy() {
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
    var qmlReady by remember { mutableStateOf(false) }
    var controlsExpanded by remember { mutableStateOf(true) }
    var actionsHeightPx by remember { mutableIntStateOf(0) }
    var analyzePage by remember { mutableStateOf<AnalyzePage?>(null) }
    var videoExpanded by remember { mutableStateOf(false) }

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

    LaunchedEffect(Unit) {
        withContext(Dispatchers.Default) {
            // Native rendering first: initNative() creates the receivers and binds them, and a
            // receiver bound before this flag is set looks for a QtQuick item that does not exist.
            Qgc.invoke("video.setNativeRendering", true)
            Qgc.invoke("video.initNative")
        }
    }

    DisposableEffect(quickView) {
        val listeners = mutableListOf<Int>()
        quickView.setStatusChangeListener { status ->
            qmlReady = status == QtQmlStatus.READY
            if (qmlReady && listeners.isEmpty()) {
                quickView.setProperty("renderViews", false)
                listeners += quickView.connectSignalListener(
                    "navigateRequest",
                    String::class.java,
                ) { _, destination -> tab = Tab.from(destination ?: "") }
            }
        }
        onDispose { listeners.forEach { quickView.disconnectSignalListener(it) } }
    }

    var appliedPage by remember { mutableStateOf(QML_DEFAULT_PAGE) }

    LaunchedEffect(tab, qmlReady) {
        if (!qmlReady) return@LaunchedEffect
        if (appliedPage != tab.page) {
            quickView.setProperty("page", tab.page)
            appliedPage = tab.page
        }
    }

    DisposableEffect(Unit) {
        onDispose {
            appliedPage = QML_DEFAULT_PAGE
        }
    }

    MaterialTheme(colorScheme = darkColorScheme()) {
        Scaffold(
            snackbarHost = { SnackbarHost(snackbars) },
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
                if (tab == Tab.Fly) {
                    StatusStrip()
                }
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

                if (tab == Tab.Fly) {
                    Column(
                        Modifier
                            .align(Alignment.TopStart)
                            .padding(12.dp)
                            .padding(top = VIDEO_INSET_HEIGHT + 12.dp),
                        verticalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        ObstacleReadout()
                        VideoSourceLayer()
                        CameraControlLayer()
                        RcControlsLayer()
                    }
                }

                AnimatedVisibility(
                    visible = tab == Tab.Fly && controlsExpanded,
                    modifier = Modifier.align(Alignment.BottomCenter),
                ) {
                    Surface(
                        Modifier.fillMaxWidth().onSizeChanged { actionsHeightPx = it.height },
                        color = MaterialTheme.colorScheme.surface.copy(alpha = 0.92f),
                    ) { FlightActions() }
                }
            }
        }
    }
}
