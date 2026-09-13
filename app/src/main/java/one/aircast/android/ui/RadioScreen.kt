package one.aircast.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.foundation.layout.Spacer
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Surface
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.TextButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcPath

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
private fun PwmBar(fraction: Float, modifier: Modifier = Modifier) {
    Box(
        modifier = modifier
            .height(8.dp)
            .clip(RoundedCornerShape(4.dp))
            .background(MaterialTheme.colorScheme.surfaceVariant),
    ) {
        Box(
            Modifier
                .fillMaxWidth(fraction)
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.primary),
        )
    }
}

@Composable
private fun AttitudeRow(stick: RadioStick) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 20.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text(
            text = stick.title,
            style = MaterialTheme.typography.bodyMedium,
            modifier = Modifier.width(72.dp),
        )
        if (!stick.mapped) {
            Text(
                text = "Not mapped",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.error,
                modifier = Modifier.weight(1f),
            )
        } else {
            PwmBar(stick.fraction, Modifier.weight(1f))
            Text(
                text = if (stick.reversed) "${stick.valueText} R" else stick.valueText,
                style = MaterialTheme.typography.bodySmall,
                fontFamily = FontFamily.Monospace,
                modifier = Modifier.width(64.dp),
            )
        }
    }
}

@Composable
private fun CalibrationStep(cal: RadioCalibration, onAction: (String) -> Unit) {
    Surface(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
        color = MaterialTheme.colorScheme.primaryContainer,
        shape = RoundedCornerShape(12.dp),
    ) {
        Column(Modifier.padding(16.dp)) {
            Text(
                text = calibrationStep(cal.statusText),
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.fillMaxWidth().padding(bottom = 16.dp),
            )
            Row(
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Button(onClick = { onAction("nextButtonClicked") }, enabled = cal.nextEnabled) {
                    Text(cal.nextText.ifBlank { "Next" })
                }
                if (cal.skipEnabled) {
                    OutlinedButton(onClick = { onAction("skipButtonClicked") }) { Text("Skip") }
                }
                Spacer(Modifier.weight(1f))
                if (cal.cancelEnabled) {
                    TextButton(onClick = { onAction("cancelButtonClicked") }) { Text("Cancel") }
                }
            }
        }
    }
}

@Composable
private fun ModeRow(mode: Int, onPick: (Int) -> Unit) {
    Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 4.dp)) {
        Text(
            "Transmitter mode — which stick is the throttle",
            style = MaterialTheme.typography.bodySmall,
            modifier = Modifier.padding(bottom = 4.dp),
        )
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            (1..4).forEach { choice ->
                FilterChip(
                    selected = choice == mode,
                    onClick = { onPick(choice) },
                    label = { Text("Mode $choice") },
                )
            }
        }
    }
}

@Composable
private fun CalibrationStart(view: RadioView, onAction: (String) -> Unit, onMode: (Int) -> Unit) {
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp)) {
        ModeRow(view.transmitterMode, onMode)
        Row(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 4.dp, vertical = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Button(
                onClick = { onAction("nextButtonClicked") },
                enabled = view.calibration.nextEnabled && view.enoughChannels,
            ) { Text(view.calibration.nextText.ifBlank { "Calibrate" }) }
            Text(
                text = when {
                    !view.enoughChannels -> view.shortfall.ifBlank {
                        "Not enough channels to calibrate."
                    }
                    else -> "Holds each stick at its extremes in turn and rewrites the mapping."
                },
                style = MaterialTheme.typography.bodySmall,
                color = when {
                    view.enoughChannels -> MaterialTheme.colorScheme.onSurfaceVariant
                    else -> MaterialTheme.colorScheme.error
                },
            )
        }
    }
}

@Composable
fun RadioScreen(modifier: Modifier = Modifier) {
    val json by qgcPath(RADIO_VIEW)
    val view = radioView(json)
    LaunchedEffect(Unit) { Qgc.invoke(radioCalAction("start")) }

    if (view == null || !view.connected) {
        RadioNotice("Connect a vehicle to check its radio.", modifier)
        return
    }

    LazyColumn(modifier.fillMaxSize()) {
        if (view.channelCount == 0) {
            item(key = "nochannels") {
                RadioNotice(
                    "No transmitter signal. Turn the transmitter on and check the " +
                        "receiver is bound.",
                )
            }
        }

        item(key = "calibration") {
            when {
                view.calibration.running -> CalibrationStep(view.calibration) { action ->
                    Qgc.invoke(radioCalAction(action))
                }
                else -> CalibrationStart(
                    view = view,
                    onAction = { action -> Qgc.invoke(radioCalAction(action)) },
                    onMode = { mode -> Qgc.set("$RADIO_CAL.transmitterMode", mode) },
                )
            }
        }

        item(key = "attitudeheader") { SectionHeader("Attitude controls") }
        items(view.sticks.size, key = { "att${view.sticks[it].title}" }) { index ->
            AttitudeRow(view.sticks[index])
        }

        item(key = "monitorheader") { SectionHeader("Channel monitor") }
        if (view.summary.isNotBlank()) {
            item(key = "summary") {
                Text(
                    text = view.summary,
                    style = MaterialTheme.typography.bodySmall,
                    modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 4.dp),
                )
            }
        }
        if (view.shortfall.isNotBlank()) {
            item(key = "shortfall") {
                Text(
                    text = view.shortfall,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 4.dp),
                )
            }
        }
        items(view.channels.size, key = { "ch${view.channels[it].label}" }) { index ->
            val channel = view.channels[index]
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 20.dp, vertical = 6.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Text(
                    text = channel.label,
                    style = MaterialTheme.typography.bodySmall,
                    fontFamily = FontFamily.Monospace,
                    modifier = Modifier.width(28.dp),
                )
                PwmBar(channel.fraction, Modifier.weight(1f))
                Text(
                    text = channel.valueText,
                    style = MaterialTheme.typography.bodySmall,
                    fontFamily = FontFamily.Monospace,
                    modifier = Modifier.width(48.dp),
                )
            }
        }

        item(key = "footer") {
            FootNote(
                "Move each stick and switch \u2014 every channel you use should move here. " +
                    "Calibration asks you to hold each stick at its extremes in turn and " +
                    "rewrites the mapping, so do it with the propellers off.",
            )
        }
    }
}
