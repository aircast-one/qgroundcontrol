package one.aircast.android.ui

import one.aircast.map.optText
import org.json.JSONObject

internal data class DeviceStat(val label: String, val value: String, val hint: String, val level: Int)

internal data class ServiceRow(val label: String, val state: String, val level: Int)

internal object CloudStage {
    const val Linked = "linked"
    const val SigningIn = "signing-in"
    const val Connect = "connect"
}

internal data class CloudAccount(
    val status: DeviceBadge,
    val stage: String,
    val deviceId: String,
    val watchUrl: String,
    val userCode: String,
    val signInUrl: String,
    val error: String,
    val issuer: String,
)

internal data class SoftwareNotice(val level: Int, val title: String, val text: String, val detailLabel: String, val detail: String)

internal data class SoftwareCandidate(val title: String, val notesUrl: String, val install: String, val confirm: String, val success: String)

internal data class SoftwareStatus(
    val status: DeviceBadge,
    val summary: String,
    val notices: List<SoftwareNotice>,
    val candidate: SoftwareCandidate?,
    val notes: List<String>,
    val checked: String,
    val canCheck: Boolean,
)

internal data class HealthBadge(val level: Int, val text: String, val href: String)

internal data class DeviceSettingsView(
    val health: HealthBadge,
    val notice: DeviceBadge?,
    val stats: List<DeviceStat>,
    val services: List<ServiceRow>,
    val cloud: CloudAccount,
    val software: SoftwareStatus,
    val hostname: String,
)

private fun parseCandidate(json: JSONObject?): SoftwareCandidate? = json?.let {
    SoftwareCandidate(it.optText("title"), it.optText("notesUrl"), it.optText("install"), it.optText("confirm"), it.optText("success"))
}

private fun parseSoftware(json: JSONObject?): SoftwareStatus = SoftwareStatus(
    status = parseBadge(json?.optJSONObject("status")),
    summary = json?.optText("summary").orEmpty(),
    notices = json.deviceObjects("notices").map { SoftwareNotice(it.optInt("level"), it.optText("title"), it.optText("text"), it.optText("detailLabel"), it.optText("detail")) },
    candidate = parseCandidate(json?.optJSONObject("candidate")),
    notes = json.deviceTexts("notes"),
    checked = json?.optText("checked").orEmpty(),
    canCheck = json?.optBoolean("canCheck") == true,
)

private fun parseCloud(json: JSONObject?): CloudAccount = CloudAccount(
    status = parseBadge(json?.optJSONObject("status")),
    stage = json?.optText("stage").orEmpty().ifEmpty { CloudStage.Connect },
    deviceId = json?.optText("deviceId").orEmpty(),
    watchUrl = json?.optText("watchUrl").orEmpty(),
    userCode = json?.optText("userCode").orEmpty(),
    signInUrl = json?.optText("signInUrl").orEmpty(),
    error = json?.optText("error").orEmpty(),
    issuer = json?.optText("issuer").orEmpty(),
)

internal fun parseDeviceSettings(json: JSONObject): DeviceSettingsView? =
    json.optJSONObject("health")?.let { health ->
        DeviceSettingsView(
            health = HealthBadge(health.optInt("level"), health.optText("text"), health.optText("href")),
            notice = json.optJSONObject("notice")?.let(::parseBadge),
            stats = json.deviceObjects("stats").map { DeviceStat(it.optText("label"), it.optText("value"), it.optText("hint"), it.optInt("level")) },
            services = json.deviceObjects("services").map { ServiceRow(it.optText("label"), it.optText("state"), it.optInt("level")) },
            cloud = parseCloud(json.optJSONObject("cloud")),
            software = parseSoftware(json.optJSONObject("software")),
            hostname = json.optText("hostname"),
        )
    }

internal const val SERVICES = "Services"
