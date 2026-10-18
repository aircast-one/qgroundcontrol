package one.aircast.android.ui

import one.aircast.map.optText
import org.json.JSONObject

internal data class RecordingsLimit(val value: String, val options: List<DeviceOption>, val used: String)

internal data class RecordCamera(val name: String, val recording: Boolean, val note: String)

internal data class RecordingSegment(
    val at: String,
    val started: String,
    val duration: String,
    val size: String,
    val url: String,
    val open: Boolean,
    val noTelemetry: Boolean,
)

internal data class RecordedCamera(
    val name: String,
    val status: DeviceBadge,
    val summary: String,
    val full: Boolean,
    val usage: String,
    val segments: List<RecordingSegment>,
)

internal data class RecordingsView(val limit: RecordingsLimit, val cameras: List<RecordCamera>, val recorded: List<RecordedCamera>)

private fun parseSegment(json: JSONObject): RecordingSegment =
    RecordingSegment(json.optText("at"), json.optText("started"), json.optText("duration"), json.optText("size"), json.optText("url"), json.optBoolean("open"), json.optBoolean("noTelemetry"))

private fun parseRecorded(json: JSONObject): RecordedCamera =
    RecordedCamera(json.optText("name"), parseBadge(json.optJSONObject("status")), json.optText("summary"), json.optBoolean("full"), json.optText("usage"), json.deviceObjects("segments").map(::parseSegment))

internal fun parseRecordings(json: JSONObject): RecordingsView? =
    json.optJSONObject("limit")?.let { limit ->
        RecordingsView(
            limit = RecordingsLimit(limit.optText("value"), parseOptions(limit, "options"), limit.optText("used")),
            cameras = json.deviceObjects("cameras").map { RecordCamera(it.optText("name"), it.optBoolean("recording"), it.optText("note")) },
            recorded = json.deviceObjects("recorded").map(::parseRecorded),
        )
    }

internal fun lowersLimit(current: String, picked: String): Boolean = picked.toLongOrNull()?.let { next -> current.toLongOrNull()?.let { next < it } } == true

internal fun recordPath(camera: String): String = "/api/streams/${uriComponent(camera)}/record"

internal fun recordingsPath(camera: String): String = "/api/recordings/${uriComponent(camera)}"

internal fun segmentPath(camera: String, segment: RecordingSegment): String = "${recordingsPath(camera)}?start=${uriComponent(segment.at)}"

internal fun segmentDetail(segment: RecordingSegment): List<String> =
    listOfNotNull(segment.duration.takeIf { it.isNotEmpty() }, segment.size, "no flight data".takeIf { segment.noTelemetry }, "downloadable once it closes".takeIf { segment.open })
