package one.aircast.android.ui

import one.aircast.map.optText
import org.json.JSONObject

internal data class RadioSettings(val status: DeviceBadge, val channel: String, val region: String, val txPower: String, val details: String)

internal object SharingStage {
    const val Intro = "intro"
    const val Deploy = "deploy"
    const val Live = "live"
}

internal data class PublicSharing(val status: DeviceBadge, val stage: String, val deployUrl: String, val signKey: String, val deviceToken: String, val baseUrl: String)

internal data class DevicePageLink(val title: String, val href: String, val summary: String)

internal data class VideoView(val radio: RadioSettings?, val sharing: PublicSharing, val engineering: DevicePageLink)

private fun parseRadio(json: JSONObject?): RadioSettings? = json?.let {
    RadioSettings(parseBadge(it.optJSONObject("status")), it.optText("channel"), it.optText("region"), it.optText("txPower"), it.optText("details"))
}

private fun parseSharing(json: JSONObject): PublicSharing =
    PublicSharing(parseBadge(json.optJSONObject("status")), json.optText("stage").ifEmpty { SharingStage.Intro }, json.optText("deployUrl"), json.optText("signKey"), json.optText("deviceToken"), json.optText("baseUrl"))

private fun parsePageLink(json: JSONObject?): DevicePageLink = DevicePageLink(json?.optText("title").orEmpty(), json?.optText("href").orEmpty(), json?.optText("summary").orEmpty())

internal fun parseVideo(json: JSONObject): VideoView? =
    json.optJSONObject("sharing")?.let { sharing ->
        VideoView(parseRadio(json.optJSONObject("radio")), parseSharing(sharing), parsePageLink(json.optJSONObject("engineering")))
    }

internal val SHARING_OFF = mapOf("baseUrl" to "", "signKey" to "", "deviceToken" to "")
