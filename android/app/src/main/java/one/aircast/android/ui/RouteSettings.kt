package one.aircast.android.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.map.SettingStepper
import org.json.JSONObject

private const val ALTITUDE_STEP = 1.0
private const val SPEED_STEP = 0.5
private const val FEET = "ft"
private val METRES_RANGE = 0.0..500.0
private val FEET_RANGE = 0.0..1640.0
private const val ROUTE_ALTITUDE_NOTE = "New waypoints use this"
private const val VEHICLE_SPEED_NOTE = "Vehicle default"

internal data class RouteAltitude(val value: Double, val units: String, val path: String)

internal fun routeAltitude(plan: JSONObject?): RouteAltitude? =
    plan?.optJSONObject("defaults")?.optJSONObject("altitude")?.let(::factFromControl)?.let { fact ->
        ((fact.value as? Number)?.toDouble() ?: fact.valueString.toDoubleOrNull())?.let { RouteAltitude(it, fact.units, fact.path) }
    }

internal fun routeSpeedRange(speed: SpeedSection): ClosedFloatingPointRange<Double> =
    speed.slider?.let { it.from.toDouble()..it.to.toDouble() } ?: 0.0..30.0

@Composable
internal fun RouteSettings(plan: JSONObject?) {
    val scope = rememberCoroutineScope()
    var refusal by remember { mutableStateOf<String?>(null) }
    val write: (() -> String?) -> Unit = { work -> scope.launch { refusal = withContext(Dispatchers.Default) { work() } } }
    val altitude = remember(plan) { routeAltitude(plan) }
    val speed = remember(plan) { speedSectionOf(plan?.optJSONObject("defaults")?.optJSONObject("flightSpeed")) }
    Column(Modifier.fillMaxWidth()) {
        Text("Route", style = MaterialTheme.typography.titleSmall, modifier = Modifier.padding(start = 12.dp, top = 4.dp))
        Column(Modifier.fillMaxWidth().padding(horizontal = 12.dp)) {
            altitude?.let { route ->
                SettingStepper(
                    label = "Altitude",
                    value = route.value,
                    unit = route.units,
                    step = ALTITUDE_STEP,
                    note = ROUTE_ALTITUDE_NOTE,
                    range = if (route.units == FEET) FEET_RANGE else METRES_RANGE,
                    slider = true,
                    onSet = { wanted -> write { Qgc.writeRefusal(route.path, wanted) } },
                )
            }
            speed?.let { section ->
                SettingStepper(
                    label = "Speed",
                    value = section.value,
                    unit = section.units,
                    step = SPEED_STEP,
                    note = VEHICLE_SPEED_NOTE.takeIf { !section.specified },
                    range = routeSpeedRange(section),
                    onSet = { wanted -> write { Qgc.writeRefusal(section.specifyPath, true) ?: Qgc.writeRefusal(section.path, wanted) } },
                    trailing = if (section.specified) {
                        { FilterChip(selected = false, onClick = { write { Qgc.writeRefusal(section.specifyPath, false) } }, label = { Text("Auto") }) }
                    } else null,
                )
            }
        }
        MissionAltitudeFrame()
        refusal?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error, modifier = Modifier.padding(horizontal = 12.dp)) }
    }
}
