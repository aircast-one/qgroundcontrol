package one.aircast.android.ui

import android.content.Context
import androidx.compose.ui.geometry.Offset
import one.aircast.map.optText
import org.json.JSONObject
import java.time.OffsetDateTime
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import kotlin.math.roundToInt
import kotlin.math.roundToLong

internal data class FlightLine(val index: Int, val clock: String, val text: String, val level: Int, val alert: Boolean, val repeats: String, val heldBack: Boolean)

internal data class SortieShare(val label: String, val confirm: String, val camera: String, val from: String, val to: String)

internal data class SortieVideo(val camera: String, val label: String, val url: String, val share: SortieShare?)

internal data class FlightPart(val title: String, val span: String, val videos: List<SortieVideo>, val lines: List<FlightLine>)

internal data class TraceLane(
    val key: String,
    val label: String,
    val chip: String,
    val values: List<Double>,
    val notes: List<String>,
    val unit: String,
    val max: Double,
    val default: Boolean,
    val peak: String,
)

internal data class TraceMarker(val offset: Int, val label: String, val level: Int)

internal data class TraceSpan(val label: String, val from: Double, val to: Double)

internal data class FlightTrace(
    val step: Int,
    val startUnix: Long,
    val tzOffset: Int,
    val lanes: List<TraceLane>,
    val markers: List<TraceMarker>,
    val spans: List<TraceSpan>,
)

internal data class FlightEntry(
    val id: String,
    val title: String,
    val summary: String,
    val badge: DeviceBadge,
    val open: Boolean,
    val sections: List<FlightPart>,
    val trace: FlightTrace?,
    val dropped: String,
    val csv: String,
    val json: String,
)

internal data class FlightsView(val flights: List<FlightEntry>)

private fun traceValues(json: JSONObject, key: String): List<Double> =
    json.optJSONArray(key)?.let { array -> (0 until array.length()).map { index -> (array.opt(index) as? Number)?.toDouble() ?: -1.0 } }.orEmpty()

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

private fun parseTrace(json: JSONObject?): FlightTrace? = json?.let {
    FlightTrace(
        step = it.optInt("step"),
        startUnix = it.optLong("startUnix"),
        tzOffset = it.optInt("tzOffset"),
        lanes = it.deviceObjects("lanes").map { lane ->
            TraceLane(
                key = lane.optText("key"),
                label = lane.optText("label"),
                chip = lane.optText("chip"),
                values = traceValues(lane, "values"),
                notes = lane.deviceTexts("notes"),
                unit = lane.optText("unit"),
                max = lane.optDouble("max", 0.0),
                default = lane.optBoolean("default"),
                peak = lane.optText("peak"),
            )
        },
        markers = it.deviceObjects("markers").map { marker -> TraceMarker(marker.optInt("offset"), marker.optText("label"), marker.optInt("level", DeviceLevel.Neutral)) },
        spans = it.deviceObjects("spans").map { span -> TraceSpan(span.optText("label"), span.optDouble("from", 0.0), span.optDouble("to", 1.0)) },
    )
}

