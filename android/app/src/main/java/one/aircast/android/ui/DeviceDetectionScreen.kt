package one.aircast.android.ui

import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Checkbox
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.File

private const val MODEL_FIELD = "hef"

@Composable
internal fun DeviceDetectionScreen(props: DeviceScreenProps) {
    val view = deviceView("detection", ::parseDetection) ?: return FootNote("Waiting for the device to answer…")
    DeviceSection("Running") {
        StackedFact("In use", view.status.text)
        if (view.unavailable) {
            DeviceNotice(DeviceBadge(DeviceLevel.Warn, "This device has no AI accelerator, so .hef models can't run. Switch to an .onnx model to detect on the CPU. Draw-a-box tracking still works on every camera."))
        }
    }
    CamerasSection(view.cameras, props.onOpen)
    ModelsSection(view)
    StatusLine(
        "Detection runs on the device and sends boxes as data — the picture itself is untouched. Pick the model for what the camera is looking at: an aerial model hallucinates trucks on a desk. " +
            "Draw-a-box tracking works whichever model is active.",
    )
}

@Composable
private fun CamerasSection(cameras: List<DetectCamera>, onOpen: (String) -> Unit) {
    DeviceSection("Cameras") {
        if (cameras.isEmpty()) {
            DeviceText("No cameras yet.")
            TextButton(onClick = { onOpen("/streaming") }, modifier = Modifier.padding(horizontal = 8.dp)) { Text("Cameras →") }
        } else {
            cameras.map { camera ->
                key(camera.name) {
                    DeviceSwitchRow(
                        title = "Draw boxes on ${camera.name}",
                        detail = camera.status.text,
                        checked = camera.on,
                        on = DeviceRequest(DeviceMethod.PUT, "/api/streams/${camera.name}/detect"),
                        off = DeviceRequest(DeviceMethod.DELETE, "/api/streams/${camera.name}/detect"),
                        success = "Detection turned on for ${camera.name}.",
                        offSuccess = "Detection turned off for ${camera.name}.",
                    )
                }
            }
        }
    }
}

@Composable
private fun ModelsSection(view: DetectionView) {
    var uploads by remember { mutableIntStateOf(0) }
    DeviceSection("Models") {
        if (view.models.isEmpty()) DeviceText("No models installed yet.") else view.models.map { model -> key(model.name) { ModelRow(model) } }
        key(uploads) {
            DeviceDisclosure("Add a model", summary = "A .hef for the Hailo-8 or an .onnx for the CPU.") { ModelUploadForm(detecting(view)) { uploads += 1 } }
        }
    }
}

@Composable
private fun ModelRow(model: DetectModel) {
    Row(
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp).semantics { contentDescription = model.name },
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Column(Modifier.weight(1f)) {
            Text(model.name, style = MaterialTheme.typography.bodyLarge)
            if (model.about.isNotEmpty()) Text(model.about, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                Text(model.size, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                DeviceBadgeText(model.classes)
                if (model.engine.isNotEmpty()) Text(model.engine, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            if (model.classes.text == NO_LABELS) AddLabels(model.name)
        }
        if (model.active) {
            DeviceBadgeText(DeviceBadge(DeviceLevel.Ready, "Active"))
        } else {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                ActionButton("Use", DeviceRequest(DeviceMethod.PUT, "/api/stream/detect/settings", JSONObject(mapOf("hef" to model.path))), "Model switched — detection restarts on the new model.")
                ActionButton(
                    label = "Delete",
                    request = DeviceRequest(DeviceMethod.DELETE, modelPath(model.name)),
                    success = "Model deleted.",
                    confirm = ActionConfirm("Delete ${model.name}?", "Delete ${model.name}? Anyone relying on it loses it from this device.", "Delete"),
                    tonal = false,
                )
            }
        }
    }
}

private fun staged(context: Context, uri: Uri): File? = runCatching {
    val name = context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
        if (cursor.moveToFirst()) cursor.getString(0) else null
    }.orEmpty().replace('/', '_')
    File(context.cacheDir, name).takeIf { target -> context.contentResolver.openInputStream(uri)?.use { source -> target.outputStream().use { source.copyTo(it) } } != null }
}.getOrNull()

