package one.aircast.android.ui

import one.aircast.map.optText
import org.json.JSONObject

internal data class DeviceSetupStep(val key: String, val label: String, val title: String, val lead: String, val status: DeviceBadge?)

internal data class SetupCheckRow(val label: String, val status: DeviceBadge, val step: Int)

internal data class SetupInternet(val online: DeviceBadge, val joinOpen: Boolean, val reopen: WifiReopen, val connection: DevicePage, val hint: String)

internal data class SetupCamera(val problem: String, val wizard: CameraWizardView, val watch: String, val cameras: DevicePage, val hint: String)

internal data class SetupTelemetry(val waiting: DeviceBadge?, val endpoints: List<DeviceEndpoint>, val hint: String)

internal data class SetupView(
    val start: Int,
    val steps: List<DeviceSetupStep>,
    val internet: SetupInternet,
    val remote: TailscaleCard,
    val camera: SetupCamera,
    val telemetry: SetupTelemetry,
    val checks: List<SetupCheckRow>,
)

internal enum class SetupNext { Next, Skip }

internal data class SetupNav(val back: Boolean, val later: Boolean, val next: SetupNext?, val finish: Boolean)

private fun parseInternet(json: JSONObject): SetupInternet {
    val reopen = json.optJSONObject("reopen") ?: JSONObject()
    return SetupInternet(
        online = parseBadge(json.optJSONObject("online")),
        joinOpen = json.optBoolean("joinOpen"),
        reopen = WifiReopen(reopen.optText("host"), reopen.optText("hostname")),
        connection = parseDevicePage(json.optJSONObject("connection")),
        hint = json.optText("hint"),
    )
}

internal fun parseDeviceSetup(json: JSONObject?): SetupView? {
    val camera = json?.optJSONObject("camera") ?: return null
    val wizard = parseCameraWizard(camera.optJSONObject("wizard")) ?: return null
    val telemetry = json.optJSONObject("telemetry") ?: JSONObject()
    val steps = json.deviceObjects("steps").map {
        DeviceSetupStep(it.optText("key"), it.optText("label"), it.optText("title"), it.optText("lead"), optDeviceBadge(it, "status"))
    }.ifEmpty { return null }
    return SetupView(
        start = clampStep(json.optInt("start", 1), steps.size),
        steps = steps,
        internet = parseInternet(json.optJSONObject("internet") ?: JSONObject()),
        remote = parseTailscale(json.optJSONObject("remote")),
        camera = SetupCamera(camera.optText("problem"), wizard, camera.optText("watch"), parseDevicePage(camera.optJSONObject("cameras")), camera.optText("hint")),
        telemetry = SetupTelemetry(optDeviceBadge(telemetry, "waiting"), parseEndpointList(telemetry, "endpoints"), telemetry.optText("hint")),
        checks = json.deviceObjects("checks").map { SetupCheckRow(it.optText("label"), parseBadge(it.optJSONObject("status")), it.optInt("step", 1)) },
    )
}

internal fun clampStep(step: Int, total: Int): Int = step.coerceAtLeast(1).coerceAtMost(total)

internal const val CAMERA_APPLYING = 3

internal fun cameraHoldsTheWizard(key: String, wizardShown: Boolean, wizardStep: Int): Boolean = key == "camera" && wizardShown && wizardStep != CAMERA_APPLYING

internal fun setupNav(step: Int, total: Int, held: Boolean): SetupNav = SetupNav(
    back = step > 1 && !held,
    later = step < total,
    next = when {
        step >= total -> null
        held -> SetupNext.Skip
        else -> SetupNext.Next
    },
    finish = step == total,
)
