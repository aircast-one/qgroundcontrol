package one.aircast.android.ui

import one.aircast.map.optText
import org.json.JSONObject

internal data class CameraCopy(val label: String, val text: String, val qr: Boolean)

internal data class CameraFact(val label: String, val value: String)

internal data class CameraFix(val label: String, val scan: Boolean)

internal data class CameraChip(val key: String, val label: String, val level: Int, val href: String)

internal data class SetupPart(val title: String, val summary: String, val badge: DeviceBadge?, val open: Boolean)

internal data class NoticeLink(val label: String, val href: String)

internal data class LinkedNotice(val level: Int, val text: String, val link: NoticeLink?)

internal data class ServerDest(
    val kind: String,
    val part: SetupPart,
    val on: Boolean,
    val url: String,
    val key: String,
    val secret: String,
    val app: String,
    val track: String,
    val watch: String,
)

internal data class ShareLink(val id: String, val label: String, val url: String, val expires: String, val dead: String)

internal data class CameraWatch(
    val part: SetupPart,
    val cloud: Boolean,
    val canConnect: Boolean,
    val signIn: Boolean,
    val cloudLinks: List<DeviceEndpoint>,
    val cloudCopy: String,
    val links: List<DeviceEndpoint>,
)

internal data class CameraRadio(val part: SetupPart, val notice: LinkedNotice, val form: Boolean, val address: String, val stop: Boolean)

internal data class CameraServers(val part: SetupPart, val dests: List<ServerDest>)

internal data class CameraShares(val part: SetupPart, val links: List<ShareLink>, val hours: List<DeviceOption>)

internal data class CameraDetect(val part: SetupPart, val on: Boolean, val model: String, val notice: LinkedNotice?)

internal data class CameraRecord(val part: SetupPart, val on: Boolean, val full: Boolean, val usage: String)

internal data class CameraGimbal(val part: SetupPart, val owner: Boolean)

internal data class CameraSetup(
    val watch: CameraWatch,
    val radio: CameraRadio,
    val servers: CameraServers,
    val shares: CameraShares,
    val detect: CameraDetect,
    val record: CameraRecord,
    val gimbal: CameraGimbal?,
)

internal data class CameraView(
    val name: String,
    val status: DeviceBadge,
    val live: Boolean,
    val shared: Boolean,
    val preview: String,
    val placeholder: String,
    val watch: String,
    val copies: List<CameraCopy>,
    val facts: List<CameraFact>,
    val bitrate: DeviceChartView?,
    val diagnosis: DeviceBadge?,
    val fix: CameraFix?,
    val links: List<DeviceEndpoint>,
    val chips: List<CameraChip>,
    val recording: Boolean,
    val wizard: CameraWizardView,
    val setup: CameraSetup,
    val topic: DevicePage,
)

internal data class CamerasView(val cameras: List<CameraView>, val add: CameraWizardView, val cloudLink: String, val video: DevicePage)

internal fun parseSetupPart(json: JSONObject?): SetupPart =
    SetupPart(json?.optText("title").orEmpty(), json?.optText("summary").orEmpty(), json?.optJSONObject("badge")?.let(::parseBadge), json?.optBoolean("open") == true)

internal fun parseLinkedNotice(json: JSONObject?): LinkedNotice? = json?.let {
    val badge = parseBadge(it)
    LinkedNotice(badge.level, badge.text, it.optJSONObject("link")?.let { link -> NoticeLink(link.optText("label"), link.optText("href")) })
}

private val EMPTY_NOTICE = LinkedNotice(DeviceLevel.Neutral, "", null)

private fun partOf(json: JSONObject?): SetupPart = parseSetupPart(json?.optJSONObject("part"))

