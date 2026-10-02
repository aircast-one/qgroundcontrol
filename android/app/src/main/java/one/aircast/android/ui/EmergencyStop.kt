package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Surface
import androidx.compose.ui.Alignment
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import one.aircast.android.R
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
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

internal const val EMERGENCY_STOP = "emergencyStop"
private const val STOP_SCRIM_ALPHA = 0.45f
private const val DISABLED_STOP_ALPHA = 0.38f

internal fun emergencyStopOffer(offers: Map<String, GuidedOffer>): GuidedOffer? =
    offers[EMERGENCY_STOP]?.takeIf { it.shown }

internal fun armedStopOffer(offers: Map<String, GuidedOffer>, armed: Boolean): GuidedOffer? =
    emergencyStopOffer(offers)?.takeIf { armed && it.ready }

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
        Surface(
            onClick = { onConfirm(emergencyStopAction(offer)) },
            enabled = offer.ready,
            modifier = Modifier.alpha(if (offer.ready) 1f else DISABLED_STOP_ALPHA),
            shape = CircleShape,
            color = Color.Black.copy(alpha = STOP_SCRIM_ALPHA),
            contentColor = MaterialTheme.colorScheme.error,
            border = BorderStroke(2.dp, MaterialTheme.colorScheme.error),
        ) {
            Row(
                Modifier.height(40.dp).padding(start = 10.dp, end = 16.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(painterResource(R.drawable.ic_stop_circle), null, Modifier.size(24.dp))
                Text(offer.title, style = MaterialTheme.typography.labelLarge)
            }
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
