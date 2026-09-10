package one.aircast.android.bridge

import android.os.Looper
import android.os.SystemClock
import android.util.Log
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import org.json.JSONArray
import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

data class Fact(
    val path: String,
    val name: String,
    val description: String,
    val units: String,
    val valueString: String,
    val value: Any?,
    val enumStrings: List<String>,
    val enumValues: List<String> = emptyList(),
    val enumIndex: Int,
    val bitmaskStrings: List<String> = emptyList(),
    val bitmaskValues: List<Long> = emptyList(),
    val isBool: Boolean,
    val isString: Boolean,
    val readOnly: Boolean,
    val minString: String = "",
    val maxString: String = "",
    val minIsDefaultForType: Boolean = true,
    val maxIsDefaultForType: Boolean = true,
    val defaultValueString: String = "",
    val vehicleRebootRequired: Boolean = false,
    val qgcRebootRequired: Boolean = false,
) {
    val title: String = description.ifBlank { name }
    val isEnum: Boolean = enumStrings.isNotEmpty() && bitmaskStrings.isEmpty()
    val isBitmask: Boolean = bitmaskStrings.isNotEmpty() && bitmaskStrings.size == bitmaskValues.size
    val valueIsOffTheEnumList: Boolean =
        enumStrings.getOrNull(enumIndex)?.startsWith("Unknown: ") == true

    val boolValue: Boolean = value == true || valueString.equals("true", ignoreCase = true) || valueString == "1"
}

object Qgc {
    private const val TAG = "QgcBridge"

    private var watched: Map<String, Int> = emptyMap()
    private val _values = MutableStateFlow<Map<String, JSONObject>>(emptyMap())

    val values: StateFlow<Map<String, JSONObject>> = _values.asStateFlow()

    fun start() {
        QGCBridge.setEventListener(CLIENT) { path, json ->
            _values.value = _values.value + (path to runCatching { JSONObject(json) }.getOrDefault(JSONObject()))
        }
    }

    private const val SLOW_CALL_MS = 250L

    private fun onMainThread(): Boolean = Looper.myLooper() == Looper.getMainLooper()

    private fun <T> timed(what: String, block: () -> T): T {
        val started = SystemClock.uptimeMillis()
        val result = block()
        val took = SystemClock.uptimeMillis() - started
        if (took >= SLOW_CALL_MS) {
            Log.w(TAG, "bridge call blocked ${took}ms in $what, onMainThread=${onMainThread()}")
        }
        return result
    }

    private const val CLIENT = "app"

    internal var sendWatch: (String) -> Unit = { QGCBridge.watch(CLIENT, it) }

    internal fun forgetWatchesForTest() {
        watched = emptyMap()
    }

    internal fun watchedPathsForTest(): Set<String> = watched.keys

    @Synchronized
    fun watch(paths: Collection<String>) {
        val next = retained(watched, paths)
        if (next.keys == watched.keys) {
            watched = next
            return
        }
        val previous = watched
        watched = next
        if (!resend()) {
            watched = previous
            Log.w(TAG, "watch failed for $paths, will retry")
        }
    }

    @Synchronized
    fun unwatch(paths: Collection<String>) {
        val next = released(watched, paths)
        val dropped = watched.keys - next.keys
        if (dropped.isEmpty()) {
            watched = next
            return
        }
        val previous = watched
        watched = next
        if (resend()) {
            _values.value = _values.value - dropped
        } else {
            watched = previous
            Log.w(TAG, "unwatch failed for $paths, still watching them")
        }
    }

    private fun retained(counts: Map<String, Int>, paths: Collection<String>): Map<String, Int> =
        paths.fold(counts) { acc, path -> acc + (path to (acc[path] ?: 0) + 1) }

    private fun released(counts: Map<String, Int>, paths: Collection<String>): Map<String, Int> =
        paths.fold(counts) { acc, path ->
            when (val held = acc[path]) {
                null -> acc
                1 -> acc - path
                else -> acc + (path to held - 1)
            }
        }

