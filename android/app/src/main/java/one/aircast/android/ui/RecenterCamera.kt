package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.LocalMinimumInteractiveComponentSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import one.aircast.android.R
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.offMainInOrder
import one.aircast.map.aircast
import kotlin.math.abs
import kotlin.math.roundToInt

private const val GIMBAL_CENTER = "gimbal.center"
private const val OFF_CENTRE_DEG = 2.0
private const val RECENTER_SCRIM_ALPHA = 0.55f
private val TILT_READOUT_WIDTH = 64.dp
private val TILT_READOUT_HEIGHT = 44.dp
private val RECENTER_GLYPH = 14.dp
private val TILT_READOUT_PADDING = 8.dp

internal enum class RecenterTarget { Synthetic, Gimbal }

private fun offCentre(vararg degrees: Double?): Boolean = degrees.any { it != null && abs(it) > OFF_CENTRE_DEG }

internal fun recenterTarget(syntheticAims: Boolean, syntheticTilt: Double?, syntheticPan: Double?, gimbal: GimbalIndicatorState?): RecenterTarget? = when {
    syntheticAims -> RecenterTarget.Synthetic.takeIf { offCentre(syntheticTilt?.minus(DEFAULT_TILT_DEG), syntheticPan) }
    gimbal != null -> RecenterTarget.Gimbal.takeIf { offCentre(gimbal.pitchDegrees, gimbal.yawDegrees) }
    else -> null
}

internal fun recenter(target: RecenterTarget, onRefused: (String?) -> Unit) = when (target) {
    RecenterTarget.Synthetic -> offMainInOrder { Qgc.invoke(SYNTHETIC_AIM, DEFAULT_TILT_DEG, 0.0) }
    RecenterTarget.Gimbal -> offMainDetached { onRefused(gimbalRefusal(Qgc.call(GIMBAL_CENTER))) }
}

internal fun tiltText(degrees: Double): String = "${degrees.roundToInt()}°"

@Composable
internal fun TiltReadout(degrees: Double, onRecenter: (() -> Unit)?, onTap: (() -> Unit)? = null, tapLabel: String? = null) {
    val text = tiltText(degrees)
    val action = onRecenter ?: onTap
    val look = Modifier
        .widthIn(min = TILT_READOUT_WIDTH)
        .height(TILT_READOUT_HEIGHT)
        .semantics {
            contentDescription = if (onRecenter != null) "Tilt $text, off centre" else "Tilt $text"
            if (action != null) onClick(label = if (onRecenter != null) "Recenter" else tapLabel) { action(); true }
        }
    val color = if (onRecenter != null) Color.Black.copy(alpha = RECENTER_SCRIM_ALPHA) else Color.Transparent
    val content: @Composable () -> Unit = {
        Row(Modifier.padding(horizontal = TILT_READOUT_PADDING), horizontalArrangement = Arrangement.spacedBy(4.dp, Alignment.CenterHorizontally), verticalAlignment = Alignment.CenterVertically) {
            if (onRecenter != null) Icon(painterResource(R.drawable.ic_my_location), null, Modifier.size(RECENTER_GLYPH))
            Text("Tilt $text", style = MaterialTheme.typography.labelMedium, modifier = if (onRecenter == null) Modifier.osdShadow() else Modifier)
        }
    }
    val white = MaterialTheme.aircast.outdoorForeground
    CompositionLocalProvider(LocalMinimumInteractiveComponentSize provides Dp.Unspecified) {
        if (action != null) {
            Surface(onClick = action, shape = CircleShape, color = color, contentColor = white, modifier = look, content = content)
        } else {
            Surface(shape = CircleShape, color = color, contentColor = white, modifier = look, content = content)
        }
    }
}
