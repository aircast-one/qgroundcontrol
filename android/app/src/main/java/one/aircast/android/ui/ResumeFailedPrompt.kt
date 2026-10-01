package one.aircast.android.ui

import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.withContext
import org.json.JSONObject

private const val RESUME_MISSION_PATH = "planFly.missionController.resumeMission"
private const val RESUME_POLL_MS = 1000L

internal fun resumeFailedIndex(view: JSONObject?): Int? =
    view?.takeIf { it.has("resumeFailedIndex") && !it.isNull("resumeFailedIndex") }?.optInt("resumeFailedIndex")

@Composable
internal fun ResumeFailedPrompt() {
    var failed by remember { mutableStateOf<Int?>(null) }
    var dismissed by remember { mutableStateOf<Int?>(null) }
    LaunchedEffect(Unit) {
        while (isActive) {
            failed = withContext(Dispatchers.Default) { resumeFailedIndex(Qgc.get(GUIDED_ACTIONS)) }
            if (failed == null) dismissed = null
            delay(RESUME_POLL_MS)
        }
    }
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