    private fun resend(): Boolean =
        runCatching { timed("watch") { sendWatch(watched.keys.joinToString(",")) } }.isSuccess

    fun get(path: String): JSONObject =
        timed("get $path") { runCatching { JSONObject(QGCBridge.get(path)) }.getOrDefault(JSONObject()) }

    fun get(path: String, fields: Collection<String>): JSONObject =
        timed("get $path") {
            runCatching { JSONObject(QGCBridge.getFields(path, fields.joinToString(","))) }
                .getOrDefault(JSONObject())
        }

    fun set(path: String, value: Any?): Boolean {
        val reply = runCatching {
            JSONObject(timed("set $path") { QGCBridge.set(path, JSONObject().put("value", value).toString()) })
        }.getOrNull()
        if (reply?.optBoolean("ok") == true) return true
        val reason = reply?.optString("reason").orEmpty().ifBlank { "the bridge rejected the write" }
        Log.w(TAG, "set $path failed: $reason")
        return false
    }

    fun invoke(path: String, vararg args: Any?): Boolean {
        val ok = call(path, *args)?.optBoolean("ok") ?: false
        if (!ok) Log.w(TAG, "invoke $path failed")
        return ok
    }

    fun invokeResult(path: String, vararg args: Any?): Any? = call(path, *args)?.opt("result")

    private fun call(path: String, vararg args: Any?): JSONObject? {
        val array = JSONArray().apply { args.forEach { put(it) } }
        return timed("invoke $path") {
            runCatching { JSONObject(QGCBridge.invoke(path, array.toString())) }.getOrNull()
        }
    }

    fun facts(groupPath: String, json: JSONObject?): List<Fact> {
        val array = json?.optJSONArray("facts") ?: return emptyList()
        return (0 until array.length()).mapNotNull { index ->
            array.optJSONObject(index)?.let { fact(groupPath, it) }
        }
    }

    fun factAt(path: String, json: JSONObject): Fact = fact("", json).copy(path = path)

    private fun JSONObject.text(key: String): String = if (isNull(key)) "" else optString(key)

    fun fact(groupPath: String, json: JSONObject): Fact {
        val name = json.text("name")
        val enums = json.optJSONArray("enumStrings")
        return Fact(
            path = if (groupPath.isEmpty()) name else "$groupPath.$name",
            name = name,
            description = json.text("shortDescription"),
            units = json.text("units"),
            valueString = json.text("valueString"),
            value = json.opt("value"),
            enumStrings = (0 until (enums?.length() ?: 0)).map { enums!!.optString(it) },
            enumValues = json.optJSONArray("enumValues")?.let { raw ->
                (0 until raw.length()).map { raw.optString(it) }
            } ?: emptyList(),
            enumIndex = json.optInt("enumIndex", -1),
            bitmaskStrings = json.optJSONArray("bitmaskStrings")?.let { bits ->
                (0 until bits.length()).map { bits.optString(it) }
            } ?: emptyList(),
            bitmaskValues = json.optJSONArray("bitmaskValues")?.let { bits ->
                (0 until bits.length()).map { bits.optLong(it) }
            } ?: emptyList(),
            isBool = json.optBoolean("typeIsBool"),
            isString = json.optBoolean("typeIsString"),
            readOnly = json.optBoolean("readOnly"),
            minString = json.text("minString"),
            maxString = json.text("maxString"),
            minIsDefaultForType = json.optBoolean("minIsDefaultForType", true),
            maxIsDefaultForType = json.optBoolean("maxIsDefaultForType", true),
            defaultValueString = json.text("defaultValueString"),
            vehicleRebootRequired = json.optBoolean("vehicleRebootRequired"),
            qgcRebootRequired = json.optBoolean("qgcRebootRequired"),
        )
    }
}
