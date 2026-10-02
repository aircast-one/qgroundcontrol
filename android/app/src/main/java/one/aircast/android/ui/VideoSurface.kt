package one.aircast.android.ui

import android.view.SurfaceHolder
import android.view.SurfaceView
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.requiredSize
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcPath
import one.aircast.android.bridge.qgcValue
import one.aircast.android.bridge.settingControl
import org.mavlink.qgroundcontrol.QGCBridge

@Composable
fun VideoSurface(
    modifier: Modifier = Modifier,
    expanded: Boolean = false,
    fullScreen: Boolean = false,
    onClick: () -> Unit = {},
    onDoubleTap: () -> Unit = {},
) {
    val videoJson by qgcPath(VIDEO_VIEW)
    val video = remember(videoJson) { videoReading(videoJson) }

    if (video?.available == false) {
        return
    }

    val showGrid by qgcBool(settingControl("settings.videoSettings.gridLines"))
    val fitMode by qgcValue(settingControl("settings.videoSettings.videoFit"))
    val aspectSetting by qgcValue(settingControl("settings.videoSettings.aspectRatio"))
    val aspect = videoAspect(video?.sourceSize, (aspectSetting as? Number)?.toDouble() ?: aspectSetting?.toString()?.toDoubleOrNull())

    BoxWithConstraints(
        (if (expanded) modifier.pointerInput(video?.decoding) { if (video?.decoding == true) detectTapGestures(onDoubleTap = { onDoubleTap() }) } else modifier.clickable { onClick() }).clipToBounds(),
        contentAlignment = Alignment.Center,
    ) {
        val (contentWidth, contentHeight) = videoContentSize(maxWidth.value, maxHeight.value, aspect, (fitMode as? Number)?.toInt() ?: fitMode?.toString()?.toIntOrNull() ?: VIDEO_FIT_HEIGHT)
        Box(Modifier.requiredSize(contentWidth.dp, contentHeight.dp)) {
        AndroidView(
            modifier = Modifier.fillMaxSize(),
            factory = { context ->
                SurfaceView(context).apply {
                    holder.addCallback(object : SurfaceHolder.Callback {
                        override fun surfaceCreated(holder: SurfaceHolder) {
                            QGCBridge.videoSetSurface(holder.surface)
                        }

                        override fun surfaceChanged(
                            holder: SurfaceHolder,
                            format: Int,
                            width: Int,
                            height: Int,
                        ) {
                            QGCBridge.videoSetSurface(holder.surface)
                        }

                        override fun surfaceDestroyed(holder: SurfaceHolder) {
                            QGCBridge.videoSetSurface(null)
                        }
                    })
                }
            },
        )

        if (video?.decoding == true) {
            if (showGrid && !fullScreen) VideoGrid(Modifier.fillMaxSize())
            ProximityRadarOverlay(Modifier.fillMaxSize())
            if (expanded) GimbalScreenControl(Modifier.fillMaxSize())
            DetectionOverlay(Modifier.fillMaxSize())
            TrackingBoxOverlay(Modifier.fillMaxSize())
            if (expanded) VideoStatsPill(Modifier.fillMaxSize())
        }
        }

        if (video?.decoding != true) {
            Surface(
                Modifier.fillMaxSize(),
                color = MaterialTheme.colorScheme.surfaceVariant,
            ) {
                Box(contentAlignment = Alignment.Center) {
                    NoVideoPanel(video)
                }
            }
        }

    }
}

internal const val VIDEO_FIT_WIDTH = 0
internal const val VIDEO_FIT_HEIGHT = 1
internal const val VIDEO_FILL = 2
internal const val VIDEO_NO_CROP = 3
private const val GRID_FRACTIONS = 3
private val GRID_COLOUR = Color(1f, 1f, 1f, 0.5f)

internal fun videoAspect(source: SourceSize?, setting: Double?): Double =
    source?.takeIf { it.width > 0 && it.height > 0 }?.let { it.width.toDouble() / it.height } ?: setting?.takeIf { it > 0 } ?: 0.0

internal fun videoContentSize(width: Float, height: Float, aspect: Double, fit: Int): Pair<Float, Float> {
    if (aspect == 0.0 || width <= 0f || height <= 0f) return width to height
    val box = width / height
    val shownWidth = when {
        fit == VIDEO_FIT_HEIGHT || (fit == VIDEO_FILL && box < aspect) || (fit == VIDEO_NO_CROP && box > aspect) -> (height * aspect).toFloat()
        else -> width
    }
    val shownHeight = when {
        fit == VIDEO_FIT_WIDTH || (fit == VIDEO_FILL && box > aspect) || (fit == VIDEO_NO_CROP && box < aspect) -> (width / aspect).toFloat()
        else -> height
    }
    return shownWidth to shownHeight
}

@Composable
private fun VideoGrid(modifier: Modifier) {
    Canvas(modifier) {
        (1 until GRID_FRACTIONS).forEach { line ->
            val x = size.width * line * 0.33f
            val y = size.height * line * 0.33f
            drawLine(GRID_COLOUR, Offset(x, 0f), Offset(x, size.height), strokeWidth = 1f)
            drawLine(GRID_COLOUR, Offset(0f, y), Offset(size.width, y), strokeWidth = 1f)
        }
    }
}
