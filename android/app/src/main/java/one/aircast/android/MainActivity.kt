package one.aircast.android

import androidx.compose.runtime.getValue
import one.aircast.android.bridge.VideoCommands
import one.aircast.android.bridge.AppCommands
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.toArgb
import android.content.Intent
import android.content.res.Configuration
import android.net.wifi.WifiManager
import android.os.Bundle
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.Window
import androidx.activity.ComponentActivity
import androidx.lifecycle.lifecycleScope
import androidx.activity.compose.BackHandler
import one.aircast.android.ui.ConnectionLocks
import one.aircast.android.ui.DeviceCameraHost
import one.aircast.android.ui.videoReading
import one.aircast.android.ui.VIDEO_VIEW
import one.aircast.android.bridge.qgcPath
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.material3.NavigationRailItem
import androidx.compose.material3.NavigationRail
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import one.aircast.map.aircast
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.movableContentOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.foundation.layout.WindowInsets
import one.aircast.android.ui.AppFontScale
import one.aircast.android.ui.OverlayEditBar
import one.aircast.android.ui.LayoutWidget
import one.aircast.android.ui.LogReplayBar
import one.aircast.android.ui.StatusPill
import one.aircast.android.ui.VehicleStateChip
import one.aircast.android.ui.ControlRequestPrompt
import one.aircast.android.ui.VtolStateCell
import one.aircast.android.ui.ResumeFailedPrompt
import one.aircast.android.ui.VirtualJoystick
import androidx.compose.ui.Modifier
import androidx.compose.ui.zIndex
import androidx.compose.ui.unit.dp
import androidx.annotation.DrawableRes
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.view.WindowCompat
import android.app.Activity
import androidx.compose.runtime.SideEffect
import androidx.compose.ui.platform.LocalView
import androidx.compose.runtime.DisposableEffect
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.map.AircastTheme
import one.aircast.android.ui.AnalyzePage
import one.aircast.android.ui.AnalyzeScreen
import one.aircast.android.ui.CameraControlLayer
import one.aircast.android.ui.FlightActions
import one.aircast.android.ui.FollowMeReadout
import one.aircast.android.ui.MapClickMenu
import one.aircast.android.ui.MapPoint
import one.aircast.android.ui.MapHoldHint
import one.aircast.android.ui.MAP_HOLD_HINT
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
import one.aircast.map.FlyMap
import one.aircast.map.TrackPoint
import one.aircast.android.ui.CameraSwitch
import androidx.compose.foundation.layout.Arrangement
import one.aircast.map.AircastSpace
import one.aircast.android.ui.VideoSurface
import one.aircast.android.ui.FlyScreen
import one.aircast.android.ui.FlyPortrait
import androidx.compose.foundation.layout.RowScope
import one.aircast.android.ui.flyIsPortrait
import org.mavlink.qgroundcontrol.QGCBridge

private val KEY_ROW_STOP_GAP = 24.dp

private val RAIL_SHUTTER_ROOM = 56.dp

private val VIRTUAL_JOYSTICK_BOTTOM_MARGIN = 96.dp

private const val MULTICAST_LOCK_TAG = "Aircast"


internal fun reselectClearsAnalyze(current: Tab, tapped: Tab): Boolean =
    current == tapped && tapped == Tab.Analyze

internal fun visibleTabs(advanced: Boolean): List<Tab> = Tab.entries.filter { advanced || it != Tab.Analyze }

enum class Tab(val label: String, @DrawableRes val icon: Int) {
    Fly("Fly", R.drawable.ic_flight),
    Plan("Plan", R.drawable.ic_map),
    Analyze("Analyze", R.drawable.ic_analytics);

    companion object {
        fun from(destination: String): Tab? = when (destination.lowercase()) {
            "fly" -> Fly
            "plan" -> Plan
            "analyze" -> Analyze
            else -> null
        }
    }
}

class MainActivity : ComponentActivity(), QGCBridge.Host {
    companion object {
        private var live: MainActivity? = null
    }

    private var multicastLock: WifiManager.MulticastLock? = null
    private val debugUiReceiver = DebugUiReceiver()
    private var hostView: android.view.View? = null

