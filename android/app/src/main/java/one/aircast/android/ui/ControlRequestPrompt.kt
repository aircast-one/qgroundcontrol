package one.aircast.android.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import org.json.JSONObject
import kotlin.math.ceil

private const val CONTROL_PROMPT_POLL_MS = 500L
private const val REVERT_ALLOW_TAKEOVER = "vehicle.startTimerRevertAllowTakeover"

internal data class IncomingControlRequest(val systemId: Int, val timeoutMs: Long, val remainingMs: Long)

internal data class ControlPrompt(val incoming: IncomingControlRequest?, val revertMs: Long?)

internal fun controlPrompt(view: JSONObject?): ControlPrompt =
    ControlPrompt(
        incoming = view?.optJSONObject("incomingRequest")?.let {
            IncomingControlRequest(it.optInt("systemId"), it.optLong("timeoutMs"), it.optLong("remainingMs"))
        },
        revertMs = view?.takeIf { it.has("takeoverRevertMs") && !it.isNull("takeoverRevertMs") && it.optBoolean("inControl") }?.optLong("takeoverRevertMs"),
    )

internal fun secondsLeft(ms: Long): Int = ceil(ms / 1000.0).toInt()

@Composable
internal fun ControlRequestPrompt() {
    var prompt by remember { mutableStateOf(ControlPrompt(null, null)) }
    var ignored by remember { mutableStateOf<Int?>(null) }
    var revertIgnored by remember { mutableStateOf(false) }
    var allowedFrom by remember { mutableStateOf<Int?>(null) }
    LaunchedEffect(Unit) {
        while (isActive) {
            prompt = withContext(Dispatchers.Default) { controlPrompt(Qgc.get(OPERATOR_CONTROL_VIEW)) }
            if (prompt.incoming == null) ignored = null
            if (prompt.revertMs == null) revertIgnored = false
            delay(CONTROL_PROMPT_POLL_MS)
        }
    }

    val incoming = prompt.incoming?.takeIf { it.systemId != ignored }
    if (incoming != null) {
        AlertDialog(
            onDismissRequest = { ignored = incoming.systemId },
            title = { Text("GCS ${incoming.systemId} is requesting control") },
            text = {
                Column {
                    Text("Ignoring automatically in ${secondsLeft(incoming.remainingMs)} seconds")
                    LinearProgressIndicator(
                        progress = { if (incoming.timeoutMs > 0) incoming.remainingMs.toFloat() / incoming.timeoutMs else 0f },
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
            },
            confirmButton = {
                TextButton(onClick = {
                    allowedFrom = incoming.systemId
                    revertIgnored = false
                    offMainDetached {
                        Qgc.invoke(REQUEST_CONTROL, true, 0)
                        Qgc.invoke(REVERT_ALLOW_TAKEOVER)
                    }
                }) { Text("Allow takeover") }
            },
            dismissButton = { TextButton(onClick = { ignored = incoming.systemId }) { Text("Ignore") } },
        )
        return
    }

    val revert = prompt.revertMs?.takeIf { !revertIgnored }
    if (revert != null) {
        AlertDialog(
            onDismissRequest = { revertIgnored = true },
            text = {
                Text("Reverting back to takeover not allowed if GCS ${allowedFrom ?: ""} doesn't take control in ${secondsLeft(revert)} seconds ...")
            },
            confirmButton = { TextButton(onClick = { revertIgnored = true }) { Text("Ignore") } },
        )
    }
}
