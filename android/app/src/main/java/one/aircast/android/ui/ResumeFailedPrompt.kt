package one.aircast.android.ui

import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath
import org.json.JSONObject

private const val RESUME_MISSION_PATH = "planFly.missionController.resumeMission"

internal fun resumeFailedIndex(view: JSONObject?): Int? =
    view?.takeIf { it.has("resumeFailedIndex") && !it.isNull("resumeFailedIndex") }?.optInt("resumeFailedIndex")

internal fun resumeCleared(view: JSONObject?): Boolean = view != null && resumeFailedIndex(view) == null

@Composable
internal fun ResumeFailedPrompt(dismissed: Int?, onDismissed: (Int?) -> Unit) {
    val actions by qgcPath(GUIDED_ACTIONS)
    val failed = remember(actions) { resumeFailedIndex(actions) }
    val cleared = resumeCleared(actions)
    LaunchedEffect(cleared) { if (cleared) onDismissed(null) }
    val index = failed?.takeIf { it != dismissed } ?: return
    AlertDialog(
        onDismissRequest = { onDismissed(index) },
        title = { Text("Resume FAILED") },
        text = { Text("Upload of resume mission failed. Confirm to retry upload") },
        confirmButton = {
            TextButton(onClick = {
                onDismissed(index)
                offMainDetached { Qgc.invoke(RESUME_MISSION_PATH, index) }
            }) { Text("Confirm") }
        },
        dismissButton = { TextButton(onClick = { onDismissed(index) }) { Text("Cancel") } },
    )
}