    @Suppress("unused")
    fun hideSplashScreen(duration: Int) = Unit

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        live = this

        QGCBridge.setHost(this)
        Qgc.start()

        acquireMulticastLock()

        hostView = HostPlatform.start(this)

        QGCBridge.notifyFontScale(resources.configuration.fontScale)
        QGCBridge.notifySafeAreaInsets(0, 0, 0, 0)
        intent?.data?.let { QGCBridge.notifyDeepLink(it.toString()) }

        setContent {
            ConnectionLocks()
            one.aircast.android.ui.VoiceAlerts()
            DeviceCameraHost()
            AppFontScale {
                val context = androidx.compose.ui.platform.LocalContext.current
                val flyScreen = remember { one.aircast.android.ui.FlyScreenState(one.aircast.android.ui.OverlayLayoutState(one.aircast.android.ui.overlayLayoutStore(context))) }
                val navigation = remember { one.aircast.android.ui.AppNavigationState() }
                val openPlan = remember { one.aircast.android.ui.OpenPlanDocument() }
                androidx.compose.runtime.CompositionLocalProvider(
                    one.aircast.android.ui.LocalAppNavigation provides navigation,
                    one.aircast.android.ui.LocalOpenPlan provides openPlan,
                    one.aircast.android.ui.LocalFlyScreenState provides flyScreen,
                    one.aircast.map.LocalFlyMapEdits provides flyScreen.mapEdits,
                ) { AircastShell(hostView) }
            }
        }
        if (BuildConfig.DEBUG) {
            androidx.core.content.ContextCompat.registerReceiver(this, debugUiReceiver, android.content.IntentFilter(DEBUG_UI_ACTION), androidx.core.content.ContextCompat.RECEIVER_EXPORTED)
        }
        GamepadInput.start(this, lifecycleScope)
        one.aircast.android.ui.VirtualStickSender.start(lifecycleScope)
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
        if (BuildConfig.DEBUG) unregisterReceiver(debugUiReceiver)
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
    val navigation = one.aircast.android.ui.LocalAppNavigation.current
    var tab by remember { mutableStateOf(Tab.Fly) }
    val flyScreen = one.aircast.android.ui.LocalFlyScreenState.current
    val overlayLayout = flyScreen.layout
    LaunchedEffect(tab) { if (tab != Tab.Fly) overlayLayout.editing = false }
    LaunchedEffect(navigation.setupPage) {
        if (navigation.setupPage != null) navigation.openAircraft()
    }
    LaunchedEffect(navigation.settingsPage) {
        if (navigation.settingsPage != null) navigation.settingsOpen = true
    }
    val vehicleNow = one.aircast.android.ui.hasVehicle()
    var hadVehicle by remember { mutableStateOf(vehicleNow) }
    LaunchedEffect(vehicleNow) {
        if (vehicleNow && !hadVehicle && navigation.settingsShowing == one.aircast.android.ui.SettingsGroup.Transmission) navigation.settingsOpen = false
        hadVehicle = vehicleNow
    }
    var analyzePage by remember { mutableStateOf<AnalyzePage?>(null) }
    var popEpoch by remember { mutableIntStateOf(0) }
    val context = androidx.compose.ui.platform.LocalContext.current
    var flyView by remember { mutableStateOf(one.aircast.android.ui.loadFlyView(context)) }
    LaunchedEffect(flyView) { one.aircast.android.ui.saveFlyView(context, flyView) }
    LaunchedEffect(Unit) {
        DebugUi.commands.collect { command ->
            when (command) {
                is DebugCommand.ShowTab -> tab = command.tab
                is DebugCommand.ShowFlyView -> {
                    tab = Tab.Fly
                    flyView = command.view
                }
                is DebugCommand.Orient -> (context as? Activity)?.requestedOrientation = command.orientation
                is DebugCommand.EditLayout -> if (command.on) overlayLayout.startEditing() else overlayLayout.editing = false
                DebugCommand.ResetLayout -> overlayLayout.reset()
                is DebugCommand.Open -> when (command.target) {
                    "readings" -> flyScreen.choosingReadings = true
                    "settings" -> navigation.settingsOpen = true
                    in one.aircast.android.ui.REQUESTABLE_SHEETS -> flyScreen.requestedSheet = command.target
                    else -> flyScreen.deckRequest = command.target
                }
            }
        }
    }
    val debugLandscape = !one.aircast.android.ui.flyIsPortrait()
    androidx.compose.runtime.SideEffect {
        DebugUi.state = org.json.JSONObject()
            .put("tab", tab.name)
            .put("flyView", flyView.name)
            .put("landscape", debugLandscape)
            .put("layoutEditing", overlayLayout.editing)
            .put("layoutLocked", overlayLayout.locked)
            .put("guidedPanelOpen", flyScreen.guidedPanelOpen)
            .put("pendingOpen", flyScreen.requestedSheet ?: flyScreen.deckRequest ?: org.json.JSONObject.NULL)
            .toString()
    }
    val videoExpanded = flyView != one.aircast.android.ui.FlyView.Map
    var videoFullScreen by remember { mutableStateOf(false) }

