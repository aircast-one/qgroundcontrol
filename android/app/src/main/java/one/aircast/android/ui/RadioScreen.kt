package one.aircast.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
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
import androidx.compose.material3.Checkbox
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
import androidx.compose.foundation.selection.selectable
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.RadioButton
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainDetached
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

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun ModeRow(mode: Int, centeredThrottle: Boolean?, onPick: (Int) -> Unit, onCentered: (Boolean) -> Unit) {
    Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 4.dp)) {
        Text(
            "Transmitter mode — which stick is the throttle",
            style = MaterialTheme.typography.bodySmall,
            modifier = Modifier.padding(bottom = 4.dp),
        )
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            (1..4).forEach { choice ->
                FilterChip(
                    selected = choice == mode,
                    onClick = { onPick(choice) },
                    label = { Text("Mode $choice") },
                )
            }
        }
        centeredThrottle?.let { centered ->
            Row(verticalAlignment = Alignment.CenterVertically) {
                Checkbox(checked = centered, onCheckedChange = onCentered)
                Text("Centered Throttle", style = MaterialTheme.typography.bodyMedium)
            }
        }
    }
}

@Composable
private fun ConfirmDialog(
    prompt: RadioPrompt,
    onDismiss: () -> Unit,
    onConfirm: (Int?) -> Unit,
) {
    var choice by remember(prompt) { mutableStateOf(prompt.choices.indices.lastOrNull()) }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(prompt.title) },
        text = {
            Column {
                Text(prompt.body)
                prompt.choices.forEachIndexed { index, label ->
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .selectable(selected = index == choice, onClick = { choice = index })
                            .padding(vertical = 8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        RadioButton(selected = index == choice, onClick = { choice = index })
                        Text(label)
                    }
                }
            }
        },
        confirmButton = {
            TextButton(onClick = { onConfirm(choice); onDismiss() }) { Text("Ok") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

@Composable
private fun AdditionalSetup(onInvoke: (String, Int?) -> Unit) {
    var prompt by remember { mutableStateOf<RadioPrompt?>(null) }
    val asked = prompt
    if (asked != null) {
        ConfirmDialog(
            prompt = asked,
            onDismiss = { prompt = null },
            onConfirm = { choice -> onInvoke(asked.action, choice) },
        )
    }
    Column(Modifier.fillMaxWidth().padding(horizontal = 4.dp, vertical = 4.dp)) {
        Text(
            "Additional radio setup",
            style = MaterialTheme.typography.bodySmall,
            modifier = Modifier.padding(bottom = 4.dp),
        )
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            RADIO_PROMPTS.forEach { entry ->
                OutlinedButton(onClick = { prompt = entry }) { Text(entry.title) }
            }
        }
    }
}

@Composable
private fun CalibrationStart(
    view: RadioView,
    onAction: (String) -> Unit,
    onMode: (Int) -> Unit,
    onCentered: (Boolean) -> Unit,
    onInvoke: (String, Int?) -> Unit,
) {
    var prompting by remember { mutableStateOf(false) }
    view.startPrompt?.takeIf { prompting }?.let { (title, message) ->
        AlertDialog(
            onDismissRequest = { prompting = false },
            title = { Text(title) },
            text = { Text(message) },
            confirmButton = { TextButton(onClick = { prompting = false; onAction("nextButtonClicked") }) { Text("Ok") } },
        )
    }
    Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp)) {
        ModeRow(view.transmitterMode, view.centeredThrottle.takeUnless { view.joystickMode }, onMode, onCentered)
        Row(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 4.dp, vertical = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Button(
                onClick = { if (view.startPrompt != null) prompting = true else onAction("nextButtonClicked") },
                enabled = view.calibration.nextEnabled && view.enoughChannels,
            ) { Text(view.calibration.nextText.ifBlank { "Calibrate" }) }
            Text(
                text = when {
                    !view.enoughChannels -> view.shortfall.ifBlank {
                        "Not enough channels to calibrate."
                    }
                    else -> view.summary
                },
                style = MaterialTheme.typography.bodySmall,
                color = when {
                    view.enoughChannels -> MaterialTheme.colorScheme.onSurfaceVariant
                    else -> MaterialTheme.colorScheme.error
                },
            )
        }
        AdditionalSetup(onInvoke)
    }
}

internal const val RADIO_SWITCHES_PAGE = "Radio Switches"

@Composable
fun RadioScreen(modifier: Modifier = Modifier) {
    val json by qgcPath(RADIO_VIEW)
    val view = radioView(json)
    LaunchedEffect(Unit) { withContext(Dispatchers.Default) { Qgc.invoke(radioCalAction("start")) } }
    var switchReads by remember { mutableIntStateOf(0) }
    var switches by remember { mutableStateOf(emptyList<ParameterRows>()) }
    LaunchedEffect(switchReads, view?.connected) {
        switches = withContext(Dispatchers.Default) { readPage(RADIO_SWITCHES_PAGE) }
    }

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
                    offMainDetached { Qgc.invoke(radioCalAction(action)) }
                }
                else -> CalibrationStart(
                    view = view,
                    onAction = { action -> offMainDetached { Qgc.invoke(radioCalAction(action)) } },
                    onMode = { mode -> offMainDetached { Qgc.set("$RADIO_CAL.transmitterMode", mode) } },
                    onCentered = { centered -> offMainDetached { Qgc.set("$RADIO_CAL.centeredThrottle", centered) } },
                    onInvoke = { action, choice ->
                        offMainDetached {
                            when (choice) {
                                null -> Qgc.invoke(radioCalAction(action))
                                else -> Qgc.invoke(radioCalAction(action), choice)
                            }
                        }
                    },
                )
            }
        }

        item(key = "attitudeheader") { SectionHeader("Attitude controls") }
        items(view.sticks.size, key = { "att${view.sticks[it].title}" }) { index ->
            AttitudeRow(view.sticks[index])
        }

        switches.forEach { section ->
            item(key = "switches:${section.title}") { SectionHeader(section.title) }
            items(section.facts.size, key = { "switch:${section.facts[it].path}" }) { index ->
                FactRow(section.facts[index]) { switchReads++ }
            }
        }

        item(key = "monitorheader") { SectionHeader("Channel monitor") }
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
