package one.aircast.android.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.key
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.map.optText
import org.json.JSONArray
import org.json.JSONObject

internal data class DeviceEndpoint(
    val proto: String,
    val label: String,
    val url: String,
    val hint: String,
    val note: String,
    val appLink: String,
    val down: Boolean,
    val primary: Boolean,
)

internal data class TelemetryGimbal(
    val title: String,
    val status: DeviceBadge,
    val about: String,
    val vendor: String,
    val model: String,
    val cameraAddr: String,
    val mavlinkAddr: String,
    val vendors: List<DeviceOption>,
    val models: List<DeviceOption>,
)

internal data class TelemetryCloud(val status: DeviceBadge, val text: String)

internal data class TelemetrySettings(val endpoints: String, val baud: String)

internal data class TelemetryView(
    val status: DeviceBadge,
    val notices: List<DeviceBadge>,
    val endpoints: List<DeviceEndpoint>,
    val endpointNotice: DeviceBadge?,
    val cloud: TelemetryCloud?,
    val settings: TelemetrySettings,
    val schemes: JSONArray,
    val gimbal: TelemetryGimbal?,
)

internal fun parseEndpointList(json: JSONObject?, key: String): List<DeviceEndpoint> =
    json.deviceObjects(key).map {
        DeviceEndpoint(it.optText("proto"), it.optText("label"), it.optText("url"), it.optText("hint"), it.optText("note"), it.optText("appLink"), it.optBoolean("down"), it.optBoolean("primary"))
    }

private fun parseGimbal(json: JSONObject?): TelemetryGimbal? = json?.let {
    TelemetryGimbal(
        title = it.optText("title"),
        status = parseBadge(it.optJSONObject("status")),
        about = it.optText("about"),
        vendor = it.optText("vendor"),
        model = it.optText("model"),
        cameraAddr = it.optText("cameraAddr"),
        mavlinkAddr = it.optText("mavlinkAddr"),
        vendors = parseOptions(it, "vendors"),
        models = parseOptions(it, "models"),
    )
}

internal fun parseTelemetry(json: JSONObject): TelemetryView? =
    json.optJSONObject("status")?.let { status ->
        val settings = json.optJSONObject("settings")
        TelemetryView(
            status = parseBadge(status),
            notices = parseBadges(json, "notices"),
            endpoints = parseEndpointList(json, "endpoints"),
            endpointNotice = json.optJSONObject("endpointNotice")?.let(::parseBadge),
            cloud = json.optJSONObject("cloud")?.let { TelemetryCloud(parseBadge(it.optJSONObject("status")), it.optText("text")) },
            settings = TelemetrySettings(settings?.optText("endpoints").orEmpty(), settings?.optText("baud").orEmpty()),
            schemes = json.optJSONArray("schemes") ?: JSONArray(),
            gimbal = parseGimbal(json.optJSONObject("gimbal")),
        )
    }

internal data class EndpointRow(val scheme: String, val host: String, val port: String)

internal data class EndpointOption(val value: String, val label: String, val row: EndpointRow)

internal data class EndpointRowView(val row: EndpointRow, val hostLabel: String, val placeholder: String, val note: String, val problem: String, val options: List<EndpointOption>)

internal data class EndpointEditor(val rows: List<EndpointRowView>, val blank: EndpointRowView, val endpoints: String)

internal const val TELEMETRY_ENDPOINTS = "deviceTelemetry.endpoints"

private fun parseRow(json: JSONObject?): EndpointRow = EndpointRow(json?.optText("scheme").orEmpty(), json?.optText("host").orEmpty(), json?.optText("port").orEmpty())

private fun parseRowView(json: JSONObject?): EndpointRowView =
    EndpointRowView(
        row = parseRow(json),
        hostLabel = json?.optText("hostLabel").orEmpty(),
        placeholder = json?.optText("placeholder").orEmpty(),
        note = json?.optText("note").orEmpty(),
        problem = json?.optText("problem").orEmpty(),
        options = json.deviceObjects("options").map { EndpointOption(it.optText("value"), it.optText("label"), parseRow(it.optJSONObject("row"))) },
    )

internal fun parseEndpointEditor(answer: JSONObject?): EndpointEditor? =
    answer?.takeIf { it.optBoolean("ok") }?.optJSONObject("result")?.let {
        EndpointEditor(it.deviceObjects("rows").map(::parseRowView), parseRowView(it.optJSONObject("blank")), it.optText("endpoints"))
    }

private fun rowsJson(rows: List<EndpointRow>): JSONArray = JSONArray(rows.map { JSONObject().put("scheme", it.scheme).put("host", it.host).put("port", it.port) })

internal suspend fun endpointEditor(schemes: JSONArray, rows: List<EndpointRow>?, stored: String): EndpointEditor? =
    withContext(Dispatchers.Default) { parseEndpointEditor(Qgc.call(TELEMETRY_ENDPOINTS, schemes, rows?.let(::rowsJson) ?: stored)) }

internal fun endpointLabel(endpoint: DeviceEndpoint): String =
    if (endpoint.proto.isNotEmpty() && !endpoint.label.startsWith(endpoint.proto)) "${endpoint.proto} · ${endpoint.label}" else endpoint.label

@Composable
internal fun EndpointRows(endpoints: List<DeviceEndpoint>) {
    endpoints.map { endpoint ->
        key(endpoint.url) {
            CopyRow(
                label = endpointLabel(endpoint),
                text = endpoint.url,
                hint = listOf(endpoint.hint, endpoint.note).filter { it.isNotEmpty() }.joinToString(" "),
                qr = true,
                level = if (endpoint.down) DeviceLevel.Warn else DeviceLevel.Neutral,
            )
        }
    }
}
