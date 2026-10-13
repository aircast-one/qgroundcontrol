package one.aircast.android.ui

import android.view.SurfaceHolder
import android.view.SurfaceView
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.viewinterop.AndroidView
import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge
import kotlin.math.roundToInt

internal const val RENDER_SCALE = 0.75f

internal fun renderSize(viewPx: Int): Int = (viewPx * RENDER_SCALE).roundToInt().coerceAtLeast(1)

internal fun nativeChosen(view: JSONObject?): Boolean = view?.optBoolean("native") == true

internal fun nativeDraw(view: JSONObject?): SyntheticDraw = when (view?.optString("draw")) {
    "drawn" -> SyntheticDraw.Drawn
    "failed" -> SyntheticDraw.Failed
    else -> SyntheticDraw.Loading
}

private val surfaceHandoff = object : SurfaceHolder.Callback {
    override fun surfaceCreated(holder: SurfaceHolder) = Unit

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        QGCBridge.syntheticAttach(holder.surface)
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        QGCBridge.syntheticRelease(holder.surface)
    }
}

@Composable
internal fun SyntheticSurface(modifier: Modifier = Modifier) {
    AndroidView({ context ->
        SurfaceView(context).apply {
            holder.addCallback(surfaceHandoff)
            addOnLayoutChangeListener { _, left, top, right, bottom, _, _, _, _ ->
                if (right > left && bottom > top) holder.setFixedSize(renderSize(right - left), renderSize(bottom - top))
            }
        }
    }, modifier)
}
