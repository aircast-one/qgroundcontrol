package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.qgcPath

internal const val EMERGENCY_STOP = "emergencyStop"

internal fun emergencyStopOffer(offers: Map<String, GuidedOffer>): GuidedOffer? =
    offers[EMERGENCY_STOP]?.takeIf { it.shown }

internal fun emergencyStopAction(offer: GuidedOffer): GuidedAction = GuidedAction(
    name = offer.title,
    confirm = offer.prompt,
    destructive = true,
    run = { offMainDetached { Qgc.invoke("vehicle.$EMERGENCY_STOP") } },
)

@Composable
internal fun EmergencyStopButton(
    offer: GuidedOffer?,
    onConfirm: (GuidedAction) -> Unit,
    modifier: Modifier = Modifier,
) {
    if (offer == null) {
        return
    }
    Column(modifier, verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Button(
            onClick = { onConfirm(emergencyStopAction(offer)) },
            enabled = offer.ready,
            colors = ButtonDefaults.buttonColors(
                containerColor = MaterialTheme.colorScheme.error,
                contentColor = MaterialTheme.colorScheme.onError,
            ),
        ) {
            Text(offer.title, fontWeight = FontWeight.Bold)
        }
        blockedReasonFor(offer)?.let { reason ->
            Text(
                text = reason,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
internal fun PinnedEmergencyStop(modifier: Modifier = Modifier) {
    val actionsJson by qgcPath(GUIDED_ACTIONS)
    val offers = remember(actionsJson) { guidedOffers(actionsJson) }
    var pending by remember { mutableStateOf<GuidedAction?>(null) }
    val confirming = pending

    if (confirming != null) {
        ConfirmTrack(
            action = confirming,
            onSent = { pending = null },
            onCancel = { pending = null },
            modifier = modifier,
        )
    } else {
        EmergencyStopButton(
            offer = emergencyStopOffer(offers),
            onConfirm = { action -> pending = action },
            modifier = modifier,
        )
    }
}
