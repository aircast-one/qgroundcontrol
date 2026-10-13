package one.aircast.android.ui

import android.annotation.SuppressLint
import android.content.Context
import android.view.ViewGroup
import android.webkit.JavascriptInterface
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebView
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.ui.input.pointer.pointerInput
import android.os.SystemClock
import androidx.compose.ui.geometry.Offset
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainInOrder
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import one.aircast.map.AircastSpace
import one.aircast.map.aircast
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.viewinterop.AndroidView
import androidx.webkit.WebViewAssetLoader
import androidx.webkit.WebViewClientCompat
import one.aircast.android.BuildConfig
import one.aircast.android.bridge.qgcPath
import one.aircast.map.coreTile
import one.aircast.map.tileMimeType
import org.json.JSONObject

internal const val SYNTHETIC_VIEW = "view.syntheticView"
internal const val SYNTHETIC_OVERLAYS = "view.syntheticOverlays"
internal const val SYNTHETIC_LABEL = "Synthetic view"
internal const val SYNTHETIC_SOURCE = "Synthetic View"
internal const val SYNTHETIC_AIM = "syntheticView.aim"
private const val TILT_SPAN_DEG = 90.0
internal const val DEFAULT_TILT_DEG = -15.0
private const val PAN_SPAN_DEG = 360.0
private const val SYNTHETIC_PAGE = "https://${WebViewAssetLoader.DEFAULT_DOMAIN}/assets/synthetic/index.html"
private const val SYNTHETIC_TILES = "/assets/synthetic/tiles"
private const val SYNTHETIC_HOST = "aircastHost"
private val NOTE_SPINNER = 16.dp
private val NOTE_SPINNER_STROKE = 2.dp

internal enum class SyntheticDraw(val note: String?) {
    Loading("Loading 3D view…"),
    Drawn(null),
    Failed("3D view failed to start"),
}

private val pages = mutableStateMapOf<Any, SyntheticDraw>()

internal fun pageOf(draws: Collection<SyntheticDraw>): SyntheticDraw = when {
    SyntheticDraw.Drawn in draws -> SyntheticDraw.Drawn
    SyntheticDraw.Failed in draws -> SyntheticDraw.Failed
    else -> SyntheticDraw.Loading
}

internal fun syntheticPage(): SyntheticDraw = pageOf(pages.values)

internal fun shownCameraStatus(camera: CameraEntry, showing: Boolean, page: SyntheticDraw): CameraStatus = when {
    !showing || camera.source != SYNTHETIC_SOURCE || camera.status != CameraStatus.Live -> camera.status
    page == SyntheticDraw.Drawn -> CameraStatus.Live
    page == SyntheticDraw.Failed -> CameraStatus.NoSignal
    else -> CameraStatus.Connecting
}

internal fun syntheticTilePath(path: String?): String? = path?.takeIf { it.startsWith("$SYNTHETIC_TILES/") }?.removePrefix(SYNTHETIC_TILES)

internal fun syntheticAvailable(view: JSONObject?): Boolean = view?.optBoolean("available") == true

internal fun syntheticPoseScript(view: JSONObject): String = "window.aircast && window.aircast.pose($view)"

internal fun syntheticOverlaysScript(view: JSONObject): String = "window.aircast && window.aircast.overlays($view)"

internal fun syntheticTilt(from: Double, draggedPx: Float, heightPx: Int): Double =
    (from + draggedPx.toDouble() / heightPx.coerceAtLeast(1) * TILT_SPAN_DEG).coerceIn(-TILT_SPAN_DEG, 0.0)

internal fun syntheticPan(from: Double, draggedPx: Float, widthPx: Int): Double =
    ((from + draggedPx.toDouble() / widthPx.coerceAtLeast(1) * PAN_SPAN_DEG + 180.0).mod(360.0)) - 180.0

internal fun syntheticAims(view: JSONObject?, cameras: CamerasReading?): Boolean =
    syntheticAvailable(view) && view?.optBoolean("aimable") == true && cameras?.cameras?.any { it.active && it.source == SYNTHETIC_SOURCE } == true

@Composable
internal fun rememberSyntheticAims(): Boolean {
    val view by qgcPath(SYNTHETIC_VIEW)
    val camerasJson by qgcPath(CAMERAS_VIEW)
    return remember(view, camerasJson) { syntheticAims(view, camerasReading(camerasJson)) }
}

@Composable
internal fun rememberSyntheticAvailable(): Boolean = one.aircast.android.bridge.qgcDerived(SYNTHETIC_VIEW, ::syntheticAvailable).value

