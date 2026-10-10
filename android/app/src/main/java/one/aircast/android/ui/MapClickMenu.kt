package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import one.aircast.map.AircastSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Slider
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcPath
import one.aircast.map.LoiterEdit
import one.aircast.map.MINIMUM_CIRCLE_RADIUS_METRES
import one.aircast.map.OrbitCircle
import one.aircast.map.loiterEditNumber
import one.aircast.map.optText
import one.aircast.map.aircast
import one.aircast.map.TrackPoint
import org.json.JSONObject
import java.util.Locale

internal const val MAP_CLICK_PATH = "view.mapClick"

internal data class MapPoint(val latitude: Double, val longitude: Double)

internal const val ORBIT_ACTION = "Orbit"

internal const val RETURN_HOME_INSTEAD = "Return home instead"

internal const val MAP_HOLD_HINT = "Press and hold the map to fly there"

enum class MapHoldHint { Unseen, Due, Learned }

internal data class GotoPreview(val distance: String, val height: String, val verdict: String, val pastReturnPoint: Boolean)

internal fun gotoPreviewPath(point: MapPoint): String = String.format(Locale.US, "view.gotoPreview(%.7f,%.7f)", point.latitude, point.longitude)

internal fun gotoPreview(view: JSONObject?): GotoPreview? =
    view?.takeIf { it.optBoolean("available") }?.let {
        GotoPreview(it.optText("distanceText"), it.optText("heightText"), it.optText("verdict"), it.optBoolean("pastReturnPoint"))
    }

internal fun gotoPreviewLines(preview: GotoPreview?, fallback: String): List<String> =
    preview?.let { listOf(listOf(it.distance, it.height).filter(String::isNotBlank).joinToString(" \u00b7 "), it.verdict).filter(String::isNotBlank) }?.ifEmpty { null } ?: listOf(fallback)
internal const val GOTO_ACTION = "GoTo"

private val COMPASS_POINTS = listOf("north", "north-east", "east", "south-east", "south", "south-west", "west", "north-west")

internal fun goHereText(from: MapPoint, to: MapPoint, unit: String, metresPerUnit: Double): String? {
    if (metresPerUnit <= 0.0 || unit.isBlank()) return null
    val metres = one.aircast.map.metresBetween(one.aircast.map.TrackPoint(from.latitude, from.longitude), one.aircast.map.TrackPoint(to.latitude, to.longitude))
    val fromLat = Math.toRadians(from.latitude)
    val toLat = Math.toRadians(to.latitude)
    val deltaLon = Math.toRadians(to.longitude - from.longitude)
    val bearing = (Math.toDegrees(Math.atan2(Math.sin(deltaLon) * Math.cos(toLat), Math.cos(fromLat) * Math.sin(toLat) - Math.sin(fromLat) * Math.cos(toLat) * Math.cos(deltaLon))) + 360.0) % 360.0
    val point = COMPASS_POINTS[(Math.round(bearing / 45.0).toInt()) % COMPASS_POINTS.size]
    return "${Math.round(metres / metresPerUnit)} $unit $point"
}

internal data class MapClickAction(
    val id: String,
    val path: String,
    val label: String,
    val title: String,
    val message: String,
    val confirm: Boolean,
)

internal fun mapClickActions(view: JSONObject?): List<MapClickAction> {
    val listed = view?.optJSONArray("actions") ?: return emptyList()
    return (0 until listed.length()).mapNotNull { index ->
        listed.optJSONObject(index)?.let {
            MapClickAction(
                id = it.optText("id"),
                path = it.optText("path"),
                label = it.optText("label"),
                title = it.optText("title"),
                message = it.optText("message"),
                confirm = it.optBoolean("confirm", true),
            )
        }
    }
}

internal data class OrbitChoice(val radiusMetres: Double, val clockwise: Boolean, val aboveHomeMetres: Double)

internal data class OrbitDefaults(val radius: Double, val unit: String, val metresPerUnit: Double, val clockwise: Boolean)

internal fun orbitDefaults(view: JSONObject?): OrbitDefaults = OrbitDefaults(
    radius = view?.optDouble("orbitDefaultRadius")?.takeIf { !it.isNaN() } ?: 0.0,
    unit = view?.optText("orbitRadiusUnit").orEmpty(),
    metresPerUnit = view?.optDouble("orbitMetresPerUnit")?.takeIf { !it.isNaN() && it > 0 } ?: 1.0,
    clockwise = view?.optBoolean("orbitClockwise", true) ?: true,
)

internal fun radiusMetres(entered: String, defaults: OrbitDefaults): Double? =
    one.aircast.map.typedNumber(entered.ifBlank { defaults.radius.toString() })?.let { it * defaults.metresPerUnit }

