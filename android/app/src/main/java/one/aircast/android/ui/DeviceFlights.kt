package one.aircast.android.ui

import android.content.Context
import androidx.compose.ui.geometry.Offset
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.map.optText
import org.json.JSONArray
import org.json.JSONObject
import kotlin.math.roundToInt

internal data class FlightLine(val index: Int, val clock: String, val text: String, val level: Int, val alert: Boolean, val repeats: String, val heldBack: Boolean)

internal data class SortieShare(val label: String, val confirm: String, val camera: String, val from: String, val to: String)

internal data class SortieVideo(val camera: String, val label: String, val url: String, val share: SortieShare?)

internal data class FlightPart(val title: String, val span: String, val videos: List<SortieVideo>, val lines: List<FlightLine>)

internal data class FlightEntry(
    val id: String,
    val title: String,
    val summary: String,
    val badge: DeviceBadge,
    val open: Boolean,
    val sections: List<FlightPart>,
    val trace: JSONObject?,
    val dropped: String,
    val csv: String,
    val json: String,
)

internal data class FlightsView(val flights: List<FlightEntry>)

private fun parseShare(json: JSONObject?): SortieShare? = json?.let {
    SortieShare(it.optText("label"), it.optText("confirm"), it.optText("camera"), it.optText("from"), it.optText("to"))
}

private fun parsePart(json: JSONObject): FlightPart =
    FlightPart(
        title = json.optText("title"),
        span = json.optText("span"),
        videos = json.deviceObjects("videos").map { SortieVideo(it.optText("camera"), it.optText("label"), it.optText("url"), parseShare(it.optJSONObject("share"))) },
        lines = json.deviceObjects("lines").map {
            FlightLine(it.optInt("index"), it.optText("clock"), it.optText("text"), it.optInt("level", DeviceLevel.Neutral), it.optBoolean("alert"), it.optText("repeats"), it.optBoolean("heldBack"))
        },
    )

private fun parseFlight(json: JSONObject): FlightEntry =
    FlightEntry(
        id = json.optText("id"),
        title = json.optText("title"),
        summary = json.optText("summary"),
        badge = parseBadge(json.optJSONObject("badge")),
        open = json.optBoolean("open"),
        sections = json.deviceObjects("sections").map(::parsePart),
        trace = json.optJSONObject("trace"),
        dropped = json.optText("dropped"),
        csv = json.optText("csv"),
        json = json.optText("json"),
    )

internal fun parseFlights(json: JSONObject): FlightsView? = json.optJSONArray("flights")?.let { FlightsView(it.deviceObjects().map(::parseFlight)) }

internal fun hasLines(flight: FlightEntry): Boolean = flight.sections.any { it.lines.isNotEmpty() }

internal fun flightLink(base: String, id: String): String = "$base#device=${uriComponent("/flights?flight=$id")}"

internal const val LANES_KEY = "flight-lanes"

internal fun laneKeys(saved: List<String>?, defaults: List<String>): List<String> = saved ?: defaults

internal fun toggled(keys: List<String>, key: String): List<String> = if (key in keys) keys.filter { it != key } else keys + key

internal fun storedLanes(context: Context): List<String>? = readStored(context, LANES_KEY)?.split(',')?.filter { it.isNotEmpty() }

internal fun storeLanes(context: Context, keys: List<String>) = writeStored(context, LANES_KEY, keys.joinToString(","))

internal data class Zoom(val from: Double, val to: Double)

internal fun composeZoom(current: Zoom?, a: Double, b: Double): Zoom {
    val window = current ?: Zoom(0.0, 1.0)
    val span = window.to - window.from
    return Zoom(window.from + minOf(a, b) * span, window.from + maxOf(a, b) * span)
}

internal const val MIN_BRUSH = 0.02

internal data class ChartGrid(val y: Float, val label: String)

internal data class ChartTick(val x: Float, val label: String)

internal data class ChartMarker(val x: Float, val label: String, val level: Int, val index: Int)

internal data class ChartLane(
    val key: String,
    val label: String,
    val chip: String,
    val peak: String,
    val default: Boolean,
    val lines: List<List<Offset>>,
    val grid: List<ChartGrid>,
    val markers: List<ChartMarker>,
    val readings: List<String>,
    val ticks: List<ChartTick>,
    val span: String,
)

internal data class ChartSpan(val label: String, val from: Double, val to: Double)

internal data class FlightChart(val lanes: List<ChartLane>, val spans: List<ChartSpan>)

private const val FLIGHT_CHART = "deviceFlight.chart"

private fun JSONArray?.chartArrays(): List<JSONArray> = this?.let { array -> (0 until array.length()).mapNotNull(array::optJSONArray) }.orEmpty()

private fun parseChartLane(json: JSONObject): ChartLane =
    ChartLane(
        key = json.optText("key"),
        label = json.optText("label"),
        chip = json.optText("chip"),
        peak = json.optText("peak"),
        default = json.optBoolean("default"),
        lines = json.optJSONArray("lines").chartArrays().map { run -> run.chartArrays().map { Offset(it.optDouble(0).toFloat(), it.optDouble(1).toFloat()) } },
        grid = json.deviceObjects("grid").map { ChartGrid(it.optDouble("y").toFloat(), it.optText("label")) },
        markers = json.deviceObjects("markers").map { ChartMarker(it.optDouble("x").toFloat(), it.optText("label"), it.optInt("level", DeviceLevel.Neutral), it.optInt("index")) },
        readings = json.deviceTexts("readings"),
        ticks = json.deviceObjects("ticks").map { ChartTick(it.optDouble("x").toFloat(), it.optText("label")) },
        span = json.optText("span"),
    )

internal fun parseFlightChart(json: JSONObject?): FlightChart? = json?.let {
    FlightChart(it.deviceObjects("lanes").map(::parseChartLane), it.deviceObjects("spans").map { span -> ChartSpan(span.optText("label"), span.optDouble("from", 0.0), span.optDouble("to", 1.0)) })
}

internal suspend fun flightChart(trace: JSONObject, zoom: Zoom?): FlightChart? =
    withContext(Dispatchers.Default) { parseFlightChart(Qgc.call(FLIGHT_CHART, trace, zoom?.from, zoom?.to)?.optJSONObject("result")) }

internal data class ChartSample(val index: Int, val x: Float)

internal fun sampleAt(count: Int, fraction: Double): ChartSample? =
    if (count == 0) null else (fraction * (count - 1)).roundToInt().coerceIn(0, count - 1).let { ChartSample(it, if (count > 1) it.toFloat() / (count - 1) * 100 else 0f) }

internal data class FlightRender(val canStart: Boolean, val progress: String, val error: String, val url: String, val expiry: String)

private const val FLIGHT_RENDER = "deviceFlight.render"

internal fun renderPath(share: SortieShare): String = "view.deviceRender(${share.camera},${share.from},${share.to})"

internal fun parseRender(json: JSONObject?): FlightRender =
    FlightRender(json?.optBoolean("canStart", true) ?: true, json?.optText("progress").orEmpty(), json?.optText("error").orEmpty(), json?.optText("url").orEmpty(), json?.optText("expiry").orEmpty())

internal suspend fun requestRender(share: SortieShare): Boolean = withContext(Dispatchers.Default) { Qgc.invoke(FLIGHT_RENDER, share.camera, share.from, share.to) }