    val flightActions = remember { movableContentOf<one.aircast.android.ui.FlyDeckLayout> { layout -> FlightActions(layout = layout, center = { one.aircast.android.ui.CameraShutters() }) } }
    val flyVideo = remember {
        movableContentOf<Modifier, Boolean> { mod, expanded ->
            VideoSurface(modifier = mod, expanded = expanded, fullScreen = videoFullScreen, onClick = { flyView = one.aircast.android.ui.flyViewSwapped(flyView) }, onDoubleTap = { videoFullScreen = !videoFullScreen })
        }
    }
    var mapClickAt by remember { mutableStateOf<MapPoint?>(null) }
    var mapHoldHint by remember { mutableStateOf(MapHoldHint.Unseen) }
    var waypointTapped by remember { mutableStateOf<Int?>(null) }
    var roiTapped by remember { mutableStateOf<TrackPoint?>(null) }
    val flyMap = remember {
        movableContentOf<Modifier> { mod ->
            FlyMap(
                modifier = mod,
                cameraBottomPx = 0,
                topInsetPx = if (flyView == one.aircast.android.ui.FlyView.Map || flyIsPortrait()) flyScreen.mapInsets.top else 0,
                bottomInsetPx = if (flyView == one.aircast.android.ui.FlyView.Map || flyIsPortrait()) flyScreen.mapInsets.bottom else 0,
                logoEndInsetPx = with(androidx.compose.ui.platform.LocalDensity.current) { one.aircast.android.ui.MAP_LAYERS_CLEARANCE.roundToPx() }.takeIf { flyView == one.aircast.android.ui.FlyView.Map || flyIsPortrait() },
                pip = flyView == one.aircast.android.ui.FlyView.Video && !flyIsPortrait(),
                onMapClick = { lat, lon ->
                    mapHoldHint = MapHoldHint.Learned
                    mapClickAt = MapPoint(lat, lon)
                },
                onPlainTap = { if (mapHoldHint == MapHoldHint.Unseen) mapHoldHint = MapHoldHint.Due },
                onMissionItemClick = { waypointTapped = it },
                onRoiClick = { roiTapped = it },
                onTrafficClick = { flyScreen.requestedSheet = one.aircast.android.ui.TRAFFIC_SHEET },
                clickMarker = mapClickAt?.let { TrackPoint(it.latitude, it.longitude) },
            )
        }
    }
    val flyCameraSwitch = remember { movableContentOf<Boolean, Modifier> { thumbnailRoom, gap -> LayoutWidget("cameraSwitch") { CameraSwitch(thumbnailRoom, gap) } } }
    val flyCameraControlLayer = remember { movableContentOf { LayoutWidget("cameraControl") { CameraControlLayer(shutters = !flyIsPortrait()) } } }
    val flyObstacleArc = remember { movableContentOf { LayoutWidget("obstacleArc") { ObstacleArc() } } }
    val flyOrbitReadout = remember { movableContentOf { LayoutWidget("orbit") { OrbitReadout() } } }
    val flyFollowMeReadout = remember { movableContentOf { LayoutWidget("followMe") { FollowMeReadout() } } }
    val flyRcControlsLayer = remember { movableContentOf { LayoutWidget("rcControls") { RcControlsLayer() } } }

