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
private const val SYNTHETIC_PAGE = "https://${WebViewAssetLoader.DEFAULT_DOMAIN}/assets/synthetic/index.html"
private val ANY_ORIGIN = mapOf("Access-Control-Allow-Origin" to "*")

internal fun syntheticAvailable(view: JSONObject?): Boolean = view?.optBoolean("available") == true

internal fun syntheticPoseScript(view: JSONObject): String = "window.aircast && window.aircast.pose($view)"

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
internal fun SyntheticView(modifier: Modifier = Modifier, labelled: Boolean = true) {
    val view by qgcPath(SYNTHETIC_VIEW)
    val context = LocalContext.current
    val web = remember { syntheticWebView(context) }
    DisposableEffect(web) { onDispose { web.destroy() } }
    LaunchedEffect(view) { view?.let { web.evaluateJavascript(syntheticPoseScript(it), null) } }
    Box(modifier) {
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
