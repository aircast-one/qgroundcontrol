package one.aircast.android.ui

import android.app.Activity
import androidx.activity.compose.BackHandler
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.R
import one.aircast.android.bridge.Qgc
import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val CLOSE_CHECKS = "view.closeChecks"

internal fun closePrompts(view: JSONObject?): List<String> {
    val listed = view?.optJSONArray("prompts") ?: return emptyList()
    return (0 until listed.length()).mapNotNull { listed.optJSONObject(it)?.optText("message")?.takeIf(String::isNotBlank) }
}

@Composable
internal fun CloseGuard(enabled: Boolean) {
    val activity = LocalContext.current as? Activity
    val scope = rememberCoroutineScope()
    var pending by remember { mutableStateOf<List<String>?>(null) }
    fun advance(remaining: List<String>) {
        if (remaining.isEmpty()) {
            pending = null
            activity?.finish()
        } else {
            pending = remaining
        }
    }
    BackHandler(enabled = enabled && pending == null) {
        scope.launch { advance(withContext(Dispatchers.Default) { closePrompts(Qgc.get(CLOSE_CHECKS)) }) }
    }
    val shown = pending ?: return
    val title = "Close ${activity?.getString(R.string.app_name).orEmpty()}"
    AlertDialog(
        onDismissRequest = { pending = null },
        title = { Text(title) },
        text = { Text(shown.first()) },
        confirmButton = { TextButton(onClick = { advance(shown.drop(1)) }) { Text("Yes") } },
        dismissButton = { TextButton(onClick = { pending = null }) { Text("No") } },
    )
}
