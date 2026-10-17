package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import one.aircast.android.bridge.VehicleCommands
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath
import one.aircast.map.AircastSheet
import org.json.JSONObject

internal val GRIPPER_ACTIONS = listOf(RELEASE, "grab", "hold")
internal const val PAYLOAD_STATE = "vehicle.gripper"
private const val STATUS_TICK_MS = 1000L

internal fun gripperOffers(extras: List<GuidedOffer>): List<GuidedOffer> =
    GRIPPER_ACTIONS.mapNotNull { id -> extras.firstOrNull { it.id == id } }

internal data class PayloadStatus(val text: String, val warning: Boolean)

internal fun agoText(atMs: Long, nowMs: Long): String {
    val seconds = ((nowMs - atMs) / 1000).coerceAtLeast(0)
    if (seconds < 60) return "$seconds s ago"
    val minutes = seconds / 60
    return if (minutes < 60) "$minutes min ago" else "${minutes / 60} h ago"
}

private fun verbs(action: String): Pair<String, String> = when (action) {
    RELEASE -> "Drop" to "Dropped"
    "grab" -> "Grab" to "Grabbed"
    else -> "Hold" to "Holding"
}

internal fun payloadStatus(json: JSONObject?, nowMs: Long): PayloadStatus? {
    val state = json ?: return null
    val action = state.optString("action")
    val bay = state.optInt("bay")
    val (plainVerb, plainDone) = verbs(action)
    val verb = if (bay > 0) "$plainVerb $bay" else plainVerb
    val done = if (bay > 0) "Bay $bay ${plainDone.lowercase()}" else plainDone
    val ago = agoText(state.optLong("atMs"), nowMs)
    return when (state.optString("outcome")) {
        "sent" -> PayloadStatus("$verb sent, waiting for the aircraft", warning = false)
        "confirmed" -> PayloadStatus("$done $ago", warning = false)
        "refused" -> PayloadStatus("$verb refused by the flight controller $ago", warning = true)
        "unanswered" -> PayloadStatus(
            if (action == RELEASE) "$verb not confirmed $ago. It may have dropped, check before dropping again." else "$verb not confirmed $ago. Check the gripper before trying again.",
            warning = true,
        )
        else -> null
    }
}

internal fun bayLabel(bay: Int, payload: Payload): String =
    if (bay in payload.empty) "Bay $bay empty, slide to drop" else "Slide to drop bay $bay"

private fun List<GuidedOffer>.ready(id: String): Boolean = any { it.id == id && it.ready }

@Composable
private fun SingleGripper(offers: List<GuidedOffer>) {
    offers.filter { it.id == RELEASE }.forEach { offer ->
        HoldToConfirm(holdLabel(offer.title), enabled = offer.ready) { guidedCommand(offer.id, null)?.invoke() }
    }
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        offers.filter { it.id != RELEASE }.forEach { offer ->
            OutlinedButton(onClick = { guidedCommand(offer.id, null)?.invoke() }, enabled = offer.ready) { Text(offer.title) }
        }
    }
}

@Composable
private fun Bays(offers: List<GuidedOffer>, payload: Payload) {
    payload.bayNumbers.forEach { bay ->
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            HoldToConfirm(bayLabel(bay, payload), Modifier.weight(1f), enabled = offers.ready(RELEASE)) {
                offMainDetached { VehicleCommands.gripper(GRIPPER_RELEASE, bay) }
            }
            OutlinedButton(onClick = { offMainDetached { VehicleCommands.gripper(GRIPPER_GRAB, bay) } }, enabled = offers.ready("grab")) { Text("Grab") }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun GripperPanel(offers: List<GuidedOffer>, payload: Payload, onDismiss: () -> Unit) {
    val stateJson by qgcPath(PAYLOAD_STATE)
    var now by remember { mutableLongStateOf(System.currentTimeMillis()) }
    LaunchedEffect(Unit) {
        while (true) {
            delay(STATUS_TICK_MS)
            now = System.currentTimeMillis()
        }
    }
    val status = payloadStatus(stateJson?.optJSONObject("value"), now)
    AircastSheet(onDismissRequest = onDismiss) {
        Column(
            Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(bottom = 24.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text(
                status?.text ?: "No payload command sent yet",
                style = MaterialTheme.typography.bodyMedium,
                color = if (status?.warning == true) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant,
            )
            if (payload.multiBay) Bays(offers, payload) else SingleGripper(offers)
        }
    }
}