internal fun syntheticChosen(reading: CamerasReading?): Boolean = reading?.cameras?.firstOrNull { it.active }?.source == SYNTHETIC_SOURCE

@Composable
internal fun rememberSyntheticChosen(): Boolean = one.aircast.android.bridge.qgcDerived(CAMERAS_VIEW) { syntheticChosen(camerasReading(it)) }.value

private fun tileResponse(encodedPath: String): WebResourceResponse =
    coreTile(encodedPath)?.let { WebResourceResponse(tileMimeType(it), null, 200, "OK", emptyMap(), it.inputStream()) }
        ?: WebResourceResponse("image/png", null, 404, "No tile", emptyMap(), ByteArray(0).inputStream())

@SuppressLint("SetJavaScriptEnabled")
private fun syntheticWebView(context: Context, onLoaded: () -> Unit, onDraw: (SyntheticDraw) -> Unit): WebView {
    val assets = WebViewAssetLoader.Builder().addPathHandler("/assets/", WebViewAssetLoader.AssetsPathHandler(context)).build()
    WebView.setWebContentsDebuggingEnabled(BuildConfig.DEBUG)
    return WebView(context).apply {
        layoutParams = ViewGroup.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT)
        setBackgroundColor(android.graphics.Color.BLACK)
        settings.javaScriptEnabled = true
        addJavascriptInterface(
            object {
                @JavascriptInterface fun placed() { post { onDraw(SyntheticDraw.Drawn) } }

                @JavascriptInterface fun failed() { post { onDraw(SyntheticDraw.Failed) } }
            },
            SYNTHETIC_HOST,
        )
        webViewClient = object : WebViewClientCompat() {
            override fun shouldInterceptRequest(view: WebView, request: WebResourceRequest): WebResourceResponse? =
                syntheticTilePath(request.url.encodedPath)?.let(::tileResponse) ?: assets.shouldInterceptRequest(request.url)

            override fun onPageFinished(view: WebView, url: String) = onLoaded()
        }
        loadUrl(SYNTHETIC_PAGE)
    }
}

@Composable
private fun BoxScope.SyntheticPage(view: JSONObject?, standIn: String?) {
    val context = LocalContext.current
    val overlays by qgcPath(SYNTHETIC_OVERLAYS)
    var loaded by remember { mutableStateOf(false) }
    var draw by remember { mutableStateOf(SyntheticDraw.Loading) }
    val web = remember { syntheticWebView(context, onLoaded = { loaded = true }, onDraw = { draw = it }) }
    DisposableEffect(web) { onDispose { web.destroy() } }
    DisposableEffect(web, draw) {
        pages[web] = draw
        onDispose { pages.remove(web) }
    }
    LaunchedEffect(view, loaded) { view?.takeIf { loaded }?.let { web.evaluateJavascript(syntheticPoseScript(it), null) } }
    LaunchedEffect(overlays, loaded) { overlays?.takeIf { loaded }?.let { web.evaluateJavascript(syntheticOverlaysScript(it), null) } }
    AndroidView({ web }, Modifier.fillMaxSize())
    shownNote(draw, standIn)?.let { note -> SyntheticNote(note, loading = draw == SyntheticDraw.Loading, Modifier.align(Alignment.Center)) }
}

@Composable
private fun BoxScope.NativePage(view: JSONObject?, standIn: String?) {
    val shown = remember { Any() }
    val draw = nativeDraw(view)
    DisposableEffect(shown, draw) {
        pages[shown] = draw
        onDispose { pages.remove(shown) }
    }
    SyntheticSurface(Modifier.fillMaxSize())
    shownNote(draw, standIn)?.let { note -> SyntheticNote(note, loading = draw == SyntheticDraw.Loading, Modifier.align(Alignment.Center)) }
}

internal fun shownNote(draw: SyntheticDraw, standIn: String?): String? = draw.note ?: standIn

internal fun gimbalAimsSynthetic(view: JSONObject?, aimable: Boolean): Boolean =
    aimable && syntheticAvailable(view) && view?.optBoolean("aimable", true) == false

private const val GIMBAL_STATE = "view.gimbal"
private const val GIMBAL_AIM_COMMAND = "gimbal.aim"
private const val GIMBAL_CENTER_COMMAND = "gimbal.center"
private const val GIMBAL_AIM_EVERY_MS = 50L
private const val GIMBAL_AIM_SETTLES_MS = 1000L

internal fun dragStart(reported: Pair<Double, Double>?, sent: Pair<Double, Double>?, sentAgoMs: Long): Pair<Double, Double> =
    sent?.takeIf { sentAgoMs < GIMBAL_AIM_SETTLES_MS } ?: reported ?: (0.0 to 0.0)

