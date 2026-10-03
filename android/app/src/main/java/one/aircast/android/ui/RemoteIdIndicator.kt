package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import one.aircast.android.bridge.Fact
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath
import one.aircast.mapspike.aircast
import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val REMOTE_ID_STATUS_PATH = "view.remoteIdStatus"
internal const val REMOTE_ID_SETTINGS_PAGE = "Remote ID"
private const val SEND_SELF_ID = "sendSelfID"
private val SELF_ID_FACTS = listOf(SEND_SELF_ID, "selfIDType", "selfIDFree", "selfIDExtended", "selfIDEmergency")
private val BROADCAST_GATED = setOf("selfIDType", "selfIDFree", "selfIDExtended")
private val SELF_ID_LABELS = mapOf(SEND_SELF_ID to "Broadcast", "selfIDType" to "Broadcast message")
private const val SELF_ID_NOTE = "If an emergency is declared, Emergency Text will be broadcast even if Broadcast setting is not enabled."

internal fun selfIdFacts(page: List<Fact>): List<Fact> {
    val byName = page.associateBy { it.name }
    val broadcasting = byName[SEND_SELF_ID]?.boolValue == true
    return SELF_ID_FACTS.mapNotNull { byName[it] }
        .map { fact -> SELF_ID_LABELS[fact.name]?.let { fact.copy(description = it) } ?: fact }
        .map { fact -> if (fact.name in BROADCAST_GATED && !broadcasting) fact.copy(enabled = false, disabledReason = "Broadcast is off") else fact }
}

internal data class RemoteIdStatus(
    val state: String,
    val comms: Boolean,
    val arm: Boolean,
    val armError: String,
    val gps: Boolean,
    val basicId: Boolean,
    val operatorIdShown: Boolean,
    val operatorId: Boolean,
    val emergency: Boolean,
    val holdMs: Long,
)

internal fun remoteIdStatus(view: JSONObject?): RemoteIdStatus? =
    view?.takeIf { it.optBoolean("shown") }?.let {
        RemoteIdStatus(
            state = it.optText("state"),
            comms = it.optBoolean("comms"),
            arm = it.optBoolean("armStatus"),
            armError = it.optText("armStatusError"),
            gps = it.optBoolean("gcsGps"),
            basicId = it.optBoolean("basicId"),
            operatorIdShown = it.optBoolean("operatorIdShown"),
            operatorId = it.optBoolean("operatorId"),
            emergency = it.optBoolean("emergency"),
            holdMs = it.optLong("emergencyHoldMs", 800),
        )
    }

internal fun remoteIdRows(status: RemoteIdStatus): List<Pair<String, Boolean>> =
    listOfNotNull(
        (if (status.comms) "RID COMMS" else "NOT CONNECTED") to status.comms,
        ("ARM STATUS" to status.arm).takeIf { status.comms },
        ("GCS GPS" to status.gps).takeIf { status.comms },
        ("BASIC ID" to status.basicId).takeIf { status.comms },
        ("OPERATOR ID" to status.operatorId).takeIf { status.comms && status.operatorIdShown },
    )

@Composable
private fun stateColour(state: String): Color = when (state) {
    "healthy" -> MaterialTheme.aircast.success
    "warning" -> MaterialTheme.aircast.warning
    "error" -> MaterialTheme.colorScheme.error
    else -> MaterialTheme.colorScheme.onSurfaceVariant
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun RemoteIdIndicatorCell() {
    val view by qgcPath(REMOTE_ID_STATUS_PATH)
    val status = remember(view) { remoteIdStatus(view) } ?: return
    var open by remember { mutableStateOf(false) }

    Text(
        "RID",
        style = MaterialTheme.typography.labelMedium,
        color = stateColour(status.state),
        maxLines = 1,
        modifier = Modifier.clickable { open = true },
    )

    val configure = {
        AppNavigation.settingsPage = REMOTE_ID_SETTINGS_PAGE
        open = false
    }

    if (open) {
        ModalBottomSheet(onDismissRequest = { open = false }) {
            Column(
                Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = 20.dp).padding(bottom = 24.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Text("RemoteID Status", style = MaterialTheme.typography.titleMedium)
                remoteIdRows(status).forEach { (label, good) ->
                    Text(
                        label,
                        style = MaterialTheme.typography.labelLarge,
                        color = if (good) MaterialTheme.aircast.success else MaterialTheme.colorScheme.error,
                        modifier = Modifier.clickable(onClick = configure),
                    )
                }
                if (status.armError.isNotBlank()) {
                    Text("Arm Status Error  ${status.armError}", style = MaterialTheme.typography.bodySmall)
                }
                if (status.comms) {
                    Text(
                        if (status.emergency) {
                            "EMERGENCY HAS BEEN DECLARED, Press and Hold for 3 seconds to cancel"
                        } else {
                            "Press and Hold below button to declare emergency"
                        },
                        style = MaterialTheme.typography.bodySmall,
                    )
                    Surface(
                        color = MaterialTheme.colorScheme.errorContainer,
                        shape = MaterialTheme.shapes.medium,
                        modifier = Modifier.fillMaxWidth().pointerInput(status.emergency, status.holdMs) {
                            awaitEachGesture {
                                awaitFirstDown()
                                val released = withTimeoutOrNull(status.holdMs) { waitForUpOrCancellation() }
                                if (released == null) {
                                    val declare = !status.emergency
                                    offMainDetached { Qgc.invoke("vehicle.remoteIDManager.setEmergency", declare) }
                                }
                            }
                        },
                    ) {
                        Box(Modifier.padding(16.dp), contentAlignment = Alignment.Center) {
                            Text(
                                if (status.emergency) "Clear Emergency" else "EMERGENCY",
                                color = MaterialTheme.colorScheme.onErrorContainer,
                                style = MaterialTheme.typography.titleMedium,
                            )
                        }
                    }
                }
                SelfIdSection()
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text("Remote ID", modifier = Modifier.weight(1f))
                    OutlinedButton(onClick = configure) { Text("Configure") }
                }
            }
        }
    }
}

@Composable
private fun SelfIdSection() {
    var reloads by remember { mutableIntStateOf(0) }
    var facts by remember { mutableStateOf(emptyList<Fact>()) }
    LaunchedEffect(reloads) {
        facts = withContext(Dispatchers.Default) {
            selfIdFacts(settingsSections(Qgc.get(settingsPagePath(REMOTE_ID_SETTINGS_PAGE))).flatMap { section -> section.blocks.flatMap { it.facts } })
        }
    }
    if (facts.isEmpty()) return
    Text("Self ID", style = MaterialTheme.typography.titleSmall)
    if (facts.firstOrNull { it.name == SEND_SELF_ID }?.boolValue != true) Text(SELF_ID_NOTE, style = MaterialTheme.typography.bodySmall)
    facts.forEach { FactRow(it) { reloads++ } }
}
