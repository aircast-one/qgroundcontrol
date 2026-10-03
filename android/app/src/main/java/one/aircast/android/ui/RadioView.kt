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
    val throttleReversed: Boolean = false,
    val stickPositions: List<Int> = List(4) { 0 },
)

internal data class RadioView(
    val connected: Boolean,
    val channelCount: Int,
    val summary: String,
    val shortfall: String,
    val calibration: RadioCalibration,
    val transmitterMode: Int,
    val enoughChannels: Boolean,
    val centeredThrottle: Boolean = false,
    val joystickMode: Boolean = false,
    val sticks: List<RadioStick>,
    val channels: List<RadioChannel>,
    val startPrompt: Pair<String, String>? = null,
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
        centeredThrottle = view.optBoolean("centeredThrottle"),
        joystickMode = view.optBoolean("joystickMode"),
        startPrompt = view.optJSONObject("startPrompt")?.let { it.optText("title") to it.optText("message") },
        calibration = RadioCalibration(
            running = view.optBoolean("calibrating"),
            statusText = view.optText("statusText"),
            nextText = view.optText("nextText"),
            nextEnabled = view.optBoolean("nextEnabled"),
            cancelEnabled = view.optBoolean("cancelEnabled"),
            skipEnabled = view.optBoolean("skipEnabled"),
            throttleReversed = view.optBoolean("throttleReversed"),
            stickPositions = view.optJSONArray("stickPositions")?.let { a -> (0 until a.length()).map(a::optInt) }?.takeIf { it.size == 4 } ?: List(4) { 0 },
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

internal data class RadioPrompt(
    val title: String,
    val body: String,
    val action: String,
    val choices: List<String>,
)

internal val RADIO_PROMPTS = listOf(
    RadioPrompt(
        title = "Spektrum bind",
        body = "Click Ok to place your Spektrum receiver in the bind mode.\n\nSelect the specific receiver type below:",
        action = "spektrumBindMode",
        choices = listOf("DSM2", "DSMX (7 channels or less)", "DSMX (8 channels or more)"),
    ),
    RadioPrompt(
        title = "CRSF bind",
        body = "Click Ok to place your CRSF receiver in the bind mode.",
        action = "crsfBindMode",
        choices = emptyList(),
    ),
    RadioPrompt(
        title = "Copy trims",
        body = "Center your sticks and move throttle all the way down, then press Ok to copy trims. After pressing Ok, reset the trims on your radio back to zero.",
        action = "copyTrims",
        choices = emptyList(),
    ),
)

internal const val THROTTLE_REVERSED_TITLE = "Throttle channel reversed"
internal const val THROTTLE_REVERSED_TEXT = "Calibration failed. The throttle channel on your transmitter is reversed. You must correct this on your transmitter in order to complete calibration."
