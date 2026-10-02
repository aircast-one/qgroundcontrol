package one.aircast.android.ui

import androidx.compose.foundation.layout.size
import one.aircast.android.R
import androidx.compose.ui.res.painterResource
import androidx.compose.material3.Icon
import androidx.compose.material3.SmallFloatingActionButton
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.material3.TextField
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.background
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
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import kotlinx.coroutines.withContext
import kotlinx.coroutines.launch
import kotlinx.coroutines.Dispatchers
import androidx.compose.material3.TextButton
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMain
import one.aircast.android.bridge.qgcPath
import one.aircast.mapspike.aircast
import one.aircast.mapspike.optText

private const val CONSOLE_ROOT = "mavlinkConsole"
private const val CONSOLE_VIEW = "view.mavlinkConsole"

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
        command = ""
        sentAnything = true
        scope.offMain { Qgc.invoke("$CONSOLE_ROOT.sendCommand", toSend) }
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
                .background(MaterialTheme.colorScheme.surfaceContainerLowest),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            items(lines) { line ->
                val warning = MaterialTheme.aircast.warning
                val error = MaterialTheme.colorScheme.error
                Text(
                    text = consoleLineStyled(line, warning, error),
                    fontFamily = FontFamily.Monospace,
                    style = MaterialTheme.typography.bodyMedium,
                    color = if (isPromptLine(line)) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface,
                )
            }
        }

        Row(
            modifier = Modifier
                .fillMaxWidth()
                .background(MaterialTheme.colorScheme.surfaceContainerLow)
                .padding(horizontal = 16.dp, vertical = 10.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            TextField(
                value = command,
                onValueChange = { command = it },
                modifier = Modifier.weight(1f),
                singleLine = true,
                shape = CircleShape,
                placeholder = { Text("Type a command") },
                colors = TextFieldDefaults.colors(
                    focusedContainerColor = MaterialTheme.colorScheme.surfaceContainerHighest,
                    unfocusedContainerColor = MaterialTheme.colorScheme.surfaceContainerHighest,
                    focusedIndicatorColor = Color.Transparent,
                    unfocusedIndicatorColor = Color.Transparent,
                ),
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Send),
                keyboardActions = KeyboardActions(onSend = { send() }),
            )
            listOf("historyUp" to "\u2191", "historyDown" to "\u2193").forEach { (step, arrow) ->
                TextButton(contentPadding = PaddingValues(0.dp), modifier = Modifier.size(40.dp), onClick = {
                    val current = command
                    scope.launch { command = withContext(Dispatchers.Default) { Qgc.invokeResult("$CONSOLE_ROOT.$step", current) as? String } ?: current }
                }) { Text(arrow) }
            }
            SmallFloatingActionButton(
                onClick = { send() },
                containerColor = MaterialTheme.colorScheme.primaryContainer,
                contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
            ) { Icon(painterResource(R.drawable.ic_send), "Send") }
        }
    }
}

internal fun isPromptLine(line: String): Boolean = Regex("^\\w*sh> ").containsMatchIn(line)

internal fun consoleLineStyled(line: String, warning: Color, error: Color): AnnotatedString {
    val marked = listOf("WARN" to warning, "ERROR" to error).firstOrNull { (prefix, _) -> line.startsWith(prefix) }
    return buildAnnotatedString {
        append(line)
        marked?.let { (prefix, color) -> addStyle(SpanStyle(color = color), 0, prefix.length) }
    }
}

internal fun consoleLines(view: org.json.JSONObject?): List<String> {
    val array = view?.optJSONArray("lines") ?: return emptyList()
    return (0 until array.length()).map { array.optText(it) }
}
