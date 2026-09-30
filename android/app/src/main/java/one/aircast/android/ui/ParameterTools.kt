package one.aircast.android.ui

import androidx.compose.foundation.layout.Box
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

internal fun confirmLabel(tool: ParameterTool): String = if (tool.confirmTitle == "Reset All") "Reset" else "Ok"

@Composable
internal fun ParameterToolsMenu(onRefreshed: () -> Unit) {
    val view by qgcPath(PARAMETER_TOOLS_PATH)
    val tools = remember(view) { parameterTools(view) }
    var open by remember { mutableStateOf(false) }
    var asking by remember { mutableStateOf<ParameterTool?>(null) }
    var refusal by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    fun run(tool: ParameterTool) {
        scope.launch {
            refusal = withContext(Dispatchers.Default) { Qgc.refusalOf(tool.path) }
            if (refusal == null && tool.path == PARAMETER_REFRESH_PATH) onRefreshed()
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
    refusal?.let {
        AlertDialog(
            onDismissRequest = { refusal = null },
            title = { Text("Parameters") },
            text = { Text(it) },
            confirmButton = { TextButton(onClick = { refusal = null }) { Text("Close") } },
        )
    }
}
