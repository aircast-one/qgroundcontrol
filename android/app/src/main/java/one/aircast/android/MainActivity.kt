package one.aircast.android

import android.content.Intent
import android.content.res.Configuration
import android.net.wifi.WifiManager
import android.os.Bundle
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.Window
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.lifecycle.lifecycleScope
import androidx.activity.compose.BackHandler
import one.aircast.android.ui.videoReading
import one.aircast.android.ui.VIDEO_VIEW
import one.aircast.android.bridge.qgcPath
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Build
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.Place
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.Row
import androidx.compose.material3.NavigationRailItem
import androidx.compose.material3.NavigationRail
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.ui.graphics.Color
import one.aircast.mapspike.aircast
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.movableContentOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.foundation.layout.WindowInsets
import one.aircast.android.ui.AppFontScale
import one.aircast.android.ui.AppNavigation
import one.aircast.android.ui.OverlayEditBar
import one.aircast.android.ui.Hideable
import one.aircast.android.ui.LogReplayBar
import one.aircast.android.ui.StatusReadingsInline
import one.aircast.android.ui.VehicleStateChip
import one.aircast.android.ui.ControlRequestPrompt
import one.aircast.android.ui.VtolStateCell
import one.aircast.android.ui.ResumeFailedPrompt
import one.aircast.android.ui.VirtualJoystick
import androidx.compose.ui.Modifier
import androidx.compose.ui.zIndex
import androidx.compose.ui.unit.dp
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.view.WindowCompat
import android.app.Activity
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.SideEffect
import androidx.compose.ui.platform.LocalView
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
import one.aircast.android.ui.AttitudeInstrument
import one.aircast.android.ui.MapClickMenu
import one.aircast.android.ui.MapPoint
import one.aircast.android.ui.FirstRunDialog
import one.aircast.android.ui.MissionCompleteDialog
import one.aircast.android.ui.SetWaypointSheet
import one.aircast.android.ui.RoiSheet
import one.aircast.android.ui.ObstacleArc
import one.aircast.android.ui.ObstacleReadout
import one.aircast.android.ui.TerrainProgress
import one.aircast.android.ui.OrbitReadout
import one.aircast.android.ui.PlanTab
import one.aircast.android.ui.RcControlsLayer
import one.aircast.android.ui.SettingsScreen
import one.aircast.android.ui.SetupScreen
import one.aircast.android.ui.TrafficReadout
import one.aircast.mapspike.FlyMap
import one.aircast.mapspike.TrackPoint
import one.aircast.android.ui.VideoSourceLayer
import one.aircast.android.ui.VideoSurface
import one.aircast.android.ui.FlyScreen
import one.aircast.android.ui.PinnedEmergencyStop
import one.aircast.android.ui.flyIsPortrait
import org.mavlink.qgroundcontrol.QGCBridge

private val KEY_ROW_STOP_GAP = 24.dp