    val snackbars = remember { SnackbarHostState() }
    LaunchedEffect(mapHoldHint) {
        if (mapHoldHint != MapHoldHint.Due) return@LaunchedEffect
        snackbars.showSnackbar(MAP_HOLD_HINT)
        mapHoldHint = MapHoldHint.Learned
    }
    val alerts = remember { SnackbarHostState() }
    var acknowledgedThrough by remember { mutableLongStateOf(-1L) }
    val notices by one.aircast.android.bridge.qgcPath(one.aircast.android.ui.hostNoticesPath(acknowledgedThrough))
    val noticeScope = rememberCoroutineScope()
    var shownAt by remember { mutableStateOf(emptyMap<String, Long>()) }
    var appMessages by remember { mutableStateOf(emptyList<one.aircast.android.ui.AppMessage>()) }

    val refusalScope = rememberCoroutineScope()
    val refuseNavigation: () -> Boolean = {
        one.aircast.android.ui.navigationRefusal(navigation.blockedReason, leaving = true)
            ?.also { said -> refusalScope.launch { snackbars.showSnackbar(said) } } != null
    }
    BackHandler(enabled = tab != Tab.Fly) { if (!refuseNavigation()) tab = Tab.Fly }

    LaunchedEffect(notices) {
        val batch = one.aircast.android.ui.noticeBatch(notices) ?: return@LaunchedEffect
        if (batch.through <= acknowledgedThrough) return@LaunchedEffect
        acknowledgedThrough = batch.through
        batch.unknownKinds.forEach {
            android.util.Log.w("HostNotices", "unrecognised notice kind '$it' - showing it rather than guessing")
        }
        batch.destination?.let { destination ->
            Tab.from(destination)?.let { tab = it } ?: if (destination.equals("setup", ignoreCase = true)) navigation.openAircraft() else navigation.settingsOpen = true
        }
        val now = System.currentTimeMillis()
        val banners = one.aircast.android.ui.quietBanners(batch.banners, shownAt, now)
        shownAt = shownAt + banners.associateWith { now }
        appMessages = (appMessages + batch.dialogs).distinct()
        noticeScope.launch {
            withContext(Dispatchers.Default) {
                AppCommands.acknowledgeNoticesThrough(batch.through)
            }
            val errors = banners.filter { it in batch.errorBanners }
            one.aircast.android.ui.criticalBanner(errors)?.takeIf { tab != Tab.Fly }?.let { critical ->
                launch {
                    alerts.showSnackbar(critical, withDismissAction = true, duration = androidx.compose.material3.SnackbarDuration.Indefinite)
                    withContext(Dispatchers.Default) { Qgc.invoke(one.aircast.android.ui.RESET_ERROR_LEVEL_MESSAGES) }
                }
            }
            banners.filterNot { it in batch.errorBanners }.forEach { banner -> snackbars.showSnackbar(banner) }
        }
    }