private fun parseSetup(json: JSONObject?): CameraSetup {
    val watch = json?.optJSONObject("watch")
    val radio = json?.optJSONObject("radio")
    val servers = json?.optJSONObject("servers")
    val shares = json?.optJSONObject("shares")
    val detect = json?.optJSONObject("detect")
    val record = json?.optJSONObject("record")
    return CameraSetup(
        watch = CameraWatch(
            part = partOf(watch),
            cloud = watch?.optBoolean("cloud") == true,
            canConnect = watch?.optBoolean("canConnect") == true,
            signIn = watch?.optBoolean("signIn") == true,
            cloudLinks = parseEndpointList(watch, "cloudLinks"),
            cloudCopy = watch?.optText("cloudCopy").orEmpty(),
            links = parseEndpointList(watch, "links"),
        ),
        radio = CameraRadio(
            part = partOf(radio),
            notice = parseLinkedNotice(radio?.optJSONObject("notice")) ?: EMPTY_NOTICE,
            form = radio?.optBoolean("form") == true,
            address = radio?.optText("address").orEmpty(),
            stop = radio?.optBoolean("stop") == true,
        ),
        servers = CameraServers(
            partOf(servers),
            servers.deviceObjects("dests").map {
                ServerDest(it.optText("kind"), partOf(it), it.optBoolean("on"), it.optText("url"), it.optText("key"), it.optText("secret"), it.optText("app"), it.optText("track"), it.optText("watch"))
            },
        ),
        shares = CameraShares(
            partOf(shares),
            shares.deviceObjects("links").map { ShareLink(it.optText("id"), it.optText("label"), it.optText("url"), it.optText("expires"), it.optText("dead")) },
            parseOptions(shares, "hours"),
        ),
        detect = CameraDetect(partOf(detect), detect?.optBoolean("on") == true, detect?.optText("model").orEmpty(), parseLinkedNotice(detect?.optJSONObject("notice"))),
        record = CameraRecord(partOf(record), record?.optBoolean("on") == true, record?.optBoolean("full") == true, record?.optText("usage").orEmpty()),
        gimbal = json?.optJSONObject("gimbal")?.let { CameraGimbal(partOf(it), it.optBoolean("owner")) },
    )
}

private fun parseCamera(json: JSONObject): CameraView? = parseCameraWizard(json.optJSONObject("wizard"))?.let { wizard ->
    CameraView(
        name = json.optText("name"),
        status = parseBadge(json.optJSONObject("status")),
        live = json.optBoolean("live"),
        shared = json.optBoolean("shared"),
        preview = json.optText("preview"),
        placeholder = json.optText("placeholder"),
        watch = json.optText("watch"),
        copies = json.deviceObjects("copies").map { CameraCopy(it.optText("label"), it.optText("text"), it.optBoolean("qr")) },
        facts = json.deviceObjects("facts").map { CameraFact(it.optText("label"), it.optText("value")) },
        bitrate = parseChart(json.optJSONObject("bitrate")),
        diagnosis = json.optJSONObject("diagnosis")?.let(::parseBadge),
        fix = json.optJSONObject("fix")?.let { CameraFix(it.optText("label"), it.optBoolean("scan")) },
        links = parseEndpointList(json, "links"),
        chips = json.deviceObjects("chips").map { CameraChip(it.optText("key"), it.optText("label"), it.optInt("level"), it.optText("href")) },
        recording = json.optBoolean("recording"),
        wizard = wizard,
        setup = parseSetup(json.optJSONObject("setup")),
        topic = parseDevicePage(json.optJSONObject("topic")),
    )
}

internal fun parseCameras(json: JSONObject): CamerasView? = parseCameraWizard(json.optJSONObject("add"))?.let { add ->
    CamerasView(json.deviceObjects("cameras").mapNotNull(::parseCamera), add, json.optText("cloudLink"), parseDevicePage(json.optJSONObject("video")))
}

internal sealed interface CamerasShown {
    data object Add : CamerasShown
    data object Wall : CamerasShown
    data class Camera(val camera: CameraView) : CamerasShown
}

internal fun camerasShown(cameras: List<CameraView>, picked: String, adding: Boolean): CamerasShown {
    val named = cameras.firstOrNull { it.name == picked }
    val only = cameras.singleOrNull()
    return when {
        adding || cameras.isEmpty() -> CamerasShown.Add
        named != null -> CamerasShown.Camera(named)
        only != null -> CamerasShown.Camera(only)
        else -> CamerasShown.Wall
    }
}

internal fun cameBack(before: Map<String, Boolean>, cameras: List<CameraView>): List<String> =
    cameras.filter { it.live && before[it.name] == false }.map { it.name }

internal fun liveness(cameras: List<CameraView>): Map<String, Boolean> = cameras.associate { it.name to it.live }

internal fun cloudflareTarget(app: String, track: String): String =
    if (app.isNotBlank() && track.isNotBlank()) "cloudflare://${app.trim()}/${track.trim()}" else ""

internal object CameraSetupSection {
    const val Watch = "cam-watch"
    const val Radio = "cam-radio"
    const val Servers = "cam-servers"
    const val Shares = "cam-share"
    const val Detect = "cam-detect"
    const val Record = "cam-record"
    const val Gimbal = "cam-gimbal"
}

internal fun openedSection(href: String): String = href.substringAfter('#', "")

internal fun cameraSectionHref(camera: String, section: String): String = "/settings/camera?camera=${uriComponent(camera)}#$section"
