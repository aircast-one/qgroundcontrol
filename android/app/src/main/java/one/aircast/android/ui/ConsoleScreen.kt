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
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.unit.dp
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.offMainInOrder
import one.aircast.android.bridge.qgcPath
import one.aircast.mapspike.aircast
import one.aircast.mapspike.optText

private const val CONSOLE_ROOT = "mavlinkConsole"
private const val CONSOLE_VIEW = "view.mavlinkConsole"

internal const val CONSOLE_OPEN = "$CONSOLE_ROOT.open"
internal const val CONSOLE_EMPTY_TEXT = "> "

internal fun splitCompleteLines(field: TextFieldValue): Pair<String?, TextFieldValue> {
    val cut = field.text.lastIndexOf('\n')
    return if (cut < 0) {
        null to field
    } else {
        val leftover = field.text.substring(cut + 1)
        field.text.substring(0, cut) to TextFieldValue(leftover, TextRange((field.selection.end - cut - 1).coerceIn(0, leftover.length)))
    }
}

internal fun shouldFollowTail(lastVisibleIndex: Int?, count: Int): Boolean =
    lastVisibleIndex == null || lastVisibleIndex >= count - 2

@Composable
fun ConsoleScreen(modifier: Modifier = Modifier) {
    val consoleJson by qgcPath(CONSOLE_VIEW)
    val lines = remember(consoleJson) { consoleLines(consoleJson) }
    var field by remember { mutableStateOf(TextFieldValue("")) }
    val scope = rememberCoroutineScope()
    val listState = rememberLazyListState()

    LaunchedEffect(Unit) {
        offMainInOrder { Qgc.invoke(CONSOLE_OPEN) }
    }

    fun sendText(toSend: String) {
        offMainInOrder { Qgc.invoke("$CONSOLE_ROOT.sendCommand", toSend) }
    }

    fun send() {
        val toSend = field.text
        field = TextFieldValue("")
        sendText(toSend)
        if (lines.isNotEmpty()) {
            scope.launch { listState.scrollToItem(lines.size - 1) }
        }
    }

    fun edit(next: TextFieldValue) {
        val (complete, leftover) = splitCompleteLines(next)
        field = leftover
        complete?.let(::sendText)
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

    Column(
        modifier = modifier
            .fillMaxSize()
            .imePadding(),
    ) {
        LazyColumn(
            state = listState,
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth()
                .background(MaterialTheme.colorScheme.surfaceContainerLowest),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            if (lines.isEmpty()) {
                item {
                    Text(
                        text = CONSOLE_EMPTY_TEXT,
                        fontFamily = FontFamily.Monospace,
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                }
            }
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
                value = field,
                onValueChange = ::edit,
                modifier = Modifier.weight(1f),
                singleLine = true,
                shape = CircleShape,
                placeholder = { Text("Enter commands here...") },
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
                    val current = field.text
                    scope.launch {
                        val recalled = withContext(Dispatchers.Default) { Qgc.invokeResult("$CONSOLE_ROOT.$step", current) as? String } ?: current
                        field = TextFieldValue(recalled, TextRange(recalled.length))
                    }
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
