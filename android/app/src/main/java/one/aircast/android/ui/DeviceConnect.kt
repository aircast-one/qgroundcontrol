package one.aircast.android.ui

import android.content.Context
import one.aircast.map.optText
import org.json.JSONObject

internal data class PageLink(val label: String, val href: String)

internal data class GcsChoice(val value: String, val label: String, val hides: List<String>)

internal data class ConnectAddress(val label: String, val url: String)

internal data class ConnectTarget(
    val proto: String,
    val label: String,
    val url: String,
    val hint: String,
    val tag: String,
    val down: Boolean,
    val reach: String,
    val qr: Boolean,
    val alternatives: List<ConnectAddress>,
)

internal data class ConnectCamera(val name: String, val status: DeviceBadge, val notice: String, val fix: PageLink?, val targets: List<ConnectTarget>)

internal data class ConnectGroup(val label: String, val targets: List<ConnectTarget>)

internal data class ConnectAddresses(val title: String, val groups: List<ConnectGroup>)

internal data class ConnectTrouble(val fallback: List<ConnectTarget>, val addresses: ConnectAddresses?)

internal data class ConnectTelemetry(val status: DeviceBadge, val targets: List<ConnectTarget>)

internal data class ConnectView(
    val choices: List<GcsChoice>,
    val scope: String,
    val cameras: List<ConnectCamera>,
    val noCamera: PageLink?,
    val telemetry: ConnectTelemetry?,
    val trouble: ConnectTrouble?,
)

private fun parseLink(json: JSONObject?): PageLink? = json?.let { PageLink(it.optText("label"), it.optText("href")) }

private fun parseTargets(json: JSONObject?, key: String): List<ConnectTarget> =
    json.deviceObjects(key).map {
        ConnectTarget(
            proto = it.optText("proto"),
            label = it.optText("label"),
            url = it.optText("url"),
            hint = it.optText("hint"),
            tag = it.optText("tag"),
            down = it.optBoolean("down"),
            reach = it.optText("reach"),
            qr = it.optBoolean("qr"),
            alternatives = it.deviceObjects("alternatives").map { alt -> ConnectAddress(alt.optText("label"), alt.optText("url")) },
        )
    }

private fun parseTrouble(json: JSONObject?): ConnectTrouble? = json?.let {
    ConnectTrouble(
        fallback = parseTargets(it, "fallback"),
        addresses = it.optJSONObject("addresses")?.let { addresses ->
            ConnectAddresses(addresses.optText("title"), addresses.deviceObjects("groups").map { group -> ConnectGroup(group.optText("label"), parseTargets(group, "targets")) })
        },
    )
}

internal fun parseConnect(json: JSONObject): ConnectView? =
    json.takeIf { it.optJSONArray("choices") != null }?.let {
        ConnectView(
            choices = it.deviceObjects("choices").map { choice -> GcsChoice(choice.optText("value"), choice.optText("label"), choice.deviceTexts("hides")) },
            scope = it.optText("scope"),
            cameras = it.deviceObjects("cameras").map { camera ->
                ConnectCamera(camera.optText("name"), parseBadge(camera.optJSONObject("status")), camera.optText("notice"), parseLink(camera.optJSONObject("fix")), parseTargets(camera, "targets"))
            },
            noCamera = parseLink(it.optJSONObject("noCamera")),
            telemetry = it.optJSONObject("telemetry")?.let { telemetry -> ConnectTelemetry(parseBadge(telemetry.optJSONObject("status")), parseTargets(telemetry, "targets")) },
            trouble = parseTrouble(it.optJSONObject("trouble")),
        )
    }

internal const val GCS_KEY = "gcs"

internal fun chosenGcs(choices: List<GcsChoice>, saved: String?): GcsChoice? = choices.firstOrNull { it.value == saved } ?: choices.firstOrNull()

internal fun shownTargets(targets: List<ConnectTarget>, choice: GcsChoice?): List<ConnectTarget> = targets.filterNot { choice?.hides?.contains(it.proto) == true }

internal const val REACH_RECHECK_MS = 30_000L

internal fun probeBlocked(url: String): Boolean = url.isEmpty()

private const val DEVICE_STORE = "device"

internal fun readStored(context: Context, key: String): String? = context.getSharedPreferences(DEVICE_STORE, Context.MODE_PRIVATE).getString(key, null)

internal fun writeStored(context: Context, key: String, value: String) = context.getSharedPreferences(DEVICE_STORE, Context.MODE_PRIVATE).edit().putString(key, value).apply()
