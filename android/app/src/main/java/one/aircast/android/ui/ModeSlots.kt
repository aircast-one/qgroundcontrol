package one.aircast.android.ui

import org.json.JSONObject
import one.aircast.mapspike.optText

internal const val MODE_SLOTS = "view.modeSlots"

internal data class ModeSlot(
    val slot: Int,
    val mode: String,
    val live: Boolean,
)

internal data class ModeSlotsView(
    val channel: Int,
    val slots: List<ModeSlot>,
    val reason: String,
    val activeSwitches: List<String> = emptyList(),
    val optionChannelsOn: List<Int> = emptyList(),
)

internal fun modeSlotsView(view: JSONObject?): ModeSlotsView? {
    if (view == null || !view.optBoolean("available")) return null
    val items = view.optJSONArray("slots")
    val switches = view.optJSONArray("activeSwitches")
    val options = view.optJSONArray("channelOptions")
    return ModeSlotsView(
        activeSwitches = (0 until (switches?.length() ?: 0)).map { switches!!.optString(it) },
        optionChannelsOn = (0 until (options?.length() ?: 0)).mapNotNull { options?.optJSONObject(it) }.filter { it.optBoolean("enabled") }.map { it.optInt("channel") },
        channel = view.optInt("channel"),
        slots = (0 until (items?.length() ?: 0)).mapNotNull { index ->
            items?.optJSONObject(index)?.let {
                ModeSlot(
                    slot = it.optInt("slot"),
                    mode = it.optText("mode"),
                    live = it.optBoolean("live"),
                )
            }
        },
        reason = view.optText("reason"),
    )
}

internal fun liveSlotText(view: ModeSlotsView?): String? {
    val slots = view?.slots?.takeIf { it.isNotEmpty() } ?: return null
    val live = slots.firstOrNull { it.live }
        ?: return "The switch on channel ${view.channel} is not on any mode slot."
    return "The switch on channel ${view.channel} is on slot ${live.slot}, ${live.mode}."
}

internal fun liveSwitchesText(view: ModeSlotsView?): String? = when {
    view == null -> null
    view.activeSwitches.isNotEmpty() -> "Switches on: ${view.activeSwitches.joinToString(", ")}"
    view.optionChannelsOn.isNotEmpty() -> "Channel options on: ${view.optionChannelsOn.joinToString(", ") { "channel $it" }}"
    else -> null
}
