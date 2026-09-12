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
    val liveSlot: Int,
    val reason: String,
)

internal fun modeSlotsView(view: JSONObject?): ModeSlotsView? {
    if (view == null || !view.optBoolean("available")) return null
    val items = view.optJSONArray("slots")
    return ModeSlotsView(
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
        liveSlot = view.optInt("liveSlot"),
        reason = view.optText("reason"),
    )
}

internal fun liveSlotText(view: ModeSlotsView?): String? {
    val slots = view?.slots?.takeIf { it.isNotEmpty() } ?: return null
    val live = slots.firstOrNull { it.live }
        ?: return "The switch on channel ${view.channel} is not on any mode slot."
    return "The switch on channel ${view.channel} is on slot ${live.slot}, ${live.mode}."
}
