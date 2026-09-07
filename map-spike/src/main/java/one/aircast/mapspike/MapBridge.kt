package one.aircast.mapspike

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

object MapBridge {
    private val watched = linkedSetOf<String>()
    private val _values = MutableStateFlow<Map<String, JSONObject>>(emptyMap())

    val values: StateFlow<Map<String, JSONObject>> = _values.asStateFlow()

    private val _bridgeReady = MutableStateFlow(false)
    val bridgeReady: StateFlow<Boolean> = _bridgeReady.asStateFlow()

    // Reads, not watches. QGCBridgeCore::watch replaces the watcher's whole path
    // list and setEventListener is a single slot, so a second client of the
    // bridge cannot arm either without silently disarming the first. get() has
    // no such shared state, and the screen already polls, so the map takes its
    // values from the same pass. Costs latency, buys not breaking the app's
    // other screens.
    @Synchronized
    fun release() {
        watched.clear()
        _values.value = emptyMap()
    }

    @Synchronized
    fun watch(path: String) {
        watched.add(path)
    }

    @Synchronized
    private fun paths(): List<String> = watched.toList()

    // Driven by the screen's own poll so there is one cadence and one place that
    // decides the map is talking to a live bridge.
    fun refresh() {
        val reachable = paths().fold(false) { any, path ->
            val json = runCatching { JSONObject(QGCBridge.get(path)) }.getOrNull()
            if (json == null) {
                any
            } else {
                _values.value = _values.value + (path to json)
                true
            }
        }
        if (reachable) {
            markReachable()
        }
    }

    fun markReachable() {
        _bridgeReady.value = true
    }
}

@Composable
fun mapPath(path: String): State<JSONObject?> {
    LaunchedEffect(path) { MapBridge.watch(path) }
    val values by MapBridge.values.collectAsState()
    return remember(path) { derivedStateOf { values[path] } }
}

@Composable
fun mapDouble(path: String): State<Double> {
    val json by mapPath(path)
    return remember(path) {
        derivedStateOf {
            when (val value = json?.opt("value")) {
                is Number -> value.toDouble()
                is String -> value.toDoubleOrNull() ?: Double.NaN
                else -> Double.NaN
            }
        }
    }
}

@Composable
fun mapString(path: String): State<String> {
    val json by mapPath(path)
    return remember(path) { derivedStateOf { json?.opt("value")?.toString().orEmpty() } }
}

@Composable
fun mapInt(path: String, fallback: Int = -1): State<Int> {
    val json by mapPath(path)
    return remember(path, fallback) {
        derivedStateOf {
            when (val value = json?.opt("value")) {
                is Number -> value.toInt()
                is String -> value.toIntOrNull() ?: fallback
                else -> fallback
            }
        }
    }
}

// The number of elements a list model holds, without reading the elements.
@Composable
fun mapCount(path: String): State<Int> {
    val json by mapPath(path)
    return remember(path) { derivedStateOf { json?.optJSONArray("elements")?.length() ?: 0 } }
}

// A coordinate the vehicle has not established yet still arrives with
// latitude and longitude of zero, so the validity flag decides, not the numbers.
fun coordinateOf(json: JSONObject?): TrackPoint? {
    val coordinate = json?.takeIf { it.optBoolean("valid") } ?: return null
    val latitude = coordinate.optDouble("latitude", Double.NaN)
    val longitude = coordinate.optDouble("longitude", Double.NaN)
    return if (isPlottable(latitude, longitude)) TrackPoint(latitude, longitude) else null
}

@Composable
fun mapCoordinate(path: String): State<TrackPoint?> {
    val json by mapPath(path)
    return remember(path) { derivedStateOf { coordinateOf(json) } }
}

@Composable
fun mapBool(path: String): State<Boolean> {
    val json by mapPath(path)
    return remember(path) {
        derivedStateOf {
            val value = json?.opt("value")
            value == true || value == "true"
        }
    }
}
