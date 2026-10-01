package one.aircast.android.ui

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Checkbox
import androidx.compose.material3.MaterialTheme
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import one.aircast.mapspike.aircast
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcPath
import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val PARAMETER_TOOLS_PATH = "view.parameterTools"
internal const val PARAMETER_REFRESH_PATH = "parameterTools.refresh"
internal const val PARAMETER_SAVE_PATH = "parameterFile.save"
internal const val PARAMETER_REVIEW_PATH = "parameterFile.review"
internal const val PARAMETER_APPLY_PATH = "parameterFile.apply"
private const val PARAMETER_FILE_NAME = "vehicle.params"
private val PARAMETER_FILE_TYPES = arrayOf("text/plain", "application/octet-stream", "*/*")

internal data class ParameterDiffRow(
    val json: JSONObject,
    val name: String,
    val fileValue: String,
    val vehicleValue: String,
    val units: String,
    val cannotSend: Boolean,
)

internal data class ParameterReview(
    val rows: List<ParameterDiffRow>,
    val otherVehicle: Boolean,
    val multipleComponents: Boolean,
)

internal fun parameterReview(result: JSONObject?): ParameterReview? = result?.let { review ->
    val listed = review.optJSONArray("rows")
    ParameterReview(
        rows = (0 until (listed?.length() ?: 0)).mapNotNull { index ->
            listed!!.optJSONObject(index)?.let {
                ParameterDiffRow(it, it.optText("name"), it.optText("fileValue"), it.optText("vehicleValue"), it.optText("units"), it.optBoolean("cannotSend"))
            }
        },
        otherVehicle = review.optBoolean("otherVehicle"),
        multipleComponents = review.optBoolean("multipleComponents"),
    )
}

internal fun reviewWarnings(review: ParameterReview): List<String> = listOfNotNull(
    "The parameters in the file are from a different vehicle.".takeIf { review.otherVehicle },
    "The file contains parameters for more than one component.".takeIf { review.multipleComponents },
)

internal data class ParameterTool(
    val path: String,
    val label: String,
    val confirm: Boolean,
    val confirmTitle: String,
    val confirmMessage: String,
)

internal fun parameterTools(view: JSONObject?): List<ParameterTool> {
    val listed = view?.optJSONArray("tools") ?: return emptyList()
    return (0 until listed.length()).mapNotNull { index ->
        listed.optJSONObject(index)?.let {
            ParameterTool(
                path = it.optText("path"),
                label = it.optText("label"),
                confirm = it.optBoolean("confirm"),
                confirmTitle = it.optText("confirmTitle"),
                confirmMessage = it.optText("confirmMessage"),
            )
        }
    }
}

internal fun diffLine(row: ParameterDiffRow): String = when {
    row.cannotSend -> "File ${row.fileValue} · not on vehicle, cannot send"
    else -> listOf("Vehicle ${row.vehicleValue}", "File ${row.fileValue}", row.units).filter { it.isNotBlank() }.joinToString(" · ")
}

internal fun confirmLabel(tool: ParameterTool): String = if (tool.confirmTitle == "Reset All") "Reset" else "Ok"

