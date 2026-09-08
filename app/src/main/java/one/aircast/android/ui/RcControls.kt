package one.aircast.android.ui

import org.json.JSONArray

internal const val PWM_MIN = 1000
internal const val PWM_CENTER = 1500
internal const val PWM_MAX = 2000

enum class RcControlType { Slider, Button, Switch3, Momentary }

data class RcControl(
    val label: String,
    val channel: Int,
    val type: RcControlType,
)

private fun typeOf(raw: String): RcControlType? = when (raw) {
    "slider" -> RcControlType.Slider
    "button" -> RcControlType.Button
    "switch3" -> RcControlType.Switch3
    "momentary" -> RcControlType.Momentary
    else -> null
}

internal fun parseRcControls(json: String?): List<RcControl> {
    val array = runCatching { JSONArray(json.orEmpty()) }.getOrNull() ?: return emptyList()
    return (0 until array.length())
        .mapNotNull { array.optJSONObject(it) }
        .mapNotNull { entry ->
            val channel = entry.optInt("channel", 0)
            val type = typeOf(entry.optString("type"))
            if (channel <= 0 || type == null) {
                null
            } else {
                RcControl(
                    label = entry.optString("label").ifBlank { "CH$channel" },
                    channel = channel,
                    type = type,
                )
            }
        }
}

internal fun switch3Pwms(): List<Int> = listOf(PWM_MIN, PWM_CENTER, PWM_MAX)
