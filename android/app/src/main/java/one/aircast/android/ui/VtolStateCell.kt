package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import one.aircast.android.bridge.qgcPath
import org.json.JSONObject

internal data class VtolState(val forward: Boolean, val flying: Boolean) {
    val label: String get() = if (forward) "FW(vtol)" else "MR(vtol)"
    val transition: String get() = if (forward) "vtolTransitionToMultiRotor" else "vtolTransitionToFixedWing"
}

internal fun vtolState(view: JSONObject?): VtolState? =
    view?.takeIf { it.optBoolean("vtol") }?.let { VtolState(it.optBoolean("vtolInFwdFlight"), it.optBoolean("flying")) }

@Composable
internal fun VtolStateCell() {
    val view by qgcPath(GUIDED_ACTIONS)
    val state = remember(view) { vtolState(view) } ?: return
    val offer = remember(view, state) { guidedOffers(view)[state.transition] }
    var confirming by remember { mutableStateOf(false) }
    Text(
        state.label,
        style = if (state.flying) MaterialTheme.typography.titleMedium else MaterialTheme.typography.labelLarge,
        modifier = Modifier.clickable(enabled = state.flying) { confirming = true },
    )
    if (confirming) {
        val command = guidedCommand(state.transition, null)
        AlertDialog(
            onDismissRequest = { confirming = false },
            title = { Text(offer?.title?.ifBlank { null } ?: if (state.forward) "Transition to Multi-Rotor" else "Transition to Fixed Wing") },
            text = { offer?.let { Text(blockedReasonFor(it) ?: it.prompt) } },
            confirmButton = {
                TextButton(enabled = offer?.ready == true && command != null, onClick = {
                    confirming = false
                    command?.invoke()
                }) { Text(if (state.forward) "Transition to Multi-Rotor" else "Transition to Fixed Wing") }
            },
            dismissButton = { TextButton(onClick = { confirming = false }) { Text("Cancel") } },
        )
    }
}
