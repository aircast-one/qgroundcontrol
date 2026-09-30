package one.aircast.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.withTimeoutOrNull
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath
import one.aircast.mapspike.aircast
import one.aircast.mapspike.optText
import org.json.JSONObject

internal const val REMOTE_ID_STATUS_PATH = "view.remoteIdStatus"

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

    if (open) {
        ModalBottomSheet(onDismissRequest = { open = false }) {
            Column(
                Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(bottom = 24.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Text("RemoteID Status", style = MaterialTheme.typography.titleMedium)
                remoteIdRows(status).forEach { (label, good) ->
                    Text(label, style = MaterialTheme.typography.labelLarge, color = if (good) MaterialTheme.aircast.success else MaterialTheme.colorScheme.error)
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
            }
        }
    }
}
