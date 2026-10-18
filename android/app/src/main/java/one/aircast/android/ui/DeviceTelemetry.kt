package one.aircast.android.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.key
import one.aircast.map.optText
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

internal data class MavlinkScheme(val value: String, val label: String, val host: String, val port: String, val hostLabel: String, val placeholder: String, val hint: String)

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
    val schemes: List<MavlinkScheme>,
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
            schemes = json.deviceObjects("schemes").map {
                MavlinkScheme(it.optText("value"), it.optText("label"), it.optText("host"), it.optText("port"), it.optText("hostLabel"), it.optText("placeholder"), it.optText("hint"))
            },
            gimbal = parseGimbal(json.optJSONObject("gimbal")),
        )
    }

internal data class EndpointRow(val scheme: String, val host: String, val port: String)

internal const val UNKNOWN_SCHEME_HINT = "Set outside this page — saved exactly as it stands unless you change the type."

private const val MIN_PORT = 1
private const val MAX_PORT = 65535
private const val DEFAULT_PORT = "14550"

private fun schemeOf(schemes: List<MavlinkScheme>, name: String): MavlinkScheme? = schemes.firstOrNull { it.value == name }

internal fun newRow(schemes: List<MavlinkScheme>, name: String = schemes.firstOrNull()?.value ?: "udps"): EndpointRow =
    EndpointRow(name, schemeOf(schemes, name)?.host.orEmpty(), schemeOf(schemes, name)?.port ?: DEFAULT_PORT)

internal fun parseEndpointSpec(spec: String): EndpointRow? =
    spec.takeIf { ':' in it }?.substringAfter(':')?.let { address -> EndpointRow(spec.substringBefore(':'), address.substringBeforeLast(':'), address.substringAfterLast(':', "")) }

internal fun parseEndpointSpecs(text: String): List<EndpointRow> =
    text.split('\n').map { it.trim() }.filter { it.isNotEmpty() }.mapNotNull(::parseEndpointSpec).filter { it.scheme.isNotEmpty() }

internal fun schemeOptions(schemes: List<MavlinkScheme>, row: EndpointRow): List<DeviceOption> =
    schemes.map { DeviceOption(it.value, it.label) } + listOfNotNull(DeviceOption(row.scheme, row.scheme).takeIf { row.scheme.isNotEmpty() && schemeOf(schemes, row.scheme) == null })

internal fun retargetRow(schemes: List<MavlinkScheme>, row: EndpointRow): EndpointRow {
    val defaults = schemes.map { it.host }.filter { it.isNotEmpty() }.toSet()
    val host = row.host.trim()
    return row.copy(
        host = if (host.isEmpty() || host in defaults) schemeOf(schemes, row.scheme)?.host.orEmpty() else row.host,
        port = if (row.port.isBlank()) schemeOf(schemes, row.scheme)?.port ?: DEFAULT_PORT else row.port,
    )
}

private fun portInRange(port: String): Boolean = port.all { it in '0'..'9' } && port.toIntOrNull() in MIN_PORT..MAX_PORT

internal fun endpointError(schemes: List<MavlinkScheme>, row: EndpointRow): String {
    val host = row.host.trim()
    val port = row.port.trim()
    return when {
        host.isEmpty() && port.isEmpty() -> ""
        !portInRange(port) -> "Needs a port between $MIN_PORT and $MAX_PORT."
        host.isEmpty() && schemeOf(schemes, row.scheme)?.host.isNullOrEmpty() -> "Needs an address."
        else -> ""
    }
}

internal fun serializeEndpoints(schemes: List<MavlinkScheme>, rows: List<EndpointRow>): String =
    rows.filter { endpointError(schemes, it).isEmpty() && (it.host.isNotBlank() || it.port.isNotBlank()) }
        .joinToString("\n") { "${it.scheme}:${it.host.trim().ifEmpty { schemeOf(schemes, it.scheme)?.host.orEmpty() }}:${it.port.trim()}" }

internal fun describeEndpoint(schemes: List<MavlinkScheme>, row: EndpointRow): String = schemeOf(schemes, row.scheme)?.hint ?: UNKNOWN_SCHEME_HINT

internal fun hostLabel(schemes: List<MavlinkScheme>, row: EndpointRow): String = schemeOf(schemes, row.scheme)?.hostLabel ?: "Address"

internal fun hostPlaceholder(schemes: List<MavlinkScheme>, row: EndpointRow): String = schemeOf(schemes, row.scheme)?.placeholder.orEmpty()

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
