package one.aircast.android.ui

import android.graphics.Bitmap
import android.graphics.BitmapFactory
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
private const val OPAQUE = 0xFF shl 24

internal data class FlowFrame(val raw: Boolean, val width: Int, val height: Int, val bytes: ByteArray)

internal fun flowFrame(view: JSONObject?): FlowFrame? = view?.optJSONObject("image")?.let {
    FlowFrame(it.optBoolean("raw"), it.optInt("width"), it.optInt("height"), Base64.decode(it.optString("data"), Base64.DEFAULT))
}

internal fun greyPixels(bytes: ByteArray, count: Int): IntArray =
    IntArray(count) { at -> (bytes.getOrElse(at) { 0 }.toInt() and 0xFF).let { grey -> OPAQUE or (grey shl 16) or (grey shl 8) or grey } }

private fun bitmapOf(frame: FlowFrame): ImageBitmap? = when {
    frame.raw && frame.width > 0 && frame.height > 0 ->
        Bitmap.createBitmap(greyPixels(frame.bytes, frame.width * frame.height), frame.width, frame.height, Bitmap.Config.ARGB_8888).asImageBitmap()
    else -> BitmapFactory.decodeByteArray(frame.bytes, 0, frame.bytes.size)?.asImageBitmap()
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
