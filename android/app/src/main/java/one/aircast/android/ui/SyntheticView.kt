package one.aircast.android.ui

import android.annotation.SuppressLint
import android.content.Context
import android.view.ViewGroup
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebView
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.ui.input.pointer.pointerInput
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainInOrder
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
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
private const val DEFAULT_TILT_DEG = -15.0
private const val DEFAULT_FOV_DEG = 70.0
private const val SYNTHETIC_PAGE = "https://${WebViewAssetLoader.DEFAULT_DOMAIN}/assets/synthetic/index.html"
private const val SYNTHETIC_TILES = "/assets/synthetic/tiles"

internal fun syntheticTilePath(path: String?): String? = path?.takeIf { it.startsWith("$SYNTHETIC_TILES/") }?.removePrefix(SYNTHETIC_TILES)

internal fun syntheticAvailable(view: JSONObject?): Boolean = view?.optBoolean("available") == true

internal fun syntheticPoseScript(view: JSONObject): String = "window.aircast && window.aircast.pose($view)"

internal fun syntheticOverlaysScript(view: JSONObject): String = "window.aircast && window.aircast.overlays($view)"

internal fun syntheticTilt(from: Double, draggedPx: Float, heightPx: Int): Double =
    (from + draggedPx.toDouble() / heightPx.coerceAtLeast(1) * TILT_SPAN_DEG).coerceIn(-TILT_SPAN_DEG, 0.0)

internal fun syntheticPan(from: Double, draggedPx: Float, widthPx: Int, fovDeg: Double): Double =
    ((from - draggedPx.toDouble() / widthPx.coerceAtLeast(1) * fovDeg + 180.0).mod(360.0)) - 180.0

@Composable
internal fun rememberSyntheticAvailable(): Boolean {
    val view by qgcPath(SYNTHETIC_VIEW)
    return syntheticAvailable(view)
}

private fun tileResponse(encodedPath: String): WebResourceResponse =
    coreTile(encodedPath)?.let { WebResourceResponse(tileMimeType(it), null, 200, "OK", emptyMap(), it.inputStream()) }
        ?: WebResourceResponse("image/png", null, 404, "No tile", emptyMap(), ByteArray(0).inputStream())

@SuppressLint("SetJavaScriptEnabled")
private fun syntheticWebView(context: Context, onLoaded: () -> Unit): WebView {
    val assets = WebViewAssetLoader.Builder().addPathHandler("/assets/", WebViewAssetLoader.AssetsPathHandler(context)).build()
    WebView.setWebContentsDebuggingEnabled(BuildConfig.DEBUG)
    return WebView(context).apply {
        layoutParams = ViewGroup.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT)
        setBackgroundColor(android.graphics.Color.BLACK)
        settings.javaScriptEnabled = true
        webViewClient = object : WebViewClientCompat() {
            override fun shouldInterceptRequest(view: WebView, request: WebResourceRequest): WebResourceResponse? =
                syntheticTilePath(request.url.encodedPath)?.let(::tileResponse) ?: assets.shouldInterceptRequest(request.url)

            override fun onPageFinished(view: WebView, url: String) = onLoaded()
        }
        loadUrl(SYNTHETIC_PAGE)
    }
}

@Composable
internal fun SyntheticView(modifier: Modifier = Modifier, labelled: Boolean = true, aimable: Boolean = false) {
    val view by qgcPath(SYNTHETIC_VIEW)
    val context = LocalContext.current
    val overlays by qgcPath(SYNTHETIC_OVERLAYS)
    var loaded by remember { mutableStateOf(false) }
    val web = remember { syntheticWebView(context) { loaded = true } }
    DisposableEffect(web) { onDispose { web.destroy() } }
    LaunchedEffect(view) { view?.let { web.evaluateJavascript(syntheticPoseScript(it), null) } }
    LaunchedEffect(overlays, loaded) { overlays?.takeIf { loaded }?.let { web.evaluateJavascript(syntheticOverlaysScript(it), null) } }
    val canAim = aimable && view?.optBoolean("aimable") == true
    val pitch by rememberUpdatedState(view?.optDouble("pitch", DEFAULT_TILT_DEG) ?: DEFAULT_TILT_DEG)
    val pan by rememberUpdatedState(view?.optDouble("pan", 0.0) ?: 0.0)
    val fov by rememberUpdatedState(view?.optDouble("fov", DEFAULT_FOV_DEG) ?: DEFAULT_FOV_DEG)
    val aiming = Modifier
        .pointerInput(canAim) {
            if (!canAim) return@pointerInput
            var aim = pitch to pan
            detectDragGestures(onDragStart = { aim = pitch to pan }) { change, dragged ->
                change.consume()
                aim = syntheticTilt(aim.first, dragged.y, size.height) to syntheticPan(aim.second, dragged.x, size.width, fov)
                val (tilt, turned) = aim
                offMainInOrder { Qgc.invoke(SYNTHETIC_AIM, tilt, turned) }
            }
        }
        .pointerInput(canAim) {
            if (!canAim) return@pointerInput
            detectTapGestures(onDoubleTap = { offMainInOrder { Qgc.invoke(SYNTHETIC_AIM, DEFAULT_TILT_DEG, 0.0) } })
        }
    Box(modifier.then(aiming)) {
        AndroidView({ web }, Modifier.fillMaxSize())
        if (labelled) SyntheticTag(Modifier.align(Alignment.TopEnd).padding(6.dp))
    }
}

@Composable
internal fun SyntheticTag(modifier: Modifier = Modifier) {
    Text(
        SYNTHETIC_LABEL,
        modifier.background(Color.Black.copy(alpha = 0.45f), RoundedCornerShape(4.dp)).padding(horizontal = 6.dp, vertical = 2.dp),
        color = Color.White,
        style = MaterialTheme.typography.labelSmall,
    )
}
