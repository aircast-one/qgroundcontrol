package one.aircast.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMain
import one.aircast.android.bridge.qgcPath
import one.aircast.mapspike.optText

private const val CONSOLE_ROOT = "mavlinkConsole"
private const val CONSOLE_VIEW = "view.mavlinkConsole"

// The core answers "why is this empty" about the VEHICLE - connected or not - and this screen
// knows something the core cannot: whether the operator has sent anything yet. Its own wording
// is better once there is a vehicle, and the core's is the only correct answer when there is
// not, which this screen never used to give.
internal fun consoleEmptyText(sent: Boolean, connected: Boolean, servedReason: String): String = when {
    !connected && servedReason.isNotBlank() -> servedReason
    !connected -> "Connect a vehicle to open a shell on its autopilot."
    sent -> "Sent. Nothing back from the vehicle yet."
    else -> "No output yet. Send a command, for example help."
}

internal fun visibleConsoleLines(lines: List<String>): List<String> =
    lines.dropLastWhile { it.isBlank() }

internal fun consoleShellHint(px4Firmware: Boolean): String? =
    if (px4Firmware) {
        null
    } else {
        "This vehicle does not report PX4 firmware. The shell answers on PX4; " +
            "other autopilots may not reply to anything you send."
    }

@Composable
private fun ConsoleNotice(text: String, modifier: Modifier = Modifier) {
    Text(
        text = text,
        style = MaterialTheme.typography.bodyLarge,
        textAlign = TextAlign.Center,
        modifier = modifier
            .fillMaxWidth()
            .padding(24.dp),
    )
}

internal fun shouldFollowTail(lastVisibleIndex: Int?, count: Int): Boolean =
    lastVisibleIndex == null || lastVisibleIndex >= count - 2

@Composable
fun ConsoleScreen(modifier: Modifier = Modifier) {
    val hasVehicle = hasVehicle()
    val setupJson by qgcPath(SETUP)
    val isPx4 = remember(setupJson) { isPx4(setupReadiness(setupJson)) }
    val consoleJson by qgcPath(CONSOLE_VIEW)
    val rawLines = remember(consoleJson) { consoleLines(consoleJson) }
    val emptyReason = remember(consoleJson) { consoleJson?.optText("emptyReason").orEmpty() }
    val consoleConnected = remember(consoleJson) { consoleJson?.optBoolean("connected") == true }
    var command by remember { mutableStateOf("") }
    var sentAnything by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    val listState = rememberLazyListState()
    val lines = visibleConsoleLines(rawLines)

    fun send() {
        val toSend = command
        if (toSend.isNotBlank()) {
            command = ""
            sentAnything = true
            scope.offMain { Qgc.invoke("$CONSOLE_ROOT.sendCommand", toSend) }
        }
    }

    val following by remember {
        derivedStateOf {
            shouldFollowTail(listState.layoutInfo.visibleItemsInfo.lastOrNull()?.index, lines.size)
        }
    }

    LaunchedEffect(lines.size) {
        if (lines.isNotEmpty() && following) {
            listState.scrollToItem(lines.size - 1)
        }
    }

    if (!hasVehicle) {
        ConsoleNotice(
            consoleEmptyText(sent = false, connected = false, servedReason = emptyReason),
            modifier,
        )
        return
    }

    Column(
        modifier = modifier
            .fillMaxSize()
            .imePadding(),
    ) {
        consoleShellHint(isPx4)?.let { hint ->
            Text(
                text = hint,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 8.dp),
            )
        }

        if (lines.isEmpty()) {
            Text(
                text = consoleEmptyText(sentAnything, consoleConnected, emptyReason),
                style = MaterialTheme.typography.bodyMedium,
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(16.dp),
            )
        }

        LazyColumn(
            state = listState,
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth()
                .padding(horizontal = 12.dp),
        ) {
            items(lines) { line ->
                Text(
                    text = line,
                    fontFamily = FontFamily.Monospace,
                    style = MaterialTheme.typography.bodySmall,
                )
            }
        }

        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(12.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            OutlinedTextField(
                value = command,
                onValueChange = { command = it },
                modifier = Modifier.weight(1f),
                singleLine = true,
                label = { Text("Command") },
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Send),
                keyboardActions = KeyboardActions(onSend = { send() }),
            )
            Button(onClick = { send() }, enabled = command.isNotBlank()) { Text("Send") }
        }
    }
}

internal fun consoleLines(view: org.json.JSONObject?): List<String> {
    val array = view?.optJSONArray("lines") ?: return emptyList()
    return (0 until array.length()).map { array.optText(it) }
}
