package one.aircast.android.ui

import one.aircast.map.optText
import org.json.JSONObject
import java.io.File

internal data class RadioSettings(val status: DeviceBadge, val channel: String, val region: String, val txPower: String, val details: String)

internal object SharingStage {
    const val Intro = "intro"
    const val Deploy = "deploy"
    const val Live = "live"
}

internal data class PublicSharing(val status: DeviceBadge, val stage: String, val deployUrl: String, val signKey: String, val deviceToken: String, val baseUrl: String)

internal data class DetectModel(val name: String, val path: String, val about: String, val size: String, val classes: DeviceBadge, val active: Boolean)

internal data class DetectionSettings(val status: DeviceBadge, val unavailable: Boolean, val models: List<DetectModel>)

internal data class DevicePageLink(val title: String, val href: String, val summary: String)

internal data class VideoView(val radio: RadioSettings?, val sharing: PublicSharing, val detection: DetectionSettings, val engineering: DevicePageLink)

private fun parseRadio(json: JSONObject?): RadioSettings? = json?.let {
    RadioSettings(parseBadge(it.optJSONObject("status")), it.optText("channel"), it.optText("region"), it.optText("txPower"), it.optText("details"))
}

private fun parseSharing(json: JSONObject): PublicSharing =
    PublicSharing(parseBadge(json.optJSONObject("status")), json.optText("stage").ifEmpty { SharingStage.Intro }, json.optText("deployUrl"), json.optText("signKey"), json.optText("deviceToken"), json.optText("baseUrl"))

private fun parseDetection(json: JSONObject?): DetectionSettings = DetectionSettings(
    status = parseBadge(json?.optJSONObject("status")),
    unavailable = json?.optBoolean("unavailable") == true,
    models = json.deviceObjects("models").map {
        DetectModel(it.optText("name"), it.optText("path"), it.optText("about"), it.optText("size"), parseBadge(it.optJSONObject("classes")), it.optBoolean("active"))
    },
)

private fun parsePageLink(json: JSONObject?): DevicePageLink = DevicePageLink(json?.optText("title").orEmpty(), json?.optText("href").orEmpty(), json?.optText("summary").orEmpty())

internal fun parseVideo(json: JSONObject): VideoView? =
    json.optJSONObject("sharing")?.let { sharing ->
        VideoView(parseRadio(json.optJSONObject("radio")), parseSharing(sharing), parseDetection(json.optJSONObject("detection")), parsePageLink(json.optJSONObject("engineering")))
    }

internal val SHARING_OFF = mapOf("baseUrl" to "", "signKey" to "", "deviceToken" to "")

internal const val MODELS_PATH = "/api/detect/models"

internal fun modelPath(name: String): String = "$MODELS_PATH/${uriComponent(name)}"

internal data class ModelUpload(val hef: File, val labels: File?, val about: String, val use: Boolean)

internal fun modelFields(upload: ModelUpload): List<DevicePart> = listOfNotNull(
    DevicePart("hef", upload.hef),
    upload.labels?.let { DevicePart("labels", it) },
    upload.about.trim().takeIf { it.isNotEmpty() }?.let { DevicePart("about", value = it) },
    DevicePart("use", value = "1").takeIf { upload.use },
)

internal data class UploadOutcome(val refusal: DeviceRefusal?, val message: String)

internal fun uploadMessage(switched: Boolean, use: Boolean): UploadOutcome = when {
    switched -> UploadOutcome(null, "Model uploaded and switched — detection restarts on it.")
    use -> UploadOutcome(DeviceRefusal("Model uploaded, but switching to it failed — pick it from the list.", ""), "")
    else -> UploadOutcome(null, "Model uploaded — use it from the list when you want it.")
}

internal suspend fun uploadModel(upload: ModelUpload): UploadOutcome =
    deviceUpload(DeviceMethod.POST, MODELS_PATH, modelFields(upload)).let { answer ->
        answer.refusal?.let { UploadOutcome(it, "") } ?: uploadMessage(answer.result?.optBoolean("switched") == true, upload.use)
    }
