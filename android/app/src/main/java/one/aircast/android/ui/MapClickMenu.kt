package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Slider
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
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
import one.aircast.mapspike.optText
import one.aircast.mapspike.TrackPoint
import org.json.JSONObject
import java.util.Locale

internal const val MAP_CLICK_PATH = "view.mapClick"

internal data class MapPoint(val latitude: Double, val longitude: Double)

internal const val ORBIT_ACTION = "Orbit"
internal const val GOTO_ACTION = "GoTo"

private val COMPASS_POINTS = listOf("north", "north-east", "east", "south-east", "south", "south-west", "west", "north-west")

internal fun goHereText(from: MapPoint, to: MapPoint, unit: String, metresPerUnit: Double): String? {
    if (metresPerUnit <= 0.0 || unit.isBlank()) return null
    val metres = one.aircast.mapspike.metresBetween(one.aircast.mapspike.TrackPoint(from.latitude, from.longitude), one.aircast.mapspike.TrackPoint(to.latitude, to.longitude))
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
    entered.ifBlank { defaults.radius.toString() }.toDoubleOrNull()?.let { it * defaults.metresPerUnit }

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
    var refusal by remember(point) { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    val defaults = remember(view) { orbitDefaults(view) }
    var radiusText by remember(point) { mutableStateOf("") }
    var clockwise by remember(point) { mutableStateOf<Boolean?>(null) }
    var height by remember(point) { mutableStateOf<GuidedAltitude?>(null) }
    var target by remember(point) { mutableStateOf<Double?>(null) }
    var settled by remember(point) { mutableStateOf<Double?>(null) }

    LaunchedEffect(confirming, settled) {
        if (confirming?.id != ORBIT_ACTION) return@LaunchedEffect
        height = withContext(Dispatchers.Default) {
            guidedAltitude(Qgc.get(settled?.let { guidedAltitudePath(it) } ?: GUIDED_ALTITUDE))
        }
        if (target == null) target = height?.current
    }

    fun orbitChoice(): OrbitChoice? {
        val radius = radiusMetres(radiusText, defaults) ?: return null
        val above = height?.targetMeters ?: height?.currentMeters ?: return null
        return OrbitChoice(radius, clockwise ?: defaults.clockwise, above)
    }

    fun run(action: MapClickAction) {
        val orbit = if (action.id == ORBIT_ACTION) orbitChoice() else null
        if (action.id == ORBIT_ACTION && orbit == null) {
            refusal = "Choose a radius and a height for the orbit."
            return
        }
        scope.launch {
            val refused = withContext(Dispatchers.Default) { send(action, point, orbit) }
            if (refused == null) onDismiss() else refusal = refused
        }
    }

    ModalBottomSheet(onDismissRequest = onDismiss) {
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
                    Text(away ?: pending.message, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    if (pending.id == ORBIT_ACTION) {
                        OutlinedTextField(
                            value = radiusText,
                            onValueChange = { radiusText = it },
                            label = { Text(listOf("Radius", defaults.unit).filter { it.isNotBlank() }.joinToString(" ")) },
                            placeholder = { Text(defaults.radius.toString()) },
                            singleLine = true,
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                            modifier = Modifier.fillMaxWidth(),
                        )
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Text("Clockwise", Modifier.weight(1f))
                            Switch(checked = clockwise ?: defaults.clockwise, onCheckedChange = { clockwise = it })
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
                    }
                    SlideOrCancel(pending.title, onConfirm = { run(pending) }, onCancel = { confirming = null })
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

internal const val SET_WAYPOINT_PATH = "vehicle.setCurrentMissionSequence"

internal fun waypointTarget(sequence: Int): Int = maxOf(sequence, 1)

internal fun setWaypointMessage(sequence: Int): String = "Adjust current waypoint to ${waypointTarget(sequence)}"

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun SetWaypointSheet(sequence: Int, onDismiss: () -> Unit) {
    var refusal by remember(sequence) { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(
            Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(bottom = 24.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text("Set waypoint", style = MaterialTheme.typography.titleLarge)
            Text(setWaypointMessage(sequence), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            SlideOrCancel("Set Waypoint", onConfirm = {
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

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun LoiterRadiusSheet(offer: LoiterOffer, units: OrbitDefaults, onDismiss: () -> Unit) {
    var radiusText by remember(offer) { mutableStateOf("") }
    var clockwise by remember(offer) { mutableStateOf(offer.clockwise) }
    var refusal by remember(offer) { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    val defaults = units.copy(radius = offer.defaultRadius)
    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(
            Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(bottom = 24.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text(offer.title, style = MaterialTheme.typography.titleLarge)
            Text(offer.message, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            OutlinedTextField(
                value = radiusText,
                onValueChange = { radiusText = it },
                label = { Text(listOf("Radius", defaults.unit).filter { it.isNotBlank() }.joinToString(" ")) },
                placeholder = { Text(defaults.radius.toString()) },
                singleLine = true,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                modifier = Modifier.fillMaxWidth(),
            )
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text("Clockwise", Modifier.weight(1f))
                Switch(checked = clockwise, onCheckedChange = { clockwise = it })
            }
            SlideOrCancel(offer.title, onConfirm = {
                    val metres = radiusMetres(radiusText, defaults)
                    if (metres == null) {
                        refusal = "Enter a radius."
                    } else {
                        scope.launch {
                            val refused = withContext(Dispatchers.Default) {
                                Qgc.refusalOf(
                                    "vehicle.guidedModeGotoLocation",
                                    JSONObject().put("latitude", offer.latitude).put("longitude", offer.longitude),
                                    signedLoiterRadius(metres, clockwise),
                                )
                            }
                            if (refused == null) onDismiss() else refusal = refused
                        }
                    }
                }, onCancel = onDismiss)
            refusal?.let { Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium) }
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
        EditPositionDialog(at, onDismiss = { editing = false }, title = "Edit ROI Position", vehicleNote = "Move the ROI to the vehicle's current position.") { latitude, longitude ->
            editing = false
            run(SET_ROI_PATH, arrayOf(JSONObject().put("latitude", latitude).put("longitude", longitude)))
        }
    }
    ModalBottomSheet(onDismissRequest = onDismiss) {
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
