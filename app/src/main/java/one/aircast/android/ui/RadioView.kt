package one.aircast.android.ui

import org.json.JSONObject
import one.aircast.mapspike.optText

internal const val RADIO_VIEW = "view.radio"

internal data class RadioStick(
    val title: String,
    val valueText: String,
    val fraction: Float,
    val mapped: Boolean,
    val reversed: Boolean,
)

internal data class RadioChannel(
    val label: String,
    val valueText: String,
    val fraction: Float,
    val live: Boolean,
)

internal data class RadioCalibration(
    val running: Boolean,
    val statusText: String,
    val nextText: String,
    val nextEnabled: Boolean,
    val cancelEnabled: Boolean,
    val skipEnabled: Boolean,
)

internal data class RadioView(
    val connected: Boolean,
    val channelCount: Int,
    val summary: String,
    val shortfall: String,
    val calibration: RadioCalibration,
    val transmitterMode: Int,
    val enoughChannels: Boolean,
    val sticks: List<RadioStick>,
    val channels: List<RadioChannel>,
)

private fun <T> each(view: JSONObject?, key: String, make: (JSONObject) -> T): List<T> {
    val items = view?.optJSONArray(key) ?: return emptyList()
    return (0 until items.length()).mapNotNull { items.optJSONObject(it)?.let(make) }
}

internal fun radioView(view: JSONObject?): RadioView? {
    if (view == null || view.optText("class") != "Radio") return null
    return RadioView(
        connected = view.optBoolean("connected"),
        channelCount = view.optInt("channelCount"),
        summary = view.optText("summary"),
        shortfall = view.optText("shortfall"),
        transmitterMode = view.optInt("transmitterMode", 2),
        enoughChannels = view.optBoolean("enoughChannels"),
        calibration = RadioCalibration(
            running = view.optBoolean("calibrating"),
            statusText = view.optText("statusText"),
            nextText = view.optText("nextText"),
            nextEnabled = view.optBoolean("nextEnabled"),
            cancelEnabled = view.optBoolean("cancelEnabled"),
            skipEnabled = view.optBoolean("skipEnabled"),
        ),
        sticks = each(view, "sticks") {
            RadioStick(
                title = it.optText("title"),
                valueText = it.optText("valueText"),
                fraction = it.optDouble("fraction", 0.0).toFloat(),
                mapped = it.optBoolean("mapped"),
                reversed = it.optBoolean("reversed"),
            )
        },
        channels = each(view, "channels") {
            RadioChannel(
                label = it.optText("label"),
                valueText = it.optText("valueText"),
                fraction = it.optDouble("fraction", 0.0).toFloat(),
                live = it.optBoolean("live"),
            )
        },
    )
}

internal const val RADIO_CAL = "radioCal"

internal fun radioCalAction(action: String): String = "$RADIO_CAL.$action"

internal fun calibrationStep(statusText: String): String =
    statusText.trim()
        .lines()
        .dropLastWhile { it.isBlank() || it.trim().startsWith("Click ") }
        .joinToString("\n")
        .trim()
