package one.aircast.android.bridge

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import org.json.JSONObject

@Composable
fun qgcPath(path: String): State<JSONObject?> {
    DisposableEffect(path) {
        Qgc.watch(listOf(path))
        onDispose { Qgc.unwatch(listOf(path)) }
    }
    val values by Qgc.values.collectAsState()
    return remember(path) { derivedStateOf { values[path] } }
}

@Composable
fun qgcValue(path: String): State<Any?> {
    val json by qgcPath(path)
    return remember(path) { derivedStateOf { json?.opt("value") } }
}

@Composable
fun qgcString(path: String, fallback: String = ""): State<String> {
    val value by qgcValue(path)
    return remember(path, fallback) { derivedStateOf { value?.toString() ?: fallback } }
}

@Composable
fun qgcBool(path: String): State<Boolean> {
    val value by qgcValue(path)
    return remember(path) { derivedStateOf { truthy(value) } }
}

internal fun truthy(value: Any?): Boolean = when (value) {
    is Boolean -> value
    is Number -> value.toDouble() != 0.0
    is String -> value.equals("true", ignoreCase = true) || value.toDoubleOrNull()?.let { it != 0.0 } == true
    else -> false
}

@Composable
fun qgcDouble(path: String, fallback: Double = Double.NaN): State<Double> {
    val value by qgcValue(path)
    return remember(path, fallback) {
        derivedStateOf {
            when (val v = value) {
                is Number -> v.toDouble()
                is Boolean -> if (v) 1.0 else 0.0
                is String -> v.toDoubleOrNull() ?: fallback
                else -> fallback
            }
        }
    }
}

@Composable
fun qgcFacts(groupPath: String): State<List<Fact>> {
    val json by qgcPath(groupPath)
    return remember(groupPath) { derivedStateOf { Qgc.facts(groupPath, json) } }
}

@Composable
fun qgcStrings(path: String): State<List<String>> {
    val value by qgcValue(path)
    return remember(path) {
        derivedStateOf {
            when (value) {
                is org.json.JSONArray -> (0 until (value as org.json.JSONArray).length())
                    .map { (value as org.json.JSONArray).optString(it) }
                else -> emptyList()
            }
        }
    }
}
