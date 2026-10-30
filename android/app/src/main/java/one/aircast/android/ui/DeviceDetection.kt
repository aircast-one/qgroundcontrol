package one.aircast.android.ui

import one.aircast.map.optText
import org.json.JSONObject
import java.io.File

internal data class DetectModel(val name: String, val path: String, val about: String, val size: String, val classes: DeviceBadge, val engine: String, val active: Boolean)

internal data class DetectCamera(val name: String, val on: Boolean, val status: DeviceBadge)

internal data class DetectionView(val status: DeviceBadge, val unavailable: Boolean, val cameras: List<DetectCamera>, val models: List<DetectModel>)

internal const val NO_LABELS = "no labels"

internal fun parseDetection(json: JSONObject): DetectionView? =
    json.optJSONObject("status")?.let { status ->
        DetectionView(
            status = parseBadge(status),
            unavailable = json.optBoolean("unavailable"),
            cameras = json.deviceObjects("cameras").map { DetectCamera(it.optText("name"), it.optBoolean("on"), parseBadge(it.optJSONObject("status"))) },
            models = json.deviceObjects("models").map {
                DetectModel(it.optText("name"), it.optText("path"), it.optText("about"), it.optText("size"), parseBadge(it.optJSONObject("classes")), it.optText("engine"), it.optBoolean("active"))
            },
        )
    }

internal fun detecting(view: DetectionView): Boolean = view.cameras.any { it.on }

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

internal fun uploadMessage(switched: Boolean, use: Boolean, cameraDetecting: Boolean): UploadOutcome = when {
    switched && cameraDetecting -> UploadOutcome(null, "Model uploaded and switched — detection restarts on it.")
    switched -> UploadOutcome(null, "Model uploaded and switched — no camera is detecting yet. Turn it on for a camera on this page.")
    use -> UploadOutcome(DeviceRefusal("Model uploaded, but switching to it failed — pick it from the list.", ""), "")
    else -> UploadOutcome(null, "Model uploaded — use it from the list when you want it.")
}

internal suspend fun uploadModel(upload: ModelUpload, cameraDetecting: Boolean): UploadOutcome =
    deviceUpload(DeviceMethod.POST, MODELS_PATH, modelFields(upload)).let { answer ->
        answer.refusal?.let { UploadOutcome(it, "") } ?: uploadMessage(answer.result?.optBoolean("switched") == true, upload.use, cameraDetecting)
    }

internal fun labelsPath(name: String): String = "${modelPath(name)}/labels"

internal suspend fun addLabels(name: String, labels: File): DeviceRefusal? =
    deviceUpload(DeviceMethod.PUT, labelsPath(name), listOf(DevicePart("labels", labels))).refusal