private fun parseFlight(json: JSONObject): FlightEntry =
    FlightEntry(
        id = json.optText("id"),
        title = json.optText("title"),
        summary = json.optText("summary"),
        badge = parseBadge(json.optJSONObject("badge")),
        open = json.optBoolean("open"),
        sections = json.deviceObjects("sections").map(::parsePart),
        trace = parseTrace(json.optJSONObject("trace")),
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

internal const val PLOT_TOP = 1.0
internal const val PLOT_HEIGHT = 18.0
internal const val PLOT_BOTTOM = PLOT_TOP + PLOT_HEIGHT
internal const val PLOT_BOX = 20.0

internal fun ceilingOf(values: List<Double>, max: Double): Double {
    val peak = values.fold(0.0) { top, value -> maxOf(top, value) }
    return if (max > 0) max else if (peak > 0) peak else 1.0
}

internal fun segments(values: List<Double>, max: Double): List<List<Offset>> {
    val ceiling = ceilingOf(values, max)
    val step = if (values.size > 1) 100.0 / (values.size - 1) else 100.0
    val gaps = values.indices.filter { values[it] < 0 }
    return (listOf(-1) + gaps + values.size).zipWithNext { after, before ->
        (after + 1 until before).map { index -> Offset((index * step).toFloat(), (PLOT_BOTTOM - PLOT_HEIGHT * minOf(values[index], ceiling) / ceiling).toFloat()) }
    }.filter { it.size > 1 }
}

internal fun formatRate(bytesPerSecond: Double): String {
    val bits = bytesPerSecond * 8
    return when {
        bits >= 1e6 -> "${oneDecimal(bits / 1e6)} Mbps"
        bits >= 1e3 -> "${(bits / 1e3).roundToLong()} kbps"
        else -> "${bits.roundToLong()} bps"
    }
}

private val UNIT_FORMATS: Map<String, (Double) -> String> = mapOf(
    "rate" to ::formatRate,
    "meters" to { value -> if (value >= 1000) "${oneDecimal(value / 1000)} km" else "${value.roundToLong()} m" },
    "count" to { value -> "${value.roundToLong()}" },
    "mb" to { value -> if (value >= 1024) "${oneDecimal(value / 1024)} GB" else "${value.roundToLong()} MB" },
    "seconds" to { value -> if (value >= 60) "${(value / 60).roundToLong()} m" else "${value.roundToLong()} s" },
    "number" to { value -> ((value * 10).roundToLong() / 10.0).let { if (it % 1.0 == 0.0) "${it.toLong()}" else "$it" } },
)

internal fun formatValue(value: Double, unit: String): String = UNIT_FORMATS[unit]?.invoke(value) ?: "${value.roundToLong()}%"

internal data class LaneTick(val x: Double, val label: String)

internal fun ticks(startUnix: Long, step: Int, count: Int, clock: (Long) -> String): List<LaneTick> {
    if (count <= 1 || step <= 0) return emptyList()
    val span = step.toLong() * (count - 1)
    val want = (span / 60 + 1).toInt().coerceIn(2, 5)
    return (0 until want).map { i -> LaneTick(i.toDouble() / (want - 1) * 100, clock(startUnix + span * i / (want - 1))) }
}

internal data class MarkerLine(val x: Double, val label: String, val level: Int, val index: Int)

internal fun markerLines(markers: List<TraceMarker>, step: Int, count: Int, shift: Int = 0): List<MarkerLine> {
    val span = step * (count - 1)
    return if (span > 0) {
        markers.mapIndexedNotNull { index, marker ->
            (marker.offset - shift).takeIf { it in 0..span }?.let { offset -> MarkerLine(offset.toDouble() / span * 100, marker.label, marker.level, index) }
        }
    } else {
        emptyList()
    }
}

internal fun zoomSlice(count: Int, zoom: Zoom?): IntRange {
    if (zoom == null || count < 2) return 0 until count
    val from = (zoom.from * (count - 1)).roundToInt().coerceIn(0, count - 2)
    return from..(zoom.to * (count - 1)).roundToInt().coerceIn(from + 1, count - 1)
}

internal fun composeZoom(current: Zoom?, a: Double, b: Double): Zoom {
    val window = current ?: Zoom(0.0, 1.0)
    val span = window.to - window.from
    return Zoom(window.from + minOf(a, b) * span, window.from + maxOf(a, b) * span)
}

internal const val MIN_BRUSH = 0.02

internal fun spanLabel(seconds: Int): String =
    if (seconds >= 3600) "${seconds / 3600}h ${(seconds % 3600 / 60.0).roundToInt()}m" else "${(seconds / 60.0).roundToInt()}m"

private fun two(n: Long): String = n.toString().padStart(2, '0')

internal fun clockAt(unix: Long, tzOffset: Int, withSeconds: Boolean): String {
    val day = Math.floorMod(unix + tzOffset, 86_400L)
    val base = "${two(day / 3600)}:${two(day % 3600 / 60)}"
    return if (withSeconds) "$base:${two(day % 60)}" else base
}

internal data class LaneGridLine(val y: Double, val label: String)

internal fun gridLines(ceiling: Double, unit: String): List<LaneGridLine> =
    if (ceiling > 0) listOf(0.25, 0.5, 0.75).map { LaneGridLine(PLOT_BOTTOM - PLOT_HEIGHT * it, formatValue(ceiling * it, unit)) } else emptyList()

internal data class LaneReadout(val index: Int, val x: Double, val text: String)

internal fun readoutAt(values: List<Double>, notes: List<String>, fraction: Double, unit: String): LaneReadout? {
    if (values.isEmpty()) return null
    val index = (fraction * (values.size - 1)).roundToInt().coerceIn(0, values.size - 1)
    val value = values[index]
    val note = notes.getOrNull(index).orEmpty()
    val reading = if (value < 0) "no reading" else formatValue(value, unit)
    return LaneReadout(index, if (values.size > 1) index.toDouble() / (values.size - 1) * 100 else 0.0, if (note.isNotEmpty()) "$reading · $note" else reading)
}

internal data class RenderJob(val id: String, val state: String, val sent: Long, val total: Long, val url: String, val expiresAt: String, val error: String)

internal const val RENDER_POLL_MS = 2000L

internal val IDLE_RENDER = RenderJob("", "", 0, 0, "", "", "")

internal fun parseRenderJob(json: JSONObject?): RenderJob = (json ?: JSONObject()).let {
    val state = it.optText("state")
    RenderJob(
        id = it.optText("id"),
        state = state,
        sent = it.optLong("sent"),
        total = it.optLong("total"),
        url = it.optText("url"),
        expiresAt = it.optText("expiresAt"),
        error = if (state == "failed") it.optText("error").ifEmpty { "The cloud could not render this video." } else "",
    )
}

private fun failedRender(error: String): RenderJob = IDLE_RENDER.copy(state = "failed", error = error)

internal fun renderRunning(job: RenderJob): Boolean = job.state == "uploading" || job.state == "rendering"

internal fun renderPercent(job: RenderJob): Int = if (job.total > 0) (job.sent * 100 / job.total).toInt() else 0

private val EXPIRY_DATE = DateTimeFormatter.ofPattern("d MMM")

internal fun renderExpiry(job: RenderJob): String =
    runCatching { OffsetDateTime.parse(job.expiresAt).atZoneSameInstant(ZoneId.systemDefault()).format(EXPIRY_DATE) }.getOrNull()?.let { "Link works until $it" }.orEmpty()

internal suspend fun startRender(share: SortieShare): RenderJob =
    deviceSend(DeviceMethod.POST, "/api/renders", JSONObject(mapOf("camera" to share.camera, "from" to share.from, "to" to share.to))).let { answer ->
        answer.refusal?.let { failedRender(it.message) } ?: parseRenderJob(answer.result)
    }

internal suspend fun readRender(id: String): RenderJob = deviceRead("/api/renders/${uriComponent(id)}", ::parseRenderJob) ?: failedRender("Lost track of the render.")
