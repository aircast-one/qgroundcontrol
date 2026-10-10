package one.aircast.android.ui

import androidx.compose.runtime.getValue
import one.aircast.map.ObstacleVideoOverlay
import one.aircast.map.aircast
import android.view.SurfaceHolder
import android.view.SurfaceView
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTapGestures
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.offMainInOrder
import one.aircast.android.bridge.VideoCommands
import java.util.concurrent.atomic.AtomicInteger
import one.aircast.android.bridge.Qgc
import androidx.compose.foundation.gestures.calculateZoom
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.requiredSize
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.geometry.Rect
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

internal const val CAMERA_STEP_ZOOM = "vehicle.cameraManager.currentCameraInstance.stepZoom"

internal fun pinchStep(scale: Float): Int = if (scale < 1f) Math.round(scale * -10f) else Math.round(scale)

private fun Modifier.pinchZoom(enabled: Boolean): Modifier = if (!enabled) this else pointerInput(Unit) {
    awaitEachGesture {
        awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
        var scale = 1f
        var sent = 0
        do {
            val event = awaitPointerEvent(PointerEventPass.Initial)
            if (event.changes.count { it.pressed } >= 2) {
                event.changes.forEach { it.consume() }
                scale *= event.calculateZoom()
                val step = pinchStep(scale)
                if (step != sent) offMainDetached { Qgc.invoke(CAMERA_STEP_ZOOM, step) }
                sent = step
            }
        } while (event.changes.any { it.pressed })
    }
}

@Composable
private fun NoVideoArea(underFlyChrome: Boolean, thumb: Boolean, video: VideoReading?) {
    if (underFlyChrome) return
    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        CompositionLocalProvider(LocalNoVideoSize provides if (thumb) NoVideoSize.Thumb else NoVideoSize.Full) { NoVideoPanel(video) }
    }
}

internal fun looksForAircraft(connected: Boolean, video: VideoReading?): Boolean = !connected && video?.available != true

@Composable
internal fun FlyNoVideoMessage() {
    val videoJson by qgcPath(VIDEO_VIEW)
    val video = remember(videoJson) { videoReading(videoJson) }
    if (looksForAircraft(hasVehicle(), video)) {
        LookingForAircraft()
        return
    }
    if (rememberSyntheticAvailable()) return
    BoxWithConstraints(Modifier.fillMaxSize()) {
        val compact = maxHeight < NO_VIDEO_FULL_HEIGHT || maxWidth < NO_VIDEO_FULL_WIDTH
        Box(Modifier.fillMaxSize(), contentAlignment = if (compact) Alignment.TopEnd else Alignment.Center) {
            CompositionLocalProvider(LocalNoVideoSize provides if (compact) NoVideoSize.Pill else NoVideoSize.Full) { NoVideoPanel(video) }
        }
    }
}

internal fun centredRegion(region: Rect, centreX: Float, minWidth: Float): Rect {
    val half = minOf(centreX - region.left, region.right - centreX)
    return if (half * 2 >= minWidth) region.copy(left = centreX - half, right = centreX + half) else region
}

internal fun messageRegion(free: Rect, obstacles: List<Rect>, gap: Float, minWidth: Float, minHeight: Float): Rect? {
    val fits = { room: Rect -> room.width >= minWidth && room.height >= minHeight }
    return obstacles.filterNot { it.isEmpty }
        .fold(listOf(free).filter(fits)) { rooms, obstacle ->
            rooms.flatMap { room -> if (room.overlaps(obstacle)) sidesAround(room, obstacle, gap).filter(fits) else listOf(room) }
                .distinct()
                .let { found -> found.filterNot { room -> found.any { other -> other != room && other.contains(room) } } }
        }
        .maxByOrNull { it.width * it.height }
}

private fun Rect.contains(other: Rect): Boolean = left <= other.left && top <= other.top && right >= other.right && bottom >= other.bottom