private val VIRTUAL_JOYSTICK_BOTTOM_MARGIN = 96.dp

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
    private var hostView: android.view.View? = null

    @Suppress("unused")
    fun hideSplashScreen(duration: Int) = Unit

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        live = this

        QGCBridge.setHost(this)
        Qgc.start()

        acquireMulticastLock()
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)

        hostView = HostPlatform.start(this)

        QGCBridge.notifyFontScale(resources.configuration.fontScale)
        QGCBridge.notifySafeAreaInsets(0, 0, 0, 0)
        intent?.data?.let { QGCBridge.notifyDeepLink(it.toString()) }

        setContent { AppFontScale { AircastShell(hostView) } }
        GamepadInput.start(this, lifecycleScope)
    }

    override fun dispatchGenericMotionEvent(event: MotionEvent): Boolean =
        GamepadInput.onMotion(event) || super.dispatchGenericMotionEvent(event)

    override fun dispatchKeyEvent(event: KeyEvent): Boolean =
        GamepadInput.onKey(event) || super.dispatchKeyEvent(event)

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

    override fun setSystemBarAppearance(lightBars: Boolean) = Unit

    override fun getWindow(): Window =
        if (isDestroyed) live?.takeIf { it !== this }?.window ?: super.getWindow() else super.getWindow()

    override fun onDestroy() {
        if (live === this) live = null
        HostPlatform.stop(this)
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
fun AircastShell(hostView: android.view.View?) {
    var tab by remember { mutableStateOf(Tab.Fly) }
    LaunchedEffect(AppNavigation.setupPage) {
        if (AppNavigation.setupPage != null) tab = Tab.Setup
    }
    LaunchedEffect(AppNavigation.settingsPage) {
        if (AppNavigation.settingsPage != null) tab = Tab.Settings
    }
    var analyzePage by remember { mutableStateOf<AnalyzePage?>(null) }
    var popEpoch by remember { mutableIntStateOf(0) }
    val context = androidx.compose.ui.platform.LocalContext.current
    var flyView by remember { mutableStateOf(one.aircast.android.ui.loadFlyView(context)) }
    LaunchedEffect(flyView) { one.aircast.android.ui.saveFlyView(context, flyView) }
    val videoExpanded = flyView != one.aircast.android.ui.FlyView.Map
    var videoFullScreen by remember { mutableStateOf(false) }

    val flightActions = remember { movableContentOf<one.aircast.android.ui.FlyDeckLayout> { layout -> FlightActions(layout = layout) } }
    val flyVideo = remember {
        movableContentOf<Modifier, Boolean> { mod, expanded ->
            VideoSurface(modifier = mod, expanded = expanded, onClick = { flyView = one.aircast.android.ui.flyViewSwapped(flyView) }, onDoubleTap = { videoFullScreen = !videoFullScreen })
        }
    }
    var mapClickAt by remember { mutableStateOf<MapPoint?>(null) }
    var waypointTapped by remember { mutableStateOf<Int?>(null) }
    var roiTapped by remember { mutableStateOf<TrackPoint?>(null) }
    val flyMap = remember {
        movableContentOf<Modifier> { mod ->
            FlyMap(
                modifier = mod,
                cameraBottomPx = 0,
                onMapClick = { lat, lon -> mapClickAt = MapPoint(lat, lon) },
                onMissionItemClick = { waypointTapped = it },
                onRoiClick = { roiTapped = it },
            )
        }
    }
    val flyVideoSourceLayer = remember { movableContentOf { Hideable("videoSource") { VideoSourceLayer() } } }
    val flyCameraControlLayer = remember { movableContentOf { Hideable("cameraControl") { CameraControlLayer() } } }
    val flyObstacleArc = remember { movableContentOf { Hideable("obstacleArc") { ObstacleArc() } } }
    val flyAttitude = remember { movableContentOf { Hideable("instrumentPanel") { AttitudeInstrument() } } }
    val flyOrbitReadout = remember { movableContentOf { Hideable("orbit") { OrbitReadout() } } }
    val flyFollowMeReadout = remember { movableContentOf { Hideable("followMe") { FollowMeReadout() } } }
    val flyTrafficReadout = remember { movableContentOf { Hideable("traffic") { TrafficReadout() } } }
    val flyRcControlsLayer = remember { movableContentOf { RcControlsLayer() } }

    val snackbars = remember { SnackbarHostState() }
    var acknowledgedThrough by remember { mutableLongStateOf(-1L) }
    val notices by one.aircast.android.bridge.qgcPath(one.aircast.android.ui.hostNoticesPath(acknowledgedThrough))
    val noticeScope = rememberCoroutineScope()
    var shownAt by remember { mutableStateOf(emptyMap<String, Long>()) }

    BackHandler(enabled = tab != Tab.Fly) { tab = Tab.Fly }
    one.aircast.android.ui.CloseGuard(enabled = tab == Tab.Fly)

    LaunchedEffect(notices) {
        val batch = one.aircast.android.ui.noticeBatch(notices) ?: return@LaunchedEffect
        if (batch.through <= acknowledgedThrough) return@LaunchedEffect
        acknowledgedThrough = batch.through
        batch.unknownKinds.forEach {
            android.util.Log.w("HostNotices", "unrecognised notice kind '$it' - showing it rather than guessing")
        }
        batch.destination?.let { tab = Tab.from(it) }
        val now = System.currentTimeMillis()
        val banners = one.aircast.android.ui.quietBanners(batch.banners, shownAt, now)
        shownAt = shownAt + banners.associateWith { now }
        noticeScope.launch {
            withContext(Dispatchers.Default) {
                Qgc.invoke("host.acknowledgeThrough", batch.through)
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

    val darkBars = isSystemInDarkTheme()
    val view = LocalView.current
    SideEffect {
        (view.context as? Activity)?.window?.let { window ->
            WindowCompat.getInsetsController(window, view).apply {
                isAppearanceLightStatusBars = !darkBars
                isAppearanceLightNavigationBars = !darkBars
            }
        }
    }

    val fullScreenVideoJson by qgcPath(VIDEO_VIEW)
    val fullScreen = videoFullScreen && tab == Tab.Fly && videoExpanded && videoReading(fullScreenVideoJson)?.decoding == true
    LaunchedEffect(fullScreen) { if (!fullScreen) videoFullScreen = false }
    BackHandler(enabled = fullScreen) { videoFullScreen = false }

    AircastTheme {
        if (fullScreen) {
            Box(Modifier.fillMaxSize()) { flyVideo(Modifier.fillMaxSize(), true) }
            return@AircastTheme
        }
        val onFly = tab == Tab.Fly
        val landscape = !flyIsPortrait()
        val flyLandscape = onFly && landscape
        val flyStateJson by qgcPath(one.aircast.android.ui.FLY_STATE)
        val armedOnFly = remember(flyStateJson) { one.aircast.android.ui.flyState(flyStateJson)?.armed == true }
        val showNav = !(onFly && armedOnFly)
        val selectTab: (Tab) -> Unit = { entry ->
            if (tab == entry) {
                if (reselectClearsAnalyze(tab, entry)) analyzePage = null
                popEpoch++
            } else {
                tab = entry
            }
        }
        Row(Modifier.fillMaxSize()) {
        if (showNav && landscape) {
            NavigationRail(Modifier.fillMaxHeight()) {
                Tab.entries.forEach { entry ->
                    NavigationRailItem(
                        selected = tab == entry,
                        onClick = { selectTab(entry) },
                        icon = { Icon(entry.icon, entry.label) },
                        label = { Text(entry.label) },
                    )
                }
            }
        }
        Scaffold(
            modifier = Modifier.weight(1f),
            snackbarHost = { SnackbarHost(snackbars) },
            contentWindowInsets = if (onFly) WindowInsets(0) else androidx.compose.material3.ScaffoldDefaults.contentWindowInsets,
            bottomBar = {
                Column {
                LogReplayBar()
                if (showNav && !landscape) NavigationBar {
                    Tab.entries.forEach { entry ->
                        NavigationBarItem(
                            selected = tab == entry,
                            onClick = { selectTab(entry) },
                            icon = { Icon(entry.icon, entry.label) },
                            label = { Text(entry.label) },
                        )
                    }
                }
                }
            },
        ) { padding ->
            Box(Modifier.padding(padding).fillMaxSize()) {
                hostView?.let { view -> AndroidView(factory = { view }, modifier = Modifier.fillMaxSize()) }
                if (tab == Tab.Fly) OverlayEditBar(Modifier.align(Alignment.TopCenter).zIndex(2f).padding(top = 8.dp))

                if (onFly) {
                    FlyScreen(
                        view = flyView,
                        onView = { flyView = it },
                        landscape = flyLandscape,
                        status = {
                            VehicleStateChip()
                            VtolStateCell()
                            ControlRequestPrompt()
                            Spacer(Modifier.weight(1f))
                            if (hasVehicle()) Surface(
                                shape = CircleShape,
                                color = Color.Black.copy(alpha = 0.45f),
                                contentColor = MaterialTheme.aircast.outdoorForeground,
                            ) {
                                StatusReadingsInline(Modifier.padding(horizontal = 12.dp, vertical = 6.dp))
                            }
                        },
                        video = { mod, expanded -> flyVideo(mod, expanded) },
                        map = { mod -> flyMap(mod) },
                        keyRow = {
                            flyVideoSourceLayer()
                            flyCameraControlLayer()
                            flyObstacleArc()
                        },
                        keyRowEnd = { PinnedEmergencyStop() },
                        overlays = {
                            flyAttitude()
                            ObstacleReadout()
                            TerrainProgress()
                            flyOrbitReadout()
                            flyFollowMeReadout()
                            flyTrafficReadout()
                            flyRcControlsLayer()
                        },
                        actions = { layout -> flightActions(layout) },
                    )
                }
                if (onFly) one.aircast.android.ui.ConnectingCard(Modifier.align(Alignment.Center))

                if (tab == Tab.Fly) {
                    VirtualJoystick(
                        Modifier
                            .align(Alignment.BottomCenter)
                            .padding(horizontal = 12.dp)
                            .padding(bottom = VIRTUAL_JOYSTICK_BOTTOM_MARGIN),
                    )
                }

                mapClickAt?.takeIf { tab == Tab.Fly }?.let { point ->
                    MapClickMenu(point) { mapClickAt = null }
                }
                waypointTapped?.takeIf { tab == Tab.Fly }?.let { sequence ->
                    SetWaypointSheet(sequence) { waypointTapped = null }
                }
                roiTapped?.takeIf { tab == Tab.Fly }?.let { at ->
                    RoiSheet(at) { roiTapped = null }
                }
                MissionCompleteDialog()
                ResumeFailedPrompt()
                FirstRunDialog()

                key(popEpoch) {
                    when (tab) {
                        Tab.Settings -> Surface(Modifier.fillMaxSize()) { SettingsScreen() }
                        Tab.Setup -> Surface(Modifier.fillMaxSize()) { SetupScreen() }
                        Tab.Plan -> Surface(Modifier.fillMaxSize()) { PlanTab(onBack = { tab = Tab.Fly }) }
                        Tab.Analyze -> AnalyzeScreen(
                            page = analyzePage,
                            onSelect = { analyzePage = it },
                            modifier = Modifier.fillMaxSize(),
                        )
                        else -> Unit
                    }
                }
            }
        }
        }
    }
}
