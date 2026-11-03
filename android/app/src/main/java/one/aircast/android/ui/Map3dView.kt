package one.aircast.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.calculatePan
import androidx.compose.foundation.gestures.calculateRotation
import androidx.compose.foundation.gestures.calculateZoom
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChanged
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import one.aircast.android.R
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainInOrder
import one.aircast.android.bridge.qgcPath
import one.aircast.map.AircastSpace
import one.aircast.map.FlightMapPosition
import one.aircast.map.aircast
import one.aircast.map.readCamera
import org.json.JSONObject

private const val MAP3D_VIEW = "view.map3d"
private const val MAP3D_PAN = "map3d.pan"
private const val MAP3D_TURN = "map3d.turn"
private const val MAP3D_TILT = "map3d.tilt"
private const val MAP3D_ZOOM = "map3d.zoom"
private const val MAP3D_FOLLOW = "map3d.follow"
private const val MAP3D_NORTH = "map3d.north"
private const val MAP3D_OPEN = "map3d.open"
private const val TURN_PER_VIEW_WIDTH_DEG = 180f
private const val CONTROL_SCRIM_ALPHA = 0.55f
private val CONTROL_SIZE = 40.dp
private val CONTROL_ICON = 20.dp
internal const val BACK_TO_AIRCRAFT = "Back to aircraft"
private const val COMPASS_LABEL = "Compass. Drag to turn and tilt the view, tap to face north"

internal data class Map3dState(val heading: Double, val following: Boolean, val aircraft: Boolean)

internal fun map3dState(view: JSONObject?): Map3dState =
    Map3dState(view?.optDouble("heading", 0.0) ?: 0.0, view?.optBoolean("following", true) ?: true, view?.optBoolean("aircraft") == true)

internal fun map3dNote(synthetic: JSONObject?, state: Map3dState): String? = when {
    synthetic?.optBoolean("available") != true && state.following -> LOOKING_TITLE
    else -> nativeDraw(synthetic).note
}

private fun send(path: String, vararg args: Number) = offMainInOrder { Qgc.call(path, *args) }

private fun Modifier.map3dGestures(): Modifier = pointerInput(Unit) {
    detectTapGestures(onDoubleTap = { send(MAP3D_FOLLOW) })
}.pointerInput(Unit) {
    awaitEachGesture {
        awaitFirstDown(requireUnconsumed = false)
        var travelled = Offset.Zero
        var pinched = false
        do {
            val event = awaitPointerEvent()
            val fingers = event.changes.count { it.pressed }
            val pan = event.calculatePan()
            travelled += pan
            pinched = pinched || fingers > 1
            if (fingers > 1) {
                event.calculateZoom().takeIf { it != 1f }?.let { send(MAP3D_ZOOM, it) }
                event.calculateRotation().takeIf { it != 0f }?.let { send(MAP3D_TURN, it) }
                pan.y.takeIf { it != 0f }?.let { send(MAP3D_TILT, -it, size.height) }
            } else if (!pinched && travelled.getDistance() > viewConfiguration.touchSlop && pan != Offset.Zero) {
                send(MAP3D_PAN, pan.x, pan.y, size.width)
            }
            if (pinched || travelled.getDistance() > viewConfiguration.touchSlop) {
                event.changes.filter { it.positionChanged() }.forEach { it.consume() }
            }
        } while (event.changes.any { it.pressed })
    }
}

@Composable
private fun Compass(heading: Double, view: IntSize) {
    Surface(
        shape = CircleShape,
        color = Color.Black.copy(alpha = CONTROL_SCRIM_ALPHA),
        contentColor = MaterialTheme.aircast.outdoorForeground,
        modifier = Modifier.size(CONTROL_SIZE)
            .semantics { contentDescription = COMPASS_LABEL }
            .pointerInput(Unit) { detectTapGestures(onTap = { send(MAP3D_NORTH) }) }
            .pointerInput(view) {
                detectDragGestures { change, dragged ->
                    change.consume()
                    send(MAP3D_TURN, dragged.x / view.width.coerceAtLeast(1) * TURN_PER_VIEW_WIDTH_DEG)
                    send(MAP3D_TILT, -dragged.y, view.height)
                }
            },
    ) {
        Box(contentAlignment = Alignment.Center) {
            Icon(painterResource(R.drawable.ic_navigation), null, Modifier.size(CONTROL_ICON).rotate(-heading.toFloat()))
        }
    }
}

@Composable
private fun BackToAircraft() {
    Surface(
        onClick = { send(MAP3D_FOLLOW) },
        shape = CircleShape,
        color = Color.Black.copy(alpha = CONTROL_SCRIM_ALPHA),
        contentColor = MaterialTheme.aircast.outdoorForeground,
    ) {
        Row(
            Modifier.padding(horizontal = AircastSpace.s3, vertical = AircastSpace.s2),
            horizontalArrangement = Arrangement.spacedBy(AircastSpace.s2),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(painterResource(R.drawable.ic_my_location), null, Modifier.size(CONTROL_ICON))
            Text(BACK_TO_AIRCRAFT, style = MaterialTheme.typography.labelLarge)
        }
    }
}

@Composable
internal fun Map3dView(modifier: Modifier, bottomInsetPx: Int, logoEndInsetPx: Int?) {
    val context = LocalContext.current
    val synthetic by qgcPath(SYNTHETIC_VIEW)
    val map3dJson by qgcPath(MAP3D_VIEW)
    val state = map3dState(map3dJson)
    var size by remember { mutableStateOf(IntSize.Zero) }
    LaunchedEffect(Unit) {
        (FlightMapPosition.latest ?: readCamera(context)?.centre)
            ?.let { send(MAP3D_OPEN, it.latitude, it.longitude) }
            ?: send(MAP3D_OPEN)
    }
    val controlsAt = with(LocalDensity.current) {
        logoEndInsetPx?.let { Modifier.padding(end = it.toDp(), bottom = bottomInsetPx.toDp() + AircastSpace.s3) }
            ?: Modifier.padding(AircastSpace.s3)
    }
    Box(modifier.background(Color.Black).onSizeChanged { size = it }) {
        OrbitSurface(Modifier.fillMaxSize())
        Box(Modifier.fillMaxSize().map3dGestures())
        map3dNote(synthetic, state)?.let { note ->
            Text(
                note,
                Modifier.align(Alignment.Center).padding(AircastSpace.s4).osdShadow(),
                style = MaterialTheme.typography.labelLarge,
                color = MaterialTheme.aircast.outdoorForeground,
                textAlign = TextAlign.Center,
            )
        }
        Row(
            Modifier.align(Alignment.BottomEnd).then(controlsAt),
            horizontalArrangement = Arrangement.spacedBy(AircastSpace.s2),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            if (state.aircraft && !state.following) BackToAircraft()
            Compass(state.heading, size)
        }
    }
}