    val flyStateJson by one.aircast.android.bridge.qgcPath(one.aircast.android.ui.FLY_STATE)
    val flyNow = remember(flyStateJson) { one.aircast.android.ui.flyState(flyStateJson) }
    var wasArmed by remember { mutableStateOf(false) }
    var flewWhileArmed by remember { mutableStateOf(false) }
    var armedBattery by remember { mutableStateOf<Int?>(null) }
    LaunchedEffect(flyNow?.armed, flyNow?.state) {
        val armedNow = flyNow?.connected == true && flyNow.armed
        if (armedNow && !wasArmed) noticeScope.launch { armedBattery = withContext(Dispatchers.Default) { one.aircast.android.ui.batteryPercentNow() } }
        one.aircast.android.ui.disarmNotice(wasArmed, flewWhileArmed, armedNow)?.let { said ->
            val landed = flewWhileArmed
            val startBattery = armedBattery
            noticeScope.launch {
                val summary = if (!landed) "" else withContext(Dispatchers.Default) {
                    one.aircast.android.ui.landedSummary(
                        one.aircast.android.ui.flightTimeNow(),
                        one.aircast.android.ui.flownDistanceText(Qgc.get(one.aircast.android.ui.VEHICLE_FLIGHT_DISTANCE)),
                        startBattery?.let { start -> one.aircast.android.ui.batteryPercentNow()?.let { start - it } },
                    )
                }
                snackbars.showSnackbar(listOf(said, summary).filter { it.isNotBlank() }.joinToString(" \u00b7 "))
            }
        }
        flewWhileArmed = armedNow && (flewWhileArmed || flyNow?.state == "flying" || flyNow?.state == "landing")
        wasArmed = armedNow
    }
    val flyVideoJson by one.aircast.android.bridge.qgcPath(one.aircast.android.ui.VIDEO_VIEW)
    val noVideoSource = remember(flyVideoJson) { one.aircast.android.ui.videoReading(flyVideoJson)?.let { !it.available && !it.sourceChosen } == true }
    val shownFlyView = one.aircast.android.ui.flyViewShown(flyView, armed = flyNow?.armed == true, noVideoSource = noVideoSource)

    val vehiclesJson by one.aircast.android.bridge.qgcPath(one.aircast.map.VEHICLES_VIEW)
    var lastVehicles by remember {
        mutableStateOf<one.aircast.map.VehicleChoices?>(null)
    }

    LaunchedEffect(vehiclesJson) {
        val now = one.aircast.map.vehicleChoices(vehiclesJson)
        val notice = one.aircast.map.handoverNotice(lastVehicles, now, one.aircast.map.VehicleBridge.lastAsked)
        if (one.aircast.map.activeChanged(lastVehicles, now)) {
            one.aircast.map.VehicleBridge.forget()
        }
        lastVehicles = one.aircast.map.rememberedChoices(lastVehicles, now)
        notice?.let { said -> noticeScope.launch { snackbars.showSnackbar(said) } }
    }

    LaunchedEffect(Unit) {
        withContext(Dispatchers.Default) {
            VideoCommands.setNativeRendering(true)
            VideoCommands.initNative()
        }
    }

    val darkBars = one.aircast.android.ui.appDarkTheme()
    val overMap = tab == Tab.Fly && one.aircast.android.ui.flyIsPortrait()
    val view = LocalView.current
    SideEffect {
        (view.context as? Activity)?.window?.let { window ->
            WindowCompat.getInsetsController(window, view).apply {
                isAppearanceLightStatusBars = !darkBars && !overMap
                isAppearanceLightNavigationBars = !darkBars
            }
        }
    }

    val fullScreenVideoJson by qgcPath(VIDEO_VIEW)
    val syntheticOnScreen = one.aircast.android.ui.rememberSyntheticAvailable()
    val fullScreen = videoFullScreen && tab == Tab.Fly && videoExpanded && (videoReading(fullScreenVideoJson)?.decoding == true || syntheticOnScreen)
    LaunchedEffect(fullScreen, videoFullScreen) { if (!fullScreen) videoFullScreen = false }
    BackHandler(enabled = fullScreen) { videoFullScreen = false }

