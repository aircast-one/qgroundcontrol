package one.aircast.android.ui

import org.json.JSONObject
import one.aircast.map.optText

internal const val FLIGHT_MODES = "view.flightModes"

internal data class FlightModeOption(
    val name: String,
    val summary: String,
    val current: Boolean,
    val needsConfirm: Boolean,
    val section: String = "normal",
    val quick: Boolean = false,
    val caution: String = "",
)

internal data class FlightModesView(
    val canSet: Boolean,
    val current: String,
    val currentSummary: String,
    val all: List<FlightModeOption>,
    val quick: List<FlightModeOption>,
    val pinnedSetting: String?,
    val unknownModeNotice: String = "",
    val cannotSetNotice: String = "",
)

private fun options(view: JSONObject?, key: String): List<FlightModeOption> {
    val items = view?.optJSONArray(key) ?: return emptyList()
    return (0 until items.length()).mapNotNull { index ->
        items.optJSONObject(index)?.let { mode ->
            FlightModeOption(
                name = mode.optText("name"),
                summary = mode.optText("summary"),
                current = mode.optBoolean("current"),
                needsConfirm = mode.optBoolean("needsConfirm"),
                section = mode.optText("section").ifBlank { "normal" },
                quick = mode.optBoolean("quick"),
                caution = mode.optText("caution"),
            )
        }
    }
}

internal fun flightModesView(view: JSONObject?): FlightModesView? {
    if (view == null || !view.optBoolean("available")) return null
    return FlightModesView(
        canSet = view.optBoolean("canSet"),
        current = view.optText("current"),
        currentSummary = view.optText("currentSummary"),
        all = options(view, "modes"),
        quick = options(view, "quick"),
        pinnedSetting = view.optText("pinnedSetting").ifBlank { null },
        unknownModeNotice = view.optText("unknownModeNotice"),
        cannotSetNotice = view.optText("cannotSetNotice"),
    )
}

internal data class ModeAck(val serial: Long, val accepted: Boolean, val wording: String)

internal fun modeAck(view: JSONObject?): ModeAck? =
    view?.optJSONObject("modeAck")?.let { ModeAck(it.optLong("serial"), it.optBoolean("accepted"), it.optText("wording")) }

internal const val MODE_REPLY_MS = 3000L

internal sealed interface ModeOutcome {
    data object Pending : ModeOutcome
    data object Settled : ModeOutcome
    data class Rejected(val text: String) : ModeOutcome
}

internal fun modeOutcome(mode: String, before: ModeAck?, now: ModeAck?, reached: Boolean, elapsedMs: Long): ModeOutcome = when {
    reached -> ModeOutcome.Settled
    now != null && now.serial != before?.serial && !now.accepted -> ModeOutcome.Rejected("$mode ${now.wording}")
    elapsedMs >= MODE_REPLY_MS -> ModeOutcome.Rejected("$mode: no reply")
    else -> ModeOutcome.Pending
}

internal fun pinsAfter(quick: List<String>, mode: String, pin: Boolean): String =
    (quick.filter { it != mode } + listOfNotNull(mode.takeIf { pin })).joinToString(",")

internal fun startsSection(shown: List<FlightModeOption>, index: Int): Boolean =
    index > 0 && shown[index - 1].section != shown[index].section


internal fun modeHeading(modes: FlightModesView?): String? {
    val summary = modes?.currentSummary?.ifBlank { null } ?: return null
    val name = modes.current.ifBlank { null } ?: return null
    return "$name — $summary"
}
