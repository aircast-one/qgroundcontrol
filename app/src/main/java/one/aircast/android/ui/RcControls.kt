package one.aircast.android.ui

import org.json.JSONArray
import org.json.JSONObject

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

internal const val RC_CHANNEL_MIN = 1
internal const val RC_CHANNEL_MAX = 18

internal fun typeName(type: RcControlType): String = when (type) {
    RcControlType.Slider -> "slider"
    RcControlType.Button -> "button"
    RcControlType.Switch3 -> "switch3"
    RcControlType.Momentary -> "momentary"
}

internal fun typeLabel(type: RcControlType): String = when (type) {
    RcControlType.Slider -> "Slider"
    RcControlType.Button -> "Button"
    RcControlType.Switch3 -> "Three-way switch"
    RcControlType.Momentary -> "Momentary"
}

private fun entries(json: String?): List<JSONObject> {
    val array = runCatching { JSONArray(json.orEmpty()) }.getOrNull() ?: return emptyList()
    return (0 until array.length()).map { array.optJSONObject(it) ?: JSONObject() }
}

private fun encode(entries: List<JSONObject>): String =
    JSONArray().also { array -> entries.forEach { array.put(it) } }.toString()

private fun patched(entry: JSONObject, label: String, channel: Int, type: RcControlType): JSONObject {
    val next = JSONObject(entry.toString())
    next.put("label", label)
    next.put("channel", channel)
    next.put("type", typeName(type))
    return next
}

internal fun rcControlsAdded(json: String?, label: String, channel: Int, type: RcControlType): String =
    encode(entries(json) + patched(JSONObject(), label, channel, type))

internal fun rcControlsRemoved(json: String?, index: Int): String =
    encode(entries(json).filterIndexed { at, _ -> at != index })

internal fun rcControlsPatched(
    json: String?,
    index: Int,
    label: String,
    channel: Int,
    type: RcControlType,
): String = encode(
    entries(json).mapIndexed { at, entry ->
        if (at == index) patched(entry, label, channel, type) else entry
    },
)

internal fun channelOwner(json: String?, channel: Int, ignoring: Int, reserved: Map<Int, String>): String? {
    reserved[channel]?.let { return it }
    return entries(json)
        .mapIndexed { at, entry -> at to entry }
        .firstOrNull { (at, entry) -> at != ignoring && entry.optInt("channel", 0) == channel }
        ?.let { (at, entry) -> entry.optString("label").ifBlank { "control ${at + 1}" } }
}

internal fun firstFreeChannel(json: String?, reserved: Map<Int, String>): Int =
    (RC_CHANNEL_MIN..RC_CHANNEL_MAX).firstOrNull { channelOwner(json, it, -1, reserved) == null }
        ?: RC_CHANNEL_MIN

internal fun channelUsable(channel: Int): Boolean = channel in RC_CHANNEL_MIN..RC_CHANNEL_MAX

internal const val RC_SEND_INTERVAL_MS = 100L

internal fun rcSendDue(nowMs: Long, lastSentMs: Long, finished: Boolean): Boolean =
    finished || nowMs - lastSentMs >= RC_SEND_INTERVAL_MS
