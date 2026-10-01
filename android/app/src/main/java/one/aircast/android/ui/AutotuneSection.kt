package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath
import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val AUTOTUNE_VIEW = "view.autotune"
internal const val AUTOTUNE_REQUEST = "vehicle.autotune.autotuneRequest"

internal data class AutotuneState(val canStart: Boolean, val status: String, val progress: Float, val warning: String)

internal fun autotuneState(view: JSONObject?): AutotuneState? =
    view?.takeIf { it.optBoolean("available") }?.let {
        AutotuneState(it.optBoolean("canStart"), it.optText("status"), it.optDouble("progress", 0.0).toFloat(), it.optText("warning"))
    }

@Composable
fun AutotuneSection() {
    val view by qgcPath(AUTOTUNE_VIEW)
    val state = autotuneState(view) ?: return
    var confirming by remember { mutableStateOf(false) }
    if (confirming) {
        AlertDialog(
            onDismissRequest = { confirming = false },
            title = { Text("Start AutoTune") },
            text = { Text(state.warning) },
            confirmButton = { TextButton(onClick = { confirming = false; offMainDetached { Qgc.invoke(AUTOTUNE_REQUEST) } }) { Text("Ok") } },
            dismissButton = { TextButton(onClick = { confirming = false }) { Text("Cancel") } },
        )
    }
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Button(onClick = { confirming = true }, enabled = state.canStart) { Text("Start AutoTune") }
        Text(state.status)
        LinearProgressIndicator(progress = { state.progress }, modifier = Modifier.fillMaxWidth())
    }
}
