package one.aircast.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.calculatePan
import androidx.compose.foundation.gestures.calculateRotation
import androidx.compose.foundation.gestures.calculateZoom
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChanged
import androidx.compose.ui.text.style.TextAlign
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainInOrder
import one.aircast.android.bridge.qgcPath
import one.aircast.map.AircastSpace
import one.aircast.map.aircast
import org.json.JSONObject

private const val MAP3D_PAN = "map3d.pan"
private const val MAP3D_TURN = "map3d.turn"
private const val MAP3D_TILT = "map3d.tilt"
private const val MAP3D_ZOOM = "map3d.zoom"
private const val MAP3D_FOLLOW = "map3d.follow"

internal fun map3dNote(view: JSONObject?): String? = when {
    view?.optBoolean("available") != true -> LOOKING_TITLE
    else -> nativeDraw(view).note
}

private fun send(path: String, vararg args: Number) = offMainInOrder { Qgc.call(path, *args) }

private fun Modifier.map3dGestures(): Modifier = pointerInput(Unit) {
    detectTapGestures(onDoubleTap = { send(MAP3D_FOLLOW) })
}.pointerInput(Unit) {
    awaitEachGesture {
        awaitFirstDown(requireUnconsumed = false)
        var travelled = Offset.Zero
        do {
            val event = awaitPointerEvent()
            val fingers = event.changes.count { it.pressed }
            val pan = event.calculatePan()
            travelled += pan
            if (fingers > 1 || travelled.getDistance() > viewConfiguration.touchSlop) {
                if (fingers > 1) {
                    send(MAP3D_ZOOM, event.calculateZoom())
                    send(MAP3D_TURN, event.calculateRotation())
                    send(MAP3D_TILT, -pan.y, size.height)
                } else {
                    send(MAP3D_PAN, pan.x, pan.y, size.width)
                }
                event.changes.filter { it.positionChanged() }.forEach { it.consume() }
            }
        } while (event.changes.any { it.pressed })
    }
}

@Composable
internal fun Map3dView(modifier: Modifier = Modifier) {
    val view by qgcPath(SYNTHETIC_VIEW)
    Box(modifier.background(Color.Black).map3dGestures()) {
        OrbitSurface(Modifier.fillMaxSize())
        map3dNote(view)?.let { note ->
            Text(
                note,
                Modifier.align(Alignment.Center).padding(AircastSpace.s4).osdShadow(),
                style = MaterialTheme.typography.labelLarge,
                color = MaterialTheme.aircast.outdoorForeground,
                textAlign = TextAlign.Center,
            )
        }
    }
}
