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
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.foundation.gestures.detectVerticalDragGestures
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
import one.aircast.map.QGC_TILE_HOST
import one.aircast.map.coreTile
import one.aircast.map.tileMimeType
import org.json.JSONObject

internal const val SYNTHETIC_VIEW = "view.syntheticView"
internal const val SYNTHETIC_LABEL = "Synthetic view"
internal const val SYNTHETIC_SOURCE = "Synthetic View"
internal const val SYNTHETIC_TILT = "syntheticView.tilt"
private const val TILT_SPAN_DEG = 90.0
private const val DEFAULT_TILT_DEG = -15.0
private const val SYNTHETIC_PAGE = "https://${WebViewAssetLoader.DEFAULT_DOMAIN}/assets/synthetic/index.html"
private val ANY_ORIGIN = mapOf("Access-Control-Allow-Origin" to "*")

internal fun syntheticAvailable(view: JSONObject?): Boolean = view?.optBoolean("available") == true

internal fun syntheticPoseScript(view: JSONObject): String = "window.aircast && window.aircast.pose($view)"

internal fun syntheticTilt(from: Double, draggedPx: Float, heightPx: Int): Double =
    (from + draggedPx.toDouble() / heightPx.coerceAtLeast(1) * TILT_SPAN_DEG).coerceIn(-TILT_SPAN_DEG, 0.0)

@Composable
internal fun rememberSyntheticAvailable(): Boolean {
    val view by qgcPath(SYNTHETIC_VIEW)
    return syntheticAvailable(view)
}

private fun tileResponse(encodedPath: String): WebResourceResponse =
    coreTile(encodedPath)?.let { WebResourceResponse(tileMimeType(it), null, 200, "OK", ANY_ORIGIN, it.inputStream()) }
        ?: WebResourceResponse("image/png", null, 404, "No tile", ANY_ORIGIN, ByteArray(0).inputStream())

@SuppressLint("SetJavaScriptEnabled")
private fun syntheticWebView(context: Context): WebView {
    val assets = WebViewAssetLoader.Builder().addPathHandler("/assets/", WebViewAssetLoader.AssetsPathHandler(context)).build()
    WebView.setWebContentsDebuggingEnabled(BuildConfig.DEBUG)
    return WebView(context).apply {
        layoutParams = ViewGroup.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT)
        setBackgroundColor(android.graphics.Color.BLACK)
        settings.javaScriptEnabled = true
        webViewClient = object : WebViewClientCompat() {
            override fun shouldInterceptRequest(view: WebView, request: WebResourceRequest): WebResourceResponse? =
                if (request.url.host == QGC_TILE_HOST) tileResponse(request.url.encodedPath.orEmpty()) else assets.shouldInterceptRequest(request.url)
        }
        loadUrl(SYNTHETIC_PAGE)
    }
}

@Composable
internal fun SyntheticView(modifier: Modifier = Modifier, labelled: Boolean = true, tiltable: Boolean = false) {
    val view by qgcPath(SYNTHETIC_VIEW)
    val context = LocalContext.current
    val web = remember { syntheticWebView(context) }
    DisposableEffect(web) { onDispose { web.destroy() } }
    LaunchedEffect(view) { view?.let { web.evaluateJavascript(syntheticPoseScript(it), null) } }
    val canTilt = tiltable && view?.optBoolean("tiltable") == true
    val pitch by rememberUpdatedState(view?.optDouble("pitch", DEFAULT_TILT_DEG) ?: DEFAULT_TILT_DEG)
    val tilting = Modifier.pointerInput(canTilt) {
        if (!canTilt) return@pointerInput
        var tilt = pitch
        detectVerticalDragGestures(onDragStart = { tilt = pitch }) { change, dragged ->
            change.consume()
            tilt = syntheticTilt(tilt, dragged, size.height)
            val sent = tilt
            offMainInOrder { Qgc.invoke(SYNTHETIC_TILT, sent) }
        }
    }
    Box(modifier.then(tilting)) {
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
