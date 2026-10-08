package one.aircast.android.ui

import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath
import org.json.JSONObject

private const val RESUME_MISSION_PATH = "planFly.missionController.resumeMission"

internal fun resumeFailedIndex(view: JSONObject?): Int? =
    view?.takeIf { it.has("resumeFailedIndex") && !it.isNull("resumeFailedIndex") }?.optInt("resumeFailedIndex")

@Composable
internal fun ResumeFailedPrompt() {
    val actions by qgcPath(GUIDED_ACTIONS)
    val failed = remember(actions) { resumeFailedIndex(actions) }
    var dismissed by remember { mutableStateOf<Int?>(null) }
    LaunchedEffect(failed == null) { if (failed == null) dismissed = null }
    val index = failed?.takeIf { it != dismissed } ?: return
    AlertDialog(
        onDismissRequest = { dismissed = index },
        title = { Text("Resume FAILED") },
        text = { Text("Upload of resume mission failed. Confirm to retry upload") },
        confirmButton = {
            TextButton(onClick = {
                dismissed = index
                offMainDetached { Qgc.invoke(RESUME_MISSION_PATH, index) }
            }) { Text("Confirm") }
        },
        dismissButton = { TextButton(onClick = { dismissed = index }) { Text("Cancel") } },
    )
}