internal fun orbitOpened(point: MapPoint, defaults: OrbitDefaults): OrbitCircle =
    OrbitCircle(TrackPoint(point.latitude, point.longitude), (radiusMetres("", defaults) ?: 0.0).coerceAtLeast(MINIMUM_CIRCLE_RADIUS_METRES), defaults.clockwise)

internal fun orbitEdit(circle: OrbitCircle, defaults: OrbitDefaults): LoiterEdit =
    LoiterEdit(circle.radiusMetres, circle.clockwise, defaults.unit, defaults.metresPerUnit)

internal fun orbitArgs(point: MapPoint, choice: OrbitChoice): Array<Any> =
    arrayOf(point.latitude, point.longitude, choice.radiusMetres, choice.clockwise, choice.aboveHomeMetres)

internal fun coordinateLines(point: MapPoint): List<String> = listOf(
    String.format(Locale.US, "Lat: %.6f", point.latitude),
    String.format(Locale.US, "Lon: %.6f", point.longitude),
)

private fun send(action: MapClickAction, point: MapPoint, orbit: OrbitChoice?): String? =
    when (orbit) {
        null -> Qgc.refusalOf(action.path, JSONObject().put("latitude", point.latitude).put("longitude", point.longitude))
        else -> Qgc.refusalOf(action.path, *orbitArgs(point, orbit))
    }

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun MapClickMenu(point: MapPoint, onDismiss: () -> Unit) {
    val view by qgcPath(MAP_CLICK_PATH)
    val vehicleCoordinate by qgcPath("vehicle.coordinate")
    val actions = remember(view) { mapClickActions(view) }
    if (view != null && actions.isEmpty()) {
        LaunchedEffect(point) { onDismiss() }
        return
    }
    var confirming by remember(point) { mutableStateOf<MapClickAction?>(null) }
    LaunchedEffect(actions) {
        val gone = confirming?.takeIf { pending -> actions.none { it.id == pending.id } }
        if (gone?.id == ORBIT_ACTION) onDismiss() else if (gone != null) confirming = null
    }
    var refusal by remember(point) { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    val defaults = remember(view) { orbitDefaults(view) }
    val mapEdits = one.aircast.map.LocalFlyMapEdits.current
    val previewing = confirming?.id == GOTO_ACTION
    DisposableEffect(previewing, point) {
        mapEdits.gotoPreview = if (previewing) TrackPoint(point.latitude, point.longitude) else null
        onDispose { mapEdits.gotoPreview = null }
    }
    val previewJson by qgcPath(if (previewing) gotoPreviewPath(point) else MAP_CLICK_PATH)
    val preview = remember(previewJson, previewing) { gotoPreview(previewJson).takeIf { previewing } }

    fun run(action: MapClickAction) {
        scope.launch {
            val refused = withContext(Dispatchers.Default) { send(action, point, null) }
            if (refused == null) onDismiss() else refusal = refused
        }
    }

    confirming?.takeIf { it.id == ORBIT_ACTION }?.let { orbit ->
        OrbitPanel(point, orbit, defaults, onDone = onDismiss)
        return
    }

    AircastSheet(onDismissRequest = onDismiss) {
        Column(Modifier.fillMaxWidth().padding(bottom = 24.dp)) {
            val pending = confirming
            if (pending == null) {
                actions.forEach { action ->
                    ListItem(
                        headlineContent = { Text(sentenceCase(action.label)) },
                        modifier = Modifier.clickable {
                            refusal = null
                            if (action.confirm) confirming = action else run(action)
                        },
                    )
                }
            } else {
                Column(
                    Modifier.padding(horizontal = 20.dp),
                    verticalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    Text(sentenceCase(pending.title), style = MaterialTheme.typography.titleLarge)
                    val vehicleAt = geoOf(vehicleCoordinate)?.let { (lat, lon) -> MapPoint(lat, lon) }
                    val away = vehicleAt?.takeIf { pending.id == GOTO_ACTION }?.let { goHereText(it, point, defaults.unit, defaults.metresPerUnit) }
                    val lines = if (preview != null) gotoPreviewLines(preview.copy(distance = away ?: preview.distance), pending.message) else listOf(away ?: pending.message)
                    val warn = preview?.pastReturnPoint == true
                    lines.forEachIndexed { index, line ->
                        Text(
                            line,
                            style = MaterialTheme.typography.bodyMedium,
                            color = if (warn && index == lines.lastIndex) MaterialTheme.aircast.warning else MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    if (warn) {
                        androidx.compose.material3.Button(
                            onClick = {
                                scope.launch {
                                    withContext(Dispatchers.Default) { one.aircast.android.bridge.VehicleCommands.returnToLaunch(false) }
                                    onDismiss()
                                }
                            },
                            modifier = Modifier.fillMaxWidth(),
                        ) { Text(RETURN_HOME_INSTEAD) }
                    }
                    HoldOrCancel(pending.title, onConfirm = { run(pending) }, onCancel = { confirming = null })
                }
            }
            refusal?.let {
                Text(
                    it,
                    color = MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.bodyMedium,
                    modifier = Modifier.padding(horizontal = 20.dp, vertical = 8.dp),
                )
            }
            HorizontalDivider(Modifier.padding(vertical = 8.dp))
            coordinateLines(point).forEach {
                Text(
                    it,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.align(Alignment.CenterHorizontally),
                )
            }
        }
    }
}

@Composable
private fun OrbitPanel(point: MapPoint, action: MapClickAction, defaults: OrbitDefaults, onDone: () -> Unit) {
    val mapEdits = one.aircast.map.LocalFlyMapEdits.current
    val opened = remember(point) { orbitOpened(point, defaults) }
    DisposableEffect(opened) {
        mapEdits.orbit = opened
        onDispose { mapEdits.orbit = null }
    }
    val circle = mapEdits.orbit ?: opened
    val edit = orbitEdit(circle, defaults)
    var typed by remember(opened) { mutableStateOf<String?>(null) }
    var height by remember(opened) { mutableStateOf<GuidedAltitude?>(null) }
    var target by remember(opened) { mutableStateOf<Double?>(null) }
    var settled by remember(opened) { mutableStateOf<Double?>(null) }
    var refusal by remember(opened) { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    LaunchedEffect(settled) {
        height = withContext(Dispatchers.Default) {
            guidedAltitude(Qgc.get(settled?.let { guidedAltitudePath(it) } ?: GUIDED_ALTITUDE))
        }
        if (target == null) target = height?.current
    }

    Box(Modifier.fillMaxSize().navigationBarsPadding().padding(12.dp), contentAlignment = Alignment.BottomCenter) {
        GuidedValuePanel(
            title = sentenceCase(action.title),
            sentence = action.message,
            commitLabel = action.title,
            commitEnabled = height != null,
            onCommit = {
                val above = height?.targetMeters ?: height?.currentMeters
                if (above == null) {
                    refusal = "Choose a radius and a height for the orbit."
                } else {
                    val centre = MapPoint(circle.centre.latitude, circle.centre.longitude)
                    scope.launch {
                        val refused = withContext(Dispatchers.Default) { send(action, centre, OrbitChoice(circle.radiusMetres, circle.clockwise, above)) }
                        if (refused == null) onDone() else refusal = refused
                    }
                }
            },
            onCancel = onDone,
            modifier = Modifier.widthIn(max = 560.dp),
        ) {
            OutlinedTextField(
                value = loiterRadiusField(typed, edit),
                onValueChange = { text ->
                    typed = text
                    mapEdits.orbit = circle.copy(radiusMetres = loiterTyped(text, edit).radiusMetres)
                },
                label = { Text(listOf("Radius", defaults.unit).filter { it.isNotBlank() }.joinToString(" ")) },
                singleLine = true,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                modifier = Modifier.fillMaxWidth(),
            )
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text("Clockwise", Modifier.weight(1f))
                Switch(checked = circle.clockwise, onCheckedChange = { mapEdits.orbit = circle.copy(clockwise = it) })
            }
            height?.let { reading ->
                Text(reading.label, style = MaterialTheme.typography.bodySmall)
                Slider(
                    value = (target ?: reading.current ?: 0.0).toFloat(),
                    onValueChange = { target = it.toDouble() },
                    onValueChangeFinished = { settled = target },
                    valueRange = (reading.minimum ?: 0.0).toFloat()..(reading.maximum ?: 0.0).toFloat(),
                )
                rangeLabel(reading.minimum, reading.maximum, reading.unit)?.let { RangeHint(it) }
            }
            refusal?.let { Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium) }
        }
    }
}

internal const val SET_WAYPOINT_PATH = "vehicle.setCurrentMissionSequence"

internal fun waypointTarget(sequence: Int): Int = maxOf(sequence, 1)

internal fun setWaypointMessage(sequence: Int): String = "Adjust current waypoint to ${waypointTarget(sequence)}"

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun SetWaypointSheet(sequence: Int, onDismiss: () -> Unit) {
    var refusal by remember(sequence) { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    AircastSheet(onDismissRequest = onDismiss) {
        Column(
            Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(bottom = 24.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text("Set waypoint", style = MaterialTheme.typography.titleLarge)
            Text(setWaypointMessage(sequence), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            HoldOrCancel("Set waypoint", onConfirm = {
                scope.launch {
                    val refused = withContext(Dispatchers.Default) { Qgc.refusalOf(SET_WAYPOINT_PATH, waypointTarget(sequence)) }
                    if (refused == null) onDismiss() else refusal = refused
                }
            }, onCancel = onDismiss)
            refusal?.let { Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium) }
        }
    }
}

internal data class LoiterOffer(
    val latitude: Double,
    val longitude: Double,
    val title: String,
    val message: String,
    val defaultRadius: Double,
    val clockwise: Boolean,
)

internal fun loiterOffer(view: JSONObject?): LoiterOffer? =
    view?.optJSONObject("loiter")?.let {
        LoiterOffer(
            latitude = it.optDouble("latitude"),
            longitude = it.optDouble("longitude"),
            title = it.optText("title"),
            message = it.optText("message"),
            defaultRadius = it.optDouble("defaultRadius", 0.0),
            clockwise = it.optBoolean("clockwise", true),
        )
    }

internal fun mapClickUnits(view: JSONObject?): OrbitDefaults = orbitDefaults(view)

internal fun signedLoiterRadius(metres: Double, clockwise: Boolean): Double =
    if (clockwise) kotlin.math.abs(metres) else -kotlin.math.abs(metres)

internal fun loiterEditOpened(offer: LoiterOffer, units: OrbitDefaults): LoiterEdit =
    LoiterEdit(kotlin.math.abs(offer.defaultRadius) * units.metresPerUnit, offer.clockwise, units.unit, units.metresPerUnit)

internal fun loiterRadiusField(typed: String?, edit: LoiterEdit): String =
    typed?.takeIf { it.isBlank() || one.aircast.map.typedNumber(it)?.times(edit.metresPerUnit) == edit.radiusMetres } ?: loiterEditNumber(edit)

internal fun loiterTyped(text: String, edit: LoiterEdit): LoiterEdit =
    one.aircast.map.typedNumber(text)?.takeIf { it > 0.0 }?.let { edit.copy(radiusMetres = it * edit.metresPerUnit) } ?: edit

@Composable
internal fun LoiterRadiusPanel(offer: LoiterOffer, units: OrbitDefaults, onRefused: (String) -> Unit, onDone: () -> Unit) {
    val mapEdits = one.aircast.map.LocalFlyMapEdits.current
    val opened = remember(offer) { loiterEditOpened(offer, units) }
    DisposableEffect(opened) {
        mapEdits.gotoLoiter = opened
        onDispose { mapEdits.gotoLoiter = null }
    }
    val edit = mapEdits.gotoLoiter ?: opened
    var typed by remember(opened) { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    GuidedValuePanel(
        title = offer.title,
        sentence = offer.message,
        commitLabel = offer.title,
        commitEnabled = true,
        onCommit = {
            val sent = signedLoiterRadius(edit.radiusMetres, edit.clockwise)
            scope.launch {
                val refused = withContext(Dispatchers.Default) {
                    Qgc.refusalOf(
                        "vehicle.guidedModeGotoLocation",
                        JSONObject().put("latitude", offer.latitude).put("longitude", offer.longitude),
                        sent,
                    )
                }
                refused?.let(onRefused)
                onDone()
            }
        },
        onCancel = onDone,
    ) {
        OutlinedTextField(
            value = loiterRadiusField(typed, edit),
            onValueChange = { text ->
                typed = text
                mapEdits.gotoLoiter = loiterTyped(text, edit)
            },
            label = { Text(listOf("Radius", edit.unit).filter { it.isNotBlank() }.joinToString(" ")) },
            singleLine = true,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
            modifier = Modifier.fillMaxWidth(),
        )
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("Clockwise", Modifier.weight(1f))
            Switch(checked = edit.clockwise, onCheckedChange = { mapEdits.gotoLoiter = edit.copy(clockwise = it) })
        }
    }
}

internal const val STOP_ROI_PATH = "vehicle.stopGuidedModeROI"
internal const val SET_ROI_PATH = "vehicle.guidedModeROI"

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun RoiSheet(at: TrackPoint, onDismiss: () -> Unit) {
    var editing by remember(at) { mutableStateOf(false) }
    var refusal by remember(at) { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    val run: (String, Array<Any>) -> Unit = { path, args ->
        scope.launch {
            val refused = withContext(Dispatchers.Default) { Qgc.refusalOf(path, *args) }
            if (refused == null) onDismiss() else refusal = refused
        }
    }
    if (editing) {
        EditPositionDialog(at, onDismiss = { editing = false }, title = "Edit ROI Position") { latitude, longitude ->
            editing = false
            run(SET_ROI_PATH, arrayOf(JSONObject().put("latitude", latitude).put("longitude", longitude)))
        }
    }
    AircastSheet(onDismissRequest = onDismiss) {
        Column(
            Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(bottom = 24.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text("ROI", style = MaterialTheme.typography.titleMedium)
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedButton(onClick = { run(STOP_ROI_PATH, emptyArray()) }) { Text("Cancel ROI") }
                OutlinedButton(onClick = { editing = true }) { Text("Edit position") }
            }
            refusal?.let { Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium) }
        }
    }
}