@Composable
private fun rememberFilePicker(onPick: suspend (File?) -> Unit): () -> Unit {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        val chosen = uri ?: return@rememberLauncherForActivityResult
        scope.launch { onPick(withContext(Dispatchers.IO) { staged(context, chosen) }) }
    }
    return { picker.launch(arrayOf("*/*")) }
}

@Composable
private fun AddLabels(name: String) {
    val notice = LocalChangeNotice.current
    var busy by remember { mutableStateOf(false) }
    var refusal by remember { mutableStateOf<DeviceRefusal?>(null) }
    val pick = rememberFilePicker { file ->
        file?.let {
            busy = true
            val refused = addLabels(name, it)
            busy = false
            refusal = refused
            if (refused == null) notice.show("Labels added to $name.", null)
        }
    }
    TextButton(onClick = pick, enabled = !busy, modifier = Modifier.semantics { contentDescription = "Labels for $name" }) { Text(if (busy) "Adding labels…" else "Add labels") }
    RefusalText(refusal)
}

@Composable
private fun FilePick(label: String, description: String, file: File?, onPick: (File?) -> Unit) {
    val pick = rememberFilePicker { onPick(it) }
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(label, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        OutlinedButton(onClick = pick, modifier = Modifier.semantics { contentDescription = description }) { Text(file?.name ?: "Choose a file") }
    }
}

@Composable
private fun ModelUploadForm(cameraDetecting: Boolean, onUploaded: () -> Unit) {
    val notice = LocalChangeNotice.current
    val scope = rememberCoroutineScope()
    var hef by remember { mutableStateOf<File?>(null) }
    var labels by remember { mutableStateOf<File?>(null) }
    var about by remember { mutableStateOf("") }
    var use by remember { mutableStateOf(true) }
    var busy by remember { mutableStateOf(false) }
    var refusal by remember { mutableStateOf<DeviceRefusal?>(null) }
    val upload = { file: File ->
        busy = true
        refusal = null
        scope.launch {
            val outcome = uploadModel(ModelUpload(file, labels, about, use), cameraDetecting)
            busy = false
            refusal = outcome.refusal
            if (outcome.message.isNotEmpty()) notice.show(outcome.message, null)
            if (outcome.refusal == null) onUploaded()
        }
    }
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
            FilePick("Model (.hef for the Hailo-8, .onnx for the CPU)", "Model file", hef) {
                hef = it
                refusal = null
            }
            DeviceHint("Trained with Ultralytics? Export it first: yolo export model=best.pt format=onnx — the .onnx carries its class names.")
            RefusalText(refusal, MODEL_FIELD)
        }
        FilePick(
            "Class labels (.labels, one per line — optional). An Ultralytics .onnx brings its own; without labels, boxes are named by class number.",
            "Class labels file",
            labels,
        ) { labels = it }
        OutlinedTextField(
            value = about,
            onValueChange = { about = it },
            label = { Text("What it's for (shown next to the model — optional)") },
            placeholder = { Text("Aerial — vehicles at 400 ft") },
            singleLine = true,
            modifier = Modifier.fillMaxWidth(),
        )
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Checkbox(checked = use, onCheckedChange = { use = it }, modifier = Modifier.semantics { contentDescription = "Switch to this model after upload" })
            Text("Switch to this model after upload", style = MaterialTheme.typography.bodyMedium)
        }
        RefusalText(refusal)
        FilledTonalButton(onClick = { hef?.let(upload) ?: run { refusal = DeviceRefusal("Choose a .hef or .onnx model file.", MODEL_FIELD) } }, enabled = !busy) {
            Text(if (busy) "Uploading…" else "Upload model")
        }
    }
}
