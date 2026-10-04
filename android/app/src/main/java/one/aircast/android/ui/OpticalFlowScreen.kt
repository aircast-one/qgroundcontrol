package one.aircast.android.ui

import android.graphics.Bitmap
import android.util.Base64
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import org.json.JSONObject

internal const val OPTICAL_FLOW_SCREEN = "opticalFlow"
private const val OPTICAL_FLOW_VIEW = "view.opticalFlow"
private const val OPTICAL_FLOW_POLL_MS = 500L

internal data class FlowFrame(val width: Int, val height: Int, val rgba: ByteArray)

internal fun flowFrame(view: JSONObject?): FlowFrame? = view?.optJSONObject("image")?.let {
    FlowFrame(it.optInt("width"), it.optInt("height"), Base64.decode(it.optString("data"), Base64.DEFAULT))
}

private fun channel(bytes: ByteArray, at: Int): Int = bytes.getOrElse(at) { 0 }.toInt() and 0xFF

internal fun argbPixels(rgba: ByteArray, count: Int): IntArray =
    IntArray(count) { pixel ->
        (pixel * 4).let { at -> (channel(rgba, at + 3) shl 24) or (channel(rgba, at) shl 16) or (channel(rgba, at + 1) shl 8) or channel(rgba, at + 2) }
    }

private fun bitmapOf(frame: FlowFrame): ImageBitmap? = frame.takeIf { it.width > 0 && it.height > 0 }?.let {
    Bitmap.createBitmap(argbPixels(it.rgba, it.width * it.height), it.width, it.height, Bitmap.Config.ARGB_8888).asImageBitmap()
}

@Composable
fun OpticalFlowScreen(modifier: Modifier = Modifier) {
    var index by remember { mutableIntStateOf(-1) }
    var image by remember { mutableStateOf<ImageBitmap?>(null) }
    LaunchedEffect(Unit) {
        while (true) {
            val view = withContext(Dispatchers.Default) { Qgc.get(OPTICAL_FLOW_VIEW) }
            val latest = view?.optInt("index") ?: 0
            if (latest != index) {
                index = latest
                image = withContext(Dispatchers.Default) { flowFrame(view)?.let(::bitmapOf) }
            }
            delay(OPTICAL_FLOW_POLL_MS)
        }
    }
    Column(modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp), horizontalAlignment = Alignment.CenterHorizontally) {
        Text("Optical flow camera", style = MaterialTheme.typography.titleMedium)
        image?.let { Image(it, contentDescription = "Optical flow camera image", contentScale = ContentScale.Fit, modifier = Modifier.fillMaxWidth(0.5f).aspectRatio(4f / 3f)) }
    }
}