@Composable
internal fun ParameterToolsMenu(onRefreshed: () -> Unit) {
    val view by qgcPath(PARAMETER_TOOLS_PATH)
    val tools = remember(view) { parameterTools(view) }
    var open by remember { mutableStateOf(false) }
    var asking by remember { mutableStateOf<ParameterTool?>(null) }
    var refusal by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    val context = LocalContext.current
    var review by remember { mutableStateOf<ParameterReview?>(null) }
    var chosen by remember { mutableStateOf<Set<String>>(emptySet()) }

    val saver = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("text/plain")) { uri ->
        val target = uri ?: return@rememberLauncherForActivityResult
        scope.launch {
            refusal = withContext(Dispatchers.IO) {
                val saved = Qgc.invokeResult(PARAMETER_SAVE_PATH) as? String ?: return@withContext "The parameters could not be read from the vehicle."
                runCatching { context.contentResolver.openOutputStream(target, "wt")?.use { it.write(saved.toByteArray()) } }
                    .fold({ null }, { "The file could not be written." })
            }
        }
    }
    val loader = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        val source = uri ?: return@rememberLauncherForActivityResult
        scope.launch {
            val text = withContext(Dispatchers.IO) {
                runCatching { context.contentResolver.openInputStream(source)?.use { it.readBytes().decodeToString() } }.getOrNull()
            }
            if (text == null) {
                refusal = "Unable to open file."
                return@launch
            }
            val reviewed = withContext(Dispatchers.Default) { Qgc.call(PARAMETER_REVIEW_PATH, text) }
            val parsed = parameterReview(reviewed?.optJSONObject("result"))
            if (parsed == null) {
                refusal = reviewed?.optText("reason")?.ifBlank { null } ?: "The file could not be reviewed."
            } else {
                review = parsed
                chosen = parsed.rows.filterNot { it.cannotSend }.map { it.name }.toSet()
            }
        }
    }

    fun run(tool: ParameterTool) {
        when (tool.path) {
            PARAMETER_SAVE_PATH -> saver.launch(PARAMETER_FILE_NAME)
            PARAMETER_REVIEW_PATH -> loader.launch(PARAMETER_FILE_TYPES)
            else -> scope.launch {
                refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(tool.path) }
                if (refusal == null && tool.path == PARAMETER_REFRESH_PATH) onRefreshed()
            }
        }
    }

    if (tools.isEmpty()) return
    Box {
        TextButton(onClick = { open = true }) { Text("Tools") }
        DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            tools.forEach { tool ->
                DropdownMenuItem(
                    text = { Text(tool.label) },
                    onClick = {
                        open = false
                        if (tool.confirm) asking = tool else run(tool)
                    },
                )
            }
        }
    }
    asking?.let { tool ->
        AlertDialog(
            onDismissRequest = { asking = null },
            title = { Text(tool.confirmTitle) },
            text = { Text(tool.confirmMessage) },
            confirmButton = {
                TextButton(onClick = {
                    asking = null
                    run(tool)
                }) { Text(confirmLabel(tool)) }
            },
            dismissButton = { TextButton(onClick = { asking = null }) { Text("Cancel") } },
        )
    }
    review?.let { shown ->
        AlertDialog(
            onDismissRequest = { review = null },
            title = { Text("Load Parameters") },
            text = {
                Column {
                    reviewWarnings(shown).forEach { Text(it, color = MaterialTheme.aircast.warning) }
                    if (shown.rows.isEmpty()) Text("No differences between vehicle and file.")
                    LazyColumn(Modifier.heightIn(max = 360.dp)) {
                        items(shown.rows, key = { it.name }) { row ->
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Checkbox(
                                    checked = row.name in chosen,
                                    enabled = !row.cannotSend,
                                    onCheckedChange = { on -> chosen = if (on) chosen + row.name else chosen - row.name },
                                )
                                Column(Modifier.weight(1f)) {
                                    Text(row.name, style = MaterialTheme.typography.bodyMedium)
                                    Text(
                                        diffLine(row),
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                }
                            }
                        }
                    }
                }
            },
            confirmButton = {
                TextButton(
                    enabled = chosen.isNotEmpty(),
                    onClick = {
                        val rows = shown.rows.filter { it.name in chosen }.map { it.json }
                        review = null
                        scope.launch {
                            refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(PARAMETER_APPLY_PATH, org.json.JSONArray(rows)) }
                            onRefreshed()
                        }
                    },
                ) { Text("Send to vehicle") }
            },
            dismissButton = { TextButton(onClick = { review = null }) { Text("Cancel") } },
        )
    }
    refusal?.let {
        AlertDialog(
            onDismissRequest = { refusal = null },
            title = { Text("Parameters") },
            text = { Text(it) },
            confirmButton = { TextButton(onClick = { refusal = null }) { Text("Close") } },
        )
    }
}
