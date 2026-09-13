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
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.withContext
import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

object MapBridge {
    private const val CLIENT = "map"
    private val watched = linkedSetOf<String>()
    private val _values = MutableStateFlow<Map<String, JSONObject>>(emptyMap())

    val values: StateFlow<Map<String, JSONObject>> = _values.asStateFlow()

    private val _bridgeReady = MutableStateFlow(false)
    val bridgeReady: StateFlow<Boolean> = _bridgeReady.asStateFlow()

    fun start() {
        runCatching {
            QGCBridge.setEventListener(CLIENT) { path, json ->
                _values.value = _values.value +
                    (path to runCatching { JSONObject(json) }.getOrDefault(JSONObject()))
            }
            _bridgeReady.value = true
        }
    }

    @Synchronized
    fun release() {
        watched.clear()
        _values.value = emptyMap()
        runCatching { QGCBridge.watch(CLIENT, "") }
    }

    @Synchronized
    fun watch(path: String) {
        if (!watched.add(path)) {
            return
        }
        runCatching { QGCBridge.watch(CLIENT, watched.joinToString(",")) }
            .onSuccess { _bridgeReady.value = true }
            .onFailure {
                watched.remove(path)
                _bridgeReady.value = false
            }
    }

    fun seed(path: String) {
        if (_values.value.containsKey(path)) {
            return
        }
        val json = runCatching { JSONObject(QGCBridge.get(path)) }.getOrNull() ?: return
        _values.update { held -> if (held.containsKey(path)) held else held + (path to json) }
    }

    fun markReachable() {
        _bridgeReady.value = true
    }
}

@Composable
fun mapPath(path: String): State<JSONObject?> {
    LaunchedEffect(path) {
        MapBridge.watch(path)
        withContext(Dispatchers.Default) { MapBridge.seed(path) }
    }
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

@Composable
fun mapCount(path: String): State<Int> {
    val json by mapPath(path)
    return remember(path) { derivedStateOf { json?.optJSONArray("elements")?.length() ?: 0 } }
}

fun coordinateOf(json: JSONObject?): TrackPoint? {
    val coordinate = json?.takeIf { it.optBoolean("valid") } ?: return null
    val latitude = coordinate.optDouble("latitude", Double.NaN)
    val longitude = coordinate.optDouble("longitude", Double.NaN)
    return if (isPlottable(latitude, longitude)) TrackPoint(latitude, longitude) else null
}

const val FLY_STATE_VIEW = "view.flyState"

@Composable
fun mapCoordinate(path: String): State<TrackPoint?> {
    val json by mapPath(path)
    return remember(path) { derivedStateOf { coordinateOf(json) } }
}

@Composable
fun mapViewFlag(path: String, field: String): State<Boolean> {
    val json by mapPath(path)
    return remember(path, field) { derivedStateOf { json?.optBoolean(field) == true } }
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