    AircastTheme(dark = darkBars) {
        one.aircast.android.ui.CloseGuard(enabled = tab == Tab.Fly && !fullScreen)
        one.aircast.android.ui.GimbalTakeControlDialog()
        appMessages.firstOrNull()?.let { shown -> one.aircast.android.ui.AppMessageDialog(shown, onOpenSetup = { navigation.openAircraft() }) { appMessages = appMessages.drop(1) } }
        val barColor = MaterialTheme.colorScheme.surface.toArgb()
        SideEffect {
            (view.context as? Activity)?.window?.let { window ->
                @Suppress("DEPRECATION")
                window.statusBarColor = barColor
                @Suppress("DEPRECATION")
                window.navigationBarColor = barColor
            }
        }
        val onFly = tab == Tab.Fly
        val landscape = !flyIsPortrait()
        val flyLandscape = onFly && landscape
        val tabs = visibleTabs(one.aircast.android.ui.advancedUiShown())
        val selectTab: (Tab) -> Unit = { entry ->
            when {
                refuseNavigation() -> Unit
                tab == entry -> {
                    if (reselectClearsAnalyze(tab, entry)) analyzePage = null
                    popEpoch++
                }
                else -> tab = entry
            }
        }
        val immersiveView = LocalView.current
        DisposableEffect(flyLandscape || fullScreen) {
            val window = (immersiveView.context as? Activity)?.window
            val controller = window?.let { androidx.core.view.WindowCompat.getInsetsController(it, immersiveView) }
            if (flyLandscape || fullScreen) {
                controller?.systemBarsBehavior = androidx.core.view.WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
                controller?.hide(androidx.core.view.WindowInsetsCompat.Type.systemBars())
            } else {
                controller?.show(androidx.core.view.WindowInsetsCompat.Type.systemBars())
            }
            onDispose { }
        }
        androidx.compose.runtime.CompositionLocalProvider(one.aircast.map.LocalImmersive provides flyLandscape) {
        Row(Modifier.fillMaxSize()) {
        if (landscape && !flyLandscape) {
            NavigationRail(Modifier.fillMaxHeight(), windowInsets = WindowInsets.safeDrawing.only(WindowInsetsSides.Vertical + WindowInsetsSides.Start)) {
                tabs.forEach { entry ->
                    NavigationRailItem(
                        selected = tab == entry && !navigation.settingsOpen,
                        onClick = { selectTab(entry) },
                        icon = { Icon(painterResource(entry.icon), entry.label) },
                        label = { Text(entry.label) },
                    )
                }
                NavigationRailItem(
                    selected = navigation.settingsOpen,
                    onClick = { navigation.settingsOpen = true },
                    icon = { Icon(painterResource(R.drawable.ic_settings), "Settings") },
                    label = { Text("Settings") },
                )
            }
        }
        Scaffold(
            modifier = Modifier.weight(1f),
            snackbarHost = {
                val aboveDeck = if (onFly && flyIsPortrait()) with(androidx.compose.ui.platform.LocalDensity.current) { flyScreen.mapInsets.bottom.toDp() } else 0.dp
                SnackbarHost(snackbars, Modifier.padding(bottom = aboveDeck)) { one.aircast.android.ui.AppSnackbar(it) }
            },
            topBar = { if (!onFly) SnackbarHost(alerts, Modifier.statusBarsPadding()) { one.aircast.android.ui.AppSnackbar(it) } },
            contentWindowInsets = if (onFly) WindowInsets(0) else androidx.compose.material3.ScaffoldDefaults.contentWindowInsets,
            bottomBar = {
                Column {
                LogReplayBar()
                if (!landscape && !onFly) NavigationBar {
                    tabs.forEach { entry ->
                        NavigationBarItem(
                            selected = tab == entry && !navigation.settingsOpen,
                            onClick = { selectTab(entry) },
                            icon = { Icon(painterResource(entry.icon), entry.label) },
                            label = { Text(entry.label) },
                        )
                    }
                    NavigationBarItem(
                        selected = navigation.settingsOpen,
                        onClick = { navigation.settingsOpen = true },
                        icon = { Icon(painterResource(R.drawable.ic_settings), "Settings") },
                        label = { Text("Settings") },
                    )
                }
                }
            },
        ) { padding ->
            Box(Modifier.padding(padding).fillMaxSize()) {
                hostView?.let { view -> AndroidView(factory = { view }, modifier = Modifier.fillMaxSize()) }
                if (tab == Tab.Fly) OverlayEditBar(Modifier.align(Alignment.BottomCenter).zIndex(2f).then(if (onFly) Modifier.windowInsetsPadding(WindowInsets.navigationBars) else Modifier).padding(8.dp).widthIn(max = 560.dp))

                if (onFly) {
                    val flyStatus: @Composable RowScope.() -> Unit = {
                        one.aircast.android.ui.FlyTabMenu(tabs.map { one.aircast.android.ui.MenuDestination(it.label, it.icon, it == Tab.Fly) { selectTab(it) } } + one.aircast.android.ui.MenuDestination("Settings", R.drawable.ic_settings, false) { navigation.settingsOpen = true })
                        Box(Modifier.weight(1f), contentAlignment = Alignment.CenterStart) { LayoutWidget("vehicleState", movable = false, hideable = false) { VehicleStateChip() } }
                        LayoutWidget("vtolState", movable = false, hideable = false) { VtolStateCell() }
                        ControlRequestPrompt()
                        LayoutWidget("statusPill", movable = false, hideable = false) { StatusPill() }
                        one.aircast.android.ui.FlySettingsButton()
                    }
                    val flyKeyRow: @Composable () -> Unit = {
                        flyObstacleArc()
                    }
                    val flyRail: @Composable (Boolean, Boolean) -> Unit = { stacked, thumbnailRoom ->
                        if (stacked) {
                            Column(horizontalAlignment = Alignment.End) {
                                flyCameraSwitch(thumbnailRoom, Modifier.padding(bottom = AircastSpace.s2))
                                flyCameraControlLayer()
                            }
                        } else {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                flyCameraSwitch(thumbnailRoom, Modifier.padding(end = AircastSpace.s2))
                                Box(Modifier.widthIn(min = RAIL_SHUTTER_ROOM), contentAlignment = Alignment.Center) { flyCameraControlLayer() }
                            }
                        }
                    }
                    val flyOverlays: @Composable () -> Unit = {
                        LayoutWidget("messageBanner", hideable = false) { one.aircast.android.ui.VehicleMessageBanner() }
                        LayoutWidget("fleet") { one.aircast.android.ui.FleetCard() }
                        LayoutWidget("missionProgress") { one.aircast.android.ui.MissionProgressCard() }
                        LayoutWidget("obstacleReadout") { ObstacleReadout() }
                        LayoutWidget("terrainProgress") { TerrainProgress() }
                        flyOrbitReadout()
                        flyFollowMeReadout()
                        flyRcControlsLayer()
                    }
                    if (flyLandscape) {
                        FlyScreen(
                            view = shownFlyView,
                            onView = { flyView = it },
                            status = flyStatus,
                            video = { mod, expanded -> flyVideo(mod, expanded) },
                            map = { mod -> flyMap(mod) },
                            keyRow = flyKeyRow,
                            rail = { thumbnailRoom -> flyRail(false, thumbnailRoom) },
                            overlays = flyOverlays,
                            actions = { layout -> flightActions(layout) },
                            fullScreen = fullScreen,
                            onFullScreen = {
                                flyView = one.aircast.android.ui.FlyView.Video
                                videoFullScreen = true
                            },
                            onExitFullScreen = { videoFullScreen = false },
                        )
                    } else {
                        FlyPortrait(
                            view = shownFlyView,
                            onView = { flyView = it },
                            status = flyStatus,
                            video = { mod, expanded -> flyVideo(mod, expanded) },
                            map = { mod -> flyMap(mod) },
                            keyRow = flyKeyRow,
                            rail = { thumbnailRoom -> flyRail(true, thumbnailRoom) },
                            overlays = flyOverlays,
                            actions = { layout -> flightActions(layout) },
                            fullScreen = fullScreen,
                            onFullScreen = {
                                flyView = one.aircast.android.ui.FlyView.Video
                                videoFullScreen = true
                            },
                            onExitFullScreen = { videoFullScreen = false },
                        )
                    }
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
                if (navigation.settingsOpen) {
                    val requested = remember { navigation.settingsPage.also { navigation.settingsPage = null } }
                    one.aircast.android.ui.SettingsSheet(requested) { navigation.settingsOpen = false }
                }
                MissionCompleteDialog()
                var resumeDismissed by remember { mutableStateOf<Int?>(null) }
                if (onFly) ResumeFailedPrompt(resumeDismissed) { resumeDismissed = it }
                FirstRunDialog()
                one.aircast.android.ui.DeepLinkSetupPrompt()

                key(popEpoch) {
                    when (tab) {
                        Tab.Plan -> Surface(Modifier.fillMaxSize()) { PlanTab(onFly = { tab = Tab.Fly }) }
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
}