private fun sidesAround(room: Rect, obstacle: Rect, gap: Float): List<Rect> = listOf(
    Rect(obstacle.right + gap, room.top, room.right, room.bottom),
    Rect(room.left, room.top, obstacle.left - gap, room.bottom),
    Rect(room.left, obstacle.bottom + gap, room.right, room.bottom),
    Rect(room.left, room.top, room.right, obstacle.top - gap),
)

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
    val synthetic = rememberSyntheticAvailable()

    if (video?.available == false) {
        if (!expanded) return
        if (synthetic) {
            SyntheticView(modifier.fillMaxSize(), aimable = true)
            return
        }
        Surface(modifier.fillMaxSize(), color = MaterialTheme.colorScheme.surfaceVariant) {
            NoVideoArea(expanded && !fullScreen, thumb = false, video = video)
        }
        return
    }

    val cameraJson by qgcPath(CAMERA_VIEW)
    val zoomable = remember(cameraJson) { cameraReading(cameraJson)?.hasZoom == true }
    val showGrid by qgcBool(settingControl("settings.videoSettings.gridLines"))
    val fitMode by qgcValue(settingControl("settings.videoSettings.videoFit"))
    val aspectSetting by qgcValue(settingControl("settings.videoSettings.aspectRatio"))
    val aspect = videoAspect(video?.sourceSize, (aspectSetting as? Number)?.toDouble() ?: aspectSetting?.toString()?.toDoubleOrNull())

    BoxWithConstraints(
        (if (expanded) modifier.pinchZoom(zoomable).pointerInput(video?.decoding) { if (video?.decoding == true) detectTapGestures(onDoubleTap = { onDoubleTap() }) } else modifier.clickable { onClick() }).clipToBounds(),
        contentAlignment = Alignment.Center,
    ) {
        val (contentWidth, contentHeight) = videoContentSize(maxWidth.value, maxHeight.value, aspect, (fitMode as? Number)?.toInt() ?: fitMode?.toString()?.toIntOrNull() ?: VIDEO_FIT_HEIGHT)
        Box(Modifier.requiredSize(contentWidth.dp, contentHeight.dp)) {
        VideoChannelSurface(MAIN_VIDEO_CHANNEL, Modifier.fillMaxSize())

        if (video?.decoding == true) {
            if (showGrid && !fullScreen) VideoGrid(Modifier.fillMaxSize())
            ProximityRadarOverlay(Modifier.fillMaxSize())
            ObstacleVideoOverlay(Modifier.fillMaxSize(), showText = expanded)
            if (expanded) GimbalScreenControl(Modifier.fillMaxSize())
            DetectionOverlay(Modifier.fillMaxSize())
            TrackingBoxOverlay(Modifier.fillMaxSize())
            if (expanded) VideoStatsPill(Modifier.fillMaxSize())
        }
        }

        if (video?.decoding != true && synthetic) {
            SyntheticView(Modifier.fillMaxSize(), aimable = expanded)
        } else if (video?.decoding != true) {
            Surface(
                Modifier.fillMaxSize(),
                color = if (expanded) MaterialTheme.colorScheme.surfaceVariant else MaterialTheme.aircast.outdoorBackground,
            ) {
                NoVideoArea(expanded && !fullScreen, thumb = !expanded, video = video)
            }
        }

    }
}

internal const val MAIN_VIDEO_CHANNEL = 0
internal const val PIP_VIDEO_CHANNEL = 1

internal class PipSurfaces(private val report: (Boolean) -> Unit) {
    private val attached = AtomicInteger()
    fun created() = report(attached.incrementAndGet() > 0)
    fun destroyed() = report(attached.decrementAndGet() > 0)
}

private val pipSurfaces = PipSurfaces { shown -> offMainInOrder { VideoCommands.setPipShown(shown) } }

@Composable
fun VideoChannelSurface(channel: Int, modifier: Modifier = Modifier) {
    AndroidView(
        modifier = modifier,
        factory = { context ->
            SurfaceView(context).apply {
                if (channel != MAIN_VIDEO_CHANNEL) setZOrderMediaOverlay(true)
                holder.addCallback(object : SurfaceHolder.Callback {
                    override fun surfaceCreated(holder: SurfaceHolder) {
                        QGCBridge.videoSetSurface(channel, holder.surface)
                        if (channel == PIP_VIDEO_CHANNEL) pipSurfaces.created()
                    }

                    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
                        QGCBridge.videoSetSurface(channel, holder.surface)
                    }

                    override fun surfaceDestroyed(holder: SurfaceHolder) {
                        QGCBridge.videoSetSurface(channel, null)
                        if (channel == PIP_VIDEO_CHANNEL) pipSurfaces.destroyed()
                    }
                })
            }
        },
    )
}

private val NO_VIDEO_FULL_HEIGHT = 230.dp
private val NO_VIDEO_FULL_WIDTH = 260.dp
internal val NO_VIDEO_PILL_WIDTH = 180.dp
internal val NO_VIDEO_PILL_HEIGHT = 48.dp
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
