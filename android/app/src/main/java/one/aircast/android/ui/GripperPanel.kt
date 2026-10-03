package one.aircast.android.ui

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

internal const val HOLD_TO_CONFIRM_MS = 500
internal const val HOLD_HELP_MS = 3000L
internal const val HOLD_TO_CONFIRM_HELP = "Hold to Confirm"
private const val DISABLED_HOLD_ALPHA = 0.38f
private const val HOLD_FILL_ALPHA = 0.35f

internal val GRIPPER_ACTIONS = listOf("release", "grab", "hold")

internal fun gripperOffers(extras: List<GuidedOffer>): List<GuidedOffer> =
    GRIPPER_ACTIONS.mapNotNull { id -> extras.firstOrNull { it.id == id } }

@Composable
internal fun HoldToConfirmButton(text: String, enabled: Boolean, modifier: Modifier = Modifier, onActivated: () -> Unit) {
    val progress = remember { Animatable(0f) }
    var showHelp by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    val activate by rememberUpdatedState(onActivated)

    LaunchedEffect(showHelp) {
        if (showHelp) {
            delay(HOLD_HELP_MS)
            showHelp = false
        }
    }

    Box(
        modifier
            .fillMaxWidth()
            .height(56.dp)
            .alpha(if (enabled) 1f else DISABLED_HOLD_ALPHA)
            .clip(RoundedCornerShape(12.dp))
            .background(MaterialTheme.colorScheme.secondaryContainer)
            .pointerInput(enabled) {
                if (enabled) {
                    detectTapGestures(onPress = {
                        showHelp = false
                        val fill = scope.launch {
                            progress.animateTo(1f, tween(HOLD_TO_CONFIRM_MS, easing = LinearEasing))
                            activate()
                        }
                        tryAwaitRelease()
                        val fired = fill.isCompleted && !fill.isCancelled
                        fill.cancel()
                        progress.snapTo(0f)
                        showHelp = !fired
                    })
                }
            },
        contentAlignment = Alignment.Center,
    ) {
        Box(
            Modifier
                .align(Alignment.CenterStart)
                .fillMaxHeight()
                .fillMaxWidth(progress.value)
                .background(MaterialTheme.colorScheme.primary.copy(alpha = HOLD_FILL_ALPHA)),
        )
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Text(text, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSecondaryContainer)
            if (showHelp) {
                Text(HOLD_TO_CONFIRM_HELP, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSecondaryContainer)
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun GripperPanel(offers: List<GuidedOffer>, onDismiss: () -> Unit) {
    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(
            Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(bottom = 24.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            offers.forEach { offer ->
                HoldToConfirmButton(offer.title, offer.ready) {
                    guidedCommand(offer.id, null)?.invoke()
                    onDismiss()
                }
            }
        }
    }
}
