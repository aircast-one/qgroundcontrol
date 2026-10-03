package one.aircast.android.ui

import org.json.JSONObject
import one.aircast.mapspike.optText

internal const val FLIGHT_MODES = "view.flightModes"

internal data class FlightModeOption(
    val name: String,
    val summary: String,
    val current: Boolean,
    val needsConfirm: Boolean,
    val hidden: Boolean,
    val section: String = "normal",
)

internal data class FlightModesView(
    val canSet: Boolean,
    val current: String,
    val currentSummary: String,
    val everyday: List<FlightModeOption>,
    val folded: List<FlightModeOption>,
    val all: List<FlightModeOption>,
    val hidden: List<String>,
    val hiddenSetting: String?,
    val unknownModeNotice: String = "",
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
                hidden = mode.optBoolean("hidden"),
                section = mode.optText("section").ifBlank { "normal" },
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
        everyday = options(view, "everyday"),
        folded = options(view, "folded"),
        all = options(view, "modes"),
        hidden = view.optJSONArray("hidden")?.let { list -> (0 until list.length()).map { list.optString(it) } }.orEmpty(),
        hiddenSetting = view.optText("hiddenSetting").ifBlank { null },
        unknownModeNotice = view.optText("unknownModeNotice"),
    )
}

internal data class ModeAck(val serial: Long, val accepted: Boolean, val wording: String)

internal fun modeAck(view: JSONObject?): ModeAck? =
    view?.optJSONObject("modeAck")?.let { ModeAck(it.optLong("serial"), it.optBoolean("accepted"), it.optText("wording")) }

internal const val MODE_REPLY_MS = 3000L
internal const val MODE_REJECTION_MS = 2500L

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

internal fun hiddenModesAfter(hidden: List<String>, mode: String, hide: Boolean): String =
    (hidden.filter { it != mode } + listOfNotNull(mode.takeIf { hide })).joinToString(",")

internal fun startsSection(shown: List<FlightModeOption>, index: Int): Boolean =
    index > 0 && shown[index - 1].section != shown[index].section

internal const val HIDDEN_MODE_ALPHA = 0.55f

internal fun modeHeading(modes: FlightModesView?): String? {
    val summary = modes?.currentSummary?.ifBlank { null } ?: return null
    val name = modes.current.ifBlank { null } ?: return null
    return "$name — $summary"
}
