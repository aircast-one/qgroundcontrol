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
import androidx.compose.runtime.remember
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import java.util.Locale
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcPath
import org.json.JSONObject
import one.aircast.android.bridge.qgcDouble
import one.aircast.mapspike.optText

private val VIBE_HIGH_COLOR = Color(0xFFFF5252)
private val VIBE_WARN_COLOR = Color(0xFFFFA000)

private const val VIBRATION = "view.vibration"
private const val SCALE_STEPS = 4

private val AXIS_WIDTH = 32.dp
private val BAR_WIDTH = 56.dp

internal data class VibrationAxis(
    val axis: String,
    val value: Double?,
    val fraction: Float,
    val severity: String?,
)

internal data class VibrationReading(
    val units: String,
    val scaleMaximum: Double,
    val warningLevel: Double,
    val dangerLevel: Double,
    val axes: List<VibrationAxis>,
    val clipCounts: List<Int>,
)

private fun JSONObject.doubleOrNull(key: String): Double? =
    if (isNull(key)) null else optDouble(key).takeIf { !it.isNaN() }

private fun JSONObject.stringOrNull(key: String): String? =
    if (isNull(key)) null else optString(key).ifBlank { null }

internal fun vibrationReading(view: JSONObject?): VibrationReading? {
    if (view == null || !view.optBoolean("available")) return null
    val axes = view.optJSONArray("axes") ?: return null
    val clips = view.optJSONArray("clipCounts")
    return VibrationReading(
        units = view.optText("units"),
        scaleMaximum = view.optDouble("scaleMaximum", 0.0),
        warningLevel = view.optDouble("warningLevel", 0.0),
        dangerLevel = view.optDouble("dangerLevel", 0.0),
        axes = (0 until axes.length()).mapNotNull { index ->
            axes.optJSONObject(index)?.let { axis ->
                VibrationAxis(
                    axis = axis.stringOrNull("label") ?: axis.optText("axis").uppercase(Locale.US),
                    value = axis.doubleOrNull("value"),
                    fraction = axis.doubleOrNull("fraction")?.toFloat() ?: 0f,
                    severity = axis.stringOrNull("severity"),
                )
            }
        },
        clipCounts = (0 until (clips?.length() ?: 0)).map { clips!!.optInt(it) },
    )
}

internal fun severityLabel(severity: String?): String = when (severity) {
    "danger" -> "High"
    "warning" -> "Caution"
    "normal" -> "OK"
    else -> ""
}

internal fun vibrationHeading(units: String): String =
    if (units.isBlank()) "Vibration" else "Vibration ($units)"

internal fun bandCaption(warningLevel: Double, dangerLevel: Double): String {
    val warn = warningLevel.toInt()
    val danger = dangerLevel.toInt()
    return "Under $warn healthy · $warn-$danger watch · over $danger unsafe"
}

internal fun scaleLabels(scaleMaximum: Double, warningLevel: Double, dangerLevel: Double): List<String> =
    listOf(scaleMaximum, dangerLevel, warningLevel, 0.0).map { it.toInt().toString() }

@Composable
private fun colorFor(severity: String?) = when (severity) {
    "danger" -> VIBE_HIGH_COLOR
    "warning" -> VIBE_WARN_COLOR
    "normal" -> MaterialTheme.colorScheme.primary
    else -> MaterialTheme.colorScheme.surfaceVariant
}

@Composable
private fun ScaleAxis(labels: List<String>, modifier: Modifier = Modifier) {
    Column(
        modifier = modifier.width(AXIS_WIDTH),
        verticalArrangement = Arrangement.SpaceBetween,
        horizontalAlignment = Alignment.End,
    ) {
        labels.forEach { label ->
            Text(text = label, style = MaterialTheme.typography.labelSmall)
        }
    }
}

@Composable
private fun VibrationBarGraphic(axis: VibrationAxis, modifier: Modifier = Modifier) {
    val fraction = axis.fraction
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
                    .background(colorFor(axis.severity)),
            )
            Column(
                modifier = Modifier.fillMaxSize(),
                verticalArrangement = Arrangement.SpaceBetween,
            ) {
                repeat(SCALE_STEPS) {
                    HorizontalDivider(color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        }
    }
}

@Composable
private fun VibrationReadout(axis: VibrationAxis, modifier: Modifier = Modifier) {
    Column(
        modifier = modifier,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text(
            text = axis.value?.let { String.format(Locale.US, "%.1f", it) } ?: "--",
            style = MaterialTheme.typography.titleMedium,
        )
        Text(text = axis.axis, style = MaterialTheme.typography.labelLarge)
        Text(text = severityLabel(axis.severity), style = MaterialTheme.typography.labelMedium)
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
    val view by qgcPath(VIBRATION)
    val reading = remember(view) { vibrationReading(view) }
    val reporting = reading?.axes?.any { it.value != null } == true

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
            text = vibrationHeading(reading!!.units),
            style = MaterialTheme.typography.titleSmall,
        )

        Row(
            modifier = Modifier
                .fillMaxWidth()
                .weight(1f),
        ) {
            ScaleAxis(
                scaleLabels(reading!!.scaleMaximum, reading.warningLevel, reading.dangerLevel),
                Modifier.fillMaxHeight(),
            )
            reading.axes.forEach { axis ->
                VibrationBarGraphic(axis, Modifier.fillMaxHeight().weight(1f))
            }
        }

        Row(modifier = Modifier.fillMaxWidth()) {
            Spacer(Modifier.width(AXIS_WIDTH))
            reading.axes.forEach { axis ->
                VibrationReadout(axis, Modifier.weight(1f))
            }
        }

        Text(
            text = bandCaption(reading.warningLevel, reading.dangerLevel),
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
            reading.clipCounts.forEachIndexed { index, count ->
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Text(text = count.toString(), style = MaterialTheme.typography.titleMedium)
                    Text(text = "Accel ${index + 1}", style = MaterialTheme.typography.labelMedium)
                }
            }
        }
        Text(
            text = "Any clipping in flight means the accelerometer saturated. Expect zero.",
            style = MaterialTheme.typography.bodySmall,
        )
    }
}
