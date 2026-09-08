package one.aircast.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import java.util.Locale
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcDouble

private val VIBE_HIGH_COLOR = Color(0xFFFF5252)
private val VIBE_WARN_COLOR = Color(0xFFFFA000)

internal const val VIBE_MAX = 90.0
internal const val VIBE_WARN = 30.0
internal const val VIBE_HIGH = 60.0
private const val VIBE_UNITS = "m/s²"

private val AXIS_WIDTH = 32.dp
private val BAR_WIDTH = 56.dp

private val SCALE_LABELS = listOf("90", "60", "30", "0")

private val AXES = listOf(
    "X" to "vehicle.vibration.xAxis",
    "Y" to "vehicle.vibration.yAxis",
    "Z" to "vehicle.vibration.zAxis",
)

private val CLIPS = listOf(
    "Accel 1" to "vehicle.vibration.clipCount1",
    "Accel 2" to "vehicle.vibration.clipCount2",
    "Accel 3" to "vehicle.vibration.clipCount3",
)

internal fun barFraction(value: Double): Float =
    if (value.isNaN()) 0f else (value / VIBE_MAX).coerceIn(0.0, 1.0).toFloat()

internal fun verdictFor(value: Double) = when {
    value.isNaN() -> ""
    value >= VIBE_HIGH -> "High"
    value >= VIBE_WARN -> "Caution"
    else -> "OK"
}

@Composable
private fun colorFor(value: Double) = when {
    value.isNaN() -> MaterialTheme.colorScheme.surfaceVariant
    value >= VIBE_HIGH -> VIBE_HIGH_COLOR
    value >= VIBE_WARN -> VIBE_WARN_COLOR
    else -> MaterialTheme.colorScheme.primary
}

@Composable
private fun ScaleAxis(modifier: Modifier = Modifier) {
    Column(
        modifier = modifier.width(AXIS_WIDTH),
        verticalArrangement = Arrangement.SpaceBetween,
        horizontalAlignment = Alignment.End,
    ) {
        SCALE_LABELS.forEach { label ->
            Text(text = label, style = MaterialTheme.typography.labelSmall)
        }
    }
}

@Composable
private fun VibrationBarGraphic(value: Double, modifier: Modifier = Modifier) {
    val fraction = barFraction(value)
    Box(modifier, contentAlignment = Alignment.BottomCenter) {
        Box(
            Modifier
                .fillMaxHeight()
                .width(BAR_WIDTH)
                .clip(RoundedCornerShape(6.dp))
                .background(MaterialTheme.colorScheme.surfaceVariant),
            contentAlignment = Alignment.BottomCenter,
        ) {
            Box(
                Modifier
                    .fillMaxWidth()
                    .fillMaxHeight(fraction)
                    .background(colorFor(value)),
            )
            Column(
                modifier = Modifier.fillMaxSize(),
                verticalArrangement = Arrangement.SpaceBetween,
            ) {
                SCALE_LABELS.forEach { _ ->
                    HorizontalDivider(color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        }
    }
}

@Composable
private fun VibrationReadout(label: String, value: Double, modifier: Modifier = Modifier) {
    Column(
        modifier = modifier,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text(
            text = if (value.isNaN()) "--" else String.format(Locale.US, "%.1f", value),
            style = MaterialTheme.typography.titleMedium,
        )
        Text(text = label, style = MaterialTheme.typography.labelLarge)
        Text(text = verdictFor(value), style = MaterialTheme.typography.labelMedium)
    }
}

@Composable
private fun EmptyState(message: String, detail: String, modifier: Modifier = Modifier) {
    Column(
        modifier = modifier
            .fillMaxSize()
            .padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text(
            text = message,
            style = MaterialTheme.typography.titleMedium,
            textAlign = TextAlign.Center,
        )
        Text(
            text = detail,
            style = MaterialTheme.typography.bodyMedium,
            textAlign = TextAlign.Center,
        )
    }
}

@Composable
fun VibrationScreen(modifier: Modifier = Modifier) {
    val hasVehicle by qgcBool("vehicles.activeVehicleAvailable")
    val values = AXES.map { (label, path) -> label to qgcDouble(path).value }
    val clips = CLIPS.map { (label, path) -> label to qgcDouble(path).value }
    val reporting = values.any { !it.second.isNaN() }

    if (!hasVehicle) {
        EmptyState(
            "No vehicle connected",
            "Connect a vehicle from the Fly view to see its vibration levels.",
            modifier,
        )
        return
    }

    if (!reporting) {
        EmptyState(
            "This vehicle is not reporting vibration",
            "The autopilot has not sent a VIBRATION message. Not all firmware and airframes publish one.",
            modifier,
        )
        return
    }

    Column(
        modifier = modifier
            .fillMaxSize()
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text(
            text = "Vibration ($VIBE_UNITS)",
            style = MaterialTheme.typography.titleSmall,
        )

        Row(
            modifier = Modifier
                .fillMaxWidth()
                .weight(1f),
        ) {
            ScaleAxis(Modifier.fillMaxHeight())
            values.forEach { (_, value) ->
                VibrationBarGraphic(value, Modifier.fillMaxHeight().weight(1f))
            }
        }

        Row(modifier = Modifier.fillMaxWidth()) {
            Spacer(Modifier.width(AXIS_WIDTH))
            values.forEach { (label, value) ->
                VibrationReadout(label, value, Modifier.weight(1f))
            }
        }

        Text(
            text = "Under 30 healthy · 30-60 watch · over 60 unsafe",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )

        HorizontalDivider()

        Text(
            text = "Accelerometer clipping events",
            style = MaterialTheme.typography.titleSmall,
        )
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceEvenly,
        ) {
            clips.forEach { (label, value) ->
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Text(
                        text = if (value.isNaN()) "--" else value.toInt().toString(),
                        style = MaterialTheme.typography.titleMedium,
                    )
                    Text(text = label, style = MaterialTheme.typography.labelMedium)
                }
            }
        }
        Text(
            text = "Any clipping in flight means the accelerometer saturated. Expect zero.",
            style = MaterialTheme.typography.bodySmall,
        )
    }
}
