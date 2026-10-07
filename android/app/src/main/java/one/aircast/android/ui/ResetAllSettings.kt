package one.aircast.android.ui

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc

internal const val GENERAL_PAGE = "General"
internal const val VIDEO_PAGE = "Video"
internal const val CLEAR_SETTINGS_NEXT_BOOT = "settings.appSettings.clearSettingsNextBoot"

internal fun resetPending(value: Any?): Boolean = value == true || value == 1 || value == "true"

@Composable
internal fun ResetAllSettingsRow() {
    val scope = rememberCoroutineScope()
    var reads by remember { mutableIntStateOf(0) }
    var pending by remember { mutableStateOf(false) }
    var asking by remember { mutableStateOf(false) }

    LaunchedEffect(reads) {
        pending = withContext(Dispatchers.Default) { resetPending(Qgc.get(CLEAR_SETTINGS_NEXT_BOOT)?.opt("value")) }
    }

    fun write(value: Boolean) {
        scope.launch {
            withContext(Dispatchers.Default) { Qgc.set(CLEAR_SETTINGS_NEXT_BOOT, value) }
            reads++
        }
    }

    Row(Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 12.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(if (pending) "All settings will be cleared on next start" else "", style = MaterialTheme.typography.bodySmall, modifier = Modifier.weight(1f))
        OutlinedButton(onClick = { if (pending) write(false) else asking = true }) {
            Text(if (pending) "Cancel reset" else "Reset all settings…")
        }
    }

    if (asking) {
        AlertDialog(
            onDismissRequest = { asking = false },
            title = { Text("Reset all settings") },
            text = { Text("All settings will be cleared the next time Aircast starts. This cannot be undone.") },
            confirmButton = { TextButton(onClick = { asking = false; write(true) }) { Text("Ok") } },
            dismissButton = { TextButton(onClick = { asking = false }) { Text("Cancel") } },
        )
    }
}
