package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import one.aircast.android.R
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
import one.aircast.android.bridge.offMainInOrder
import one.aircast.android.bridge.qgcPath
import one.aircast.map.AircastSpace
import one.aircast.map.aircast
import kotlin.math.abs

private const val GIMBAL_CENTER = "gimbal.center"
private const val OFF_CENTRE_DEG = 2.0
private const val RECENTER_SCRIM_ALPHA = 0.55f
private val RECENTER_HEIGHT = 36.dp

internal enum class RecenterTarget { Synthetic, Gimbal }

private fun offCentre(vararg degrees: Double?): Boolean = degrees.any { it != null && abs(it) > OFF_CENTRE_DEG }

internal fun recenterTarget(syntheticAims: Boolean, syntheticPitch: Double?, syntheticPan: Double?, gimbal: GimbalIndicatorState?): RecenterTarget? = when {
    syntheticAims -> RecenterTarget.Synthetic.takeIf { offCentre(syntheticPitch?.minus(DEFAULT_TILT_DEG), syntheticPan) }
    gimbal != null -> RecenterTarget.Gimbal.takeIf { offCentre(gimbal.pitchDegrees, gimbal.yawDegrees) }
    else -> null
}

@Composable
internal fun RecenterCamera() {
    val synthetic by qgcPath(SYNTHETIC_VIEW)
    val gimbalJson by qgcPath(GIMBAL_INDICATOR_PATH)
    val gimbal = remember(gimbalJson) { gimbalIndicator(gimbalJson) }
    val aims = rememberSyntheticAims()
    val target = recenterTarget(aims, synthetic?.optDouble("pitch")?.takeIf(Double::isFinite), synthetic?.optDouble("pan")?.takeIf(Double::isFinite), gimbal) ?: return
    Surface(
        onClick = {
            when (target) {
                RecenterTarget.Synthetic -> offMainInOrder { Qgc.invoke(SYNTHETIC_AIM, DEFAULT_TILT_DEG, 0.0) }
                RecenterTarget.Gimbal -> offMainDetached { gimbalRefusal(Qgc.call(GIMBAL_CENTER)) }
            }
        },
        shape = CircleShape,
        color = Color.Black.copy(alpha = RECENTER_SCRIM_ALPHA),
        contentColor = MaterialTheme.aircast.outdoorForeground,
        modifier = Modifier.height(RECENTER_HEIGHT),
    ) {
        Row(Modifier.padding(horizontal = AircastSpace.s3), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(AircastSpace.s1)) {
            Icon(painterResource(R.drawable.ic_my_location), null, Modifier.size(18.dp))
            Text("Recenter", style = MaterialTheme.typography.labelLarge)
        }
    }
}
