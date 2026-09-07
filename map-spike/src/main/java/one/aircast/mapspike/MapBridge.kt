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

    fun start() {
        QGCBridge.setEventListener { path, json ->
            _values.value = _values.value + (path to runCatching { JSONObject(json) }.getOrDefault(JSONObject()))
        }
    }

    @Synchronized
    fun watch(path: String) {
        if (!watched.add(path)) return
        QGCBridge.watch(watched.joinToString(","))
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
fun mapBool(path: String): State<Boolean> {
    val json by mapPath(path)
    return remember(path) {
        derivedStateOf {
            val value = json?.opt("value")
            value == true || value == "true"
        }
    }
}
