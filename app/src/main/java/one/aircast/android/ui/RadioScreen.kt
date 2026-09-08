package one.aircast.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import org.json.JSONObject
import one.aircast.android.bridge.qgcBool
import one.aircast.android.bridge.qgcDouble
import one.aircast.android.bridge.qgcPath

private const val CAL = "radioCal"

internal data class AttitudeChannel(
    val label: String,
    val mapped: Boolean,
    val pwm: Int,
    val reversed: Boolean,
)

internal fun pwmFraction(pwm: Int): Float =
    ((pwm - PWM_MIN).toDouble() / (PWM_MAX - PWM_MIN)).toFloat().coerceIn(0f, 1f)

internal fun parseRcValues(json: JSONObject?): List<Int> {
    val array = json?.optJSONArray("value") ?: return emptyList()
    return (0 until array.length()).map { array.optInt(it) }
}

@Composable
private fun RadioNotice(text: String, modifier: Modifier = Modifier) {
    Text(
        text = text,
        style = MaterialTheme.typography.bodyLarge,
        textAlign = TextAlign.Center,
        modifier = modifier
            .fillMaxWidth()
            .padding(24.dp),
    )
}

@Composable
private fun PwmBar(pwm: Int, modifier: Modifier = Modifier) {
    Box(
        modifier = modifier
            .height(8.dp)
            .clip(RoundedCornerShape(4.dp))
            .background(MaterialTheme.colorScheme.surfaceVariant),
    ) {
        Box(
            Modifier
                .fillMaxWidth(pwmFraction(pwm))
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.primary),
        )
    }
}

@Composable
private fun AttitudeRow(channel: AttitudeChannel) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 20.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text(
            text = channel.label,
            style = MaterialTheme.typography.bodyMedium,
            modifier = Modifier.width(72.dp),
        )
        if (!channel.mapped) {
            Text(
                text = "Not mapped",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.weight(1f),
            )
        } else {
            PwmBar(channel.pwm, Modifier.weight(1f))
            Text(
                text = if (channel.reversed) "${channel.pwm} R" else "${channel.pwm}",
                style = MaterialTheme.typography.bodySmall,
                fontFamily = FontFamily.Monospace,
                modifier = Modifier.width(64.dp),
            )
        }
    }
}

@Composable
fun RadioScreen(modifier: Modifier = Modifier) {
    val hasVehicle by qgcBool("vehicles.activeVehicleAvailable")
    val channelCount by qgcDouble("$CAL.channelCount", 0.0)
    val rcValuesJson by qgcPath("$CAL.rcValues")

    val rollMapped by qgcBool("$CAL.rollChannelMapped")
    val pitchMapped by qgcBool("$CAL.pitchChannelMapped")
    val yawMapped by qgcBool("$CAL.yawChannelMapped")
    val throttleMapped by qgcBool("$CAL.throttleChannelMapped")

    val rollPwm by qgcDouble("$CAL.rollChannelRCValue", 1500.0)
    val pitchPwm by qgcDouble("$CAL.pitchChannelRCValue", 1500.0)
    val yawPwm by qgcDouble("$CAL.yawChannelRCValue", 1500.0)
    val throttlePwm by qgcDouble("$CAL.throttleChannelRCValue", 1500.0)

    val rollRev by qgcDouble("$CAL.rollChannelReversed", 0.0)
    val pitchRev by qgcDouble("$CAL.pitchChannelReversed", 0.0)
    val yawRev by qgcDouble("$CAL.yawChannelReversed", 0.0)
    val throttleRev by qgcDouble("$CAL.throttleChannelReversed", 0.0)

    if (!hasVehicle) {
        RadioNotice("Connect a vehicle to check its radio.", modifier)
        return
    }

    val attitude = listOf(
        AttitudeChannel("Roll", rollMapped, rollPwm.toInt(), rollRev != 0.0),
        AttitudeChannel("Pitch", pitchMapped, pitchPwm.toInt(), pitchRev != 0.0),
        AttitudeChannel("Yaw", yawMapped, yawPwm.toInt(), yawRev != 0.0),
        AttitudeChannel("Throttle", throttleMapped, throttlePwm.toInt(), throttleRev != 0.0),
    )
    val rcValues = parseRcValues(rcValuesJson)

    LazyColumn(modifier.fillMaxSize()) {
        if (channelCount.toInt() == 0) {
            item(key = "nochannels") {
                RadioNotice(
                    "No transmitter signal. Turn the transmitter on and check the " +
                        "receiver is bound.",
                )
            }
        }

        item(key = "attitudeheader") { SectionHeader("Attitude controls") }
        items(attitude.size, key = { "att${attitude[it].label}" }) { index ->
            AttitudeRow(attitude[index])
        }

        item(key = "monitorheader") {
            SectionHeader("Channel monitor · ${channelCount.toInt()} channels")
        }
        items(rcValues.size, key = { "ch$it" }) { index ->
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 20.dp, vertical = 6.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Text(
                    text = "${index + 1}",
                    style = MaterialTheme.typography.bodySmall,
                    fontFamily = FontFamily.Monospace,
                    modifier = Modifier.width(28.dp),
                )
                PwmBar(rcValues[index], Modifier.weight(1f))
                Text(
                    text = "${rcValues[index]}",
                    style = MaterialTheme.typography.bodySmall,
                    fontFamily = FontFamily.Monospace,
                    modifier = Modifier.width(48.dp),
                )
            }
        }

        item(key = "footer") {
            FootNote(
                "Move each stick and switch — every channel you use should move here. " +
                    "Calibration stays on the desktop: it needs you holding each stick at " +
                    "its extremes while watching the aircraft, and it rewrites the mapping.",
            )
        }
    }
}
