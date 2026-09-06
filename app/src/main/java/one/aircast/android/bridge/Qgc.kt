package one.aircast.android.bridge

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
    val enumIndex: Int,
    val isBool: Boolean,
    val isString: Boolean,
    val readOnly: Boolean,
) {
    val title: String = description.ifBlank { name }
    val isEnum: Boolean = enumStrings.isNotEmpty()
    val boolValue: Boolean = value == true || valueString.equals("true", ignoreCase = true) || valueString == "1"
}

object Qgc {
    private val watched = linkedSetOf<String>()
    private val _values = MutableStateFlow<Map<String, JSONObject>>(emptyMap())

    val values: StateFlow<Map<String, JSONObject>> = _values.asStateFlow()

    fun start() {
        QGCBridge.setEventListener { path, json ->
            _values.value = _values.value + (path to runCatching { JSONObject(json) }.getOrDefault(JSONObject()))
        }
    }

    @Synchronized
    fun watch(paths: Collection<String>) {
        if (!watched.addAll(paths)) return
        QGCBridge.watch(watched.joinToString(","))
    }

    fun get(path: String): JSONObject = runCatching { JSONObject(QGCBridge.get(path)) }.getOrDefault(JSONObject())

    fun set(path: String, value: Any?) {
        QGCBridge.set(path, JSONObject().put("value", value).toString())
    }

    fun invoke(path: String, vararg args: Any?): Boolean = call(path, *args)?.optBoolean("ok") ?: false

    fun invokeResult(path: String, vararg args: Any?): Any? = call(path, *args)?.opt("result")

    private fun call(path: String, vararg args: Any?): JSONObject? {
        val array = JSONArray().apply { args.forEach { put(it) } }
        return runCatching { JSONObject(QGCBridge.invoke(path, array.toString())) }.getOrNull()
    }

    fun facts(groupPath: String, json: JSONObject?): List<Fact> {
        val array = json?.optJSONArray("facts") ?: return emptyList()
        return (0 until array.length()).mapNotNull { index ->
            array.optJSONObject(index)?.let { fact(groupPath, it) }
        }
    }

    fun fact(groupPath: String, json: JSONObject): Fact {
        val name = json.optString("name")
        val enums = json.optJSONArray("enumStrings")
        return Fact(
            path = if (groupPath.isEmpty()) name else "$groupPath.$name",
            name = name,
            description = json.optString("shortDescription"),
            units = json.optString("units"),
            valueString = json.optString("valueString"),
            value = json.opt("value"),
            enumStrings = (0 until (enums?.length() ?: 0)).map { enums!!.optString(it) },
            enumIndex = json.optInt("enumIndex", -1),
            isBool = json.optBoolean("typeIsBool"),
            isString = json.optBoolean("typeIsString"),
            readOnly = json.optBoolean("readOnly"),
        )
    }
}