internal fun gimbalAim(view: JSONObject?): Pair<Double, Double>? =
    view?.optJSONArray("gimbals")
        ?.let { list -> (0 until list.length()).mapNotNull(list::optJSONObject) }
        ?.firstOrNull { it.optBoolean("active") && !it.isNull("pitch") && !it.isNull("bodyYaw") }
        ?.let { it.optDouble("pitch") to it.optDouble("bodyYaw") }

@Composable
internal fun SyntheticView(modifier: Modifier = Modifier, aimable: Boolean = false, standIn: String? = null) {
    val view by qgcPath(SYNTHETIC_VIEW)
    val canAim = aimable && view?.optBoolean("aimable") == true
    val aimedTilt by rememberUpdatedState(view?.optDouble("tilt", DEFAULT_TILT_DEG) ?: DEFAULT_TILT_DEG)
    val pan by rememberUpdatedState(view?.optDouble("pan", 0.0) ?: 0.0)
    val sent = remember(canAim) { mutableStateOf<Pair<Double, Double>?>(null) }
    val send = { tilt: Double, turned: Double ->
        sent.value = tilt to turned
        offMainInOrder { Qgc.invoke(SYNTHETIC_AIM, tilt, turned) }
    }
    val steersGimbal = gimbalAimsSynthetic(view, aimable)
    val gimbalJson by qgcPath(GIMBAL_STATE)
    val gimbalAt by rememberUpdatedState(gimbalAim(gimbalJson))
    val lastAim = remember { mutableStateOf<Pair<Double, Double>?>(null) }
    val lastAimAt = remember { mutableStateOf(0L) }
    val aimGimbal = { tilt: Double, yaw: Double ->
        lastAim.value = tilt to yaw
        lastAimAt.value = SystemClock.uptimeMillis()
        offMainInOrder { gimbalRefusal(Qgc.call(GIMBAL_AIM_COMMAND, tilt, yaw)) }
    }
    val aiming = Modifier
        .pointerInput(canAim) {
            if (!canAim) return@pointerInput
            var aim = aimedTilt to pan
            detectDragGestures(onDragStart = { aim = sent.value ?: (aimedTilt to pan) }) { change, dragged ->
                change.consume()
                aim = syntheticTilt(aim.first, dragged.y, size.height) to syntheticPan(aim.second, dragged.x, size.width)
                send(aim.first, aim.second)
            }
        }
        .pointerInput(canAim) {
            if (!canAim) return@pointerInput
            detectTapGestures(onDoubleTap = { send(DEFAULT_TILT_DEG, 0.0) })
        }
        .pointerInput(steersGimbal) {
            if (!steersGimbal) return@pointerInput
            var start = 0.0 to 0.0
            var moved = Offset.Zero
            var sentAt = 0L
            val target = { syntheticTilt(start.first, moved.y, size.height) to syntheticPan(start.second, moved.x, size.width) }
            detectDragGestures(
                onDragStart = {
                    start = dragStart(gimbalAt, lastAim.value, SystemClock.uptimeMillis() - lastAimAt.value)
                    moved = Offset.Zero
                },
                onDragEnd = { target().let { (tilt, yaw) -> aimGimbal(tilt, yaw) } },
            ) { change, dragged ->
                change.consume()
                moved += dragged
                val now = SystemClock.uptimeMillis()
                if (now - sentAt >= GIMBAL_AIM_EVERY_MS) {
                    sentAt = now
                    target().let { (tilt, yaw) -> aimGimbal(tilt, yaw) }
                }
            }
        }
        .pointerInput(steersGimbal) {
            if (!steersGimbal) return@pointerInput
            detectTapGestures(onDoubleTap = { offMainInOrder { gimbalRefusal(Qgc.call(GIMBAL_CENTER_COMMAND)) } })
        }
    Box(modifier.then(aiming)) {
        if (nativeChosen(view)) NativePage(view, standIn) else SyntheticPage(view, standIn)
    }
}

@Composable
private fun SyntheticNote(note: String, loading: Boolean, modifier: Modifier) {
    Row(modifier.padding(horizontal = AircastSpace.s4).osdShadow(), horizontalArrangement = Arrangement.spacedBy(AircastSpace.s2), verticalAlignment = Alignment.CenterVertically) {
        if (loading) CircularProgressIndicator(Modifier.size(NOTE_SPINNER), color = MaterialTheme.aircast.outdoorForeground, strokeWidth = NOTE_SPINNER_STROKE)
        Text(note, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.aircast.outdoorForeground, textAlign = TextAlign.Center, maxLines = 2)
    }
}
