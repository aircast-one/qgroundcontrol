package one.aircast.android.ui

import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import one.aircast.android.R
import one.aircast.android.bridge.Qgc
import one.aircast.android.bridge.qgcDerived
import one.aircast.android.bridge.refusal
import one.aircast.map.optText
import org.json.JSONObject

internal const val ROUTE_RECORDER_VIEW = "view.routeRecorder"
internal const val ROUTE_START = "routeRecorder.start"
internal const val ROUTE_STOP = "routeRecorder.stop"
internal const val ROUTE_RENAME = "routeRecorder.rename"
internal const val ROUTE_DISMISS = "routeRecorder.dismiss"

private const val BLINK_MS = 600
private const val BLINK_LOW = 0.2f
private val DOT_SIZE = 10.dp
private val NOTICE_WIDTH = 420.dp

internal data class RouteRecorder(
    val available: Boolean,
    val recording: Boolean,
    val text: String?,
    val savedFile: String?,
    val savedName: String?,
    val message: String?,
)

internal fun routeRecorder(view: JSONObject?): RouteRecorder? =
    view?.takeIf { it.optText("class") == "RouteRecorder" }?.let {
        RouteRecorder(
            available = it.optBoolean("available"),
            recording = it.optBoolean("recording"),
            text = it.optText("text").ifBlank { null },
            savedFile = it.optText("savedFile").ifBlank { null },
            savedName = it.optText("savedName").ifBlank { null },
            message = it.optText("message").ifBlank { null },
        )
    }

internal data class Renamed(val file: String, val message: String?)

internal fun renamed(answer: JSONObject?, file: String): Renamed =
    Renamed(answer?.optText("path")?.ifBlank { null } ?: file, refusal(answer, ROUTE_SUBJECT))

private const val ROUTE_SUBJECT = "The route recorder"

internal fun routeTile(recorder: RouteRecorder?, onToggle: (Boolean) -> Unit): MoreTile? =
    recorder?.takeIf { it.available }?.let { shown ->
        MoreTile(
            label = if (shown.recording) "Stop recording" else "Record route",
            icon = if (shown.recording) R.drawable.ic_stop_circle else R.drawable.ic_route,
            enabled = true,
            warning = shown.recording,
        ) { onToggle(shown.recording) }
    }

internal fun toggleRoute(recording: Boolean): String? =
    refusal(Qgc.call(if (recording) ROUTE_STOP else ROUTE_START), ROUTE_SUBJECT)

internal fun openRouteInPlan(file: String): String? {
    val answer = planLoad(file)
    if (!planLoaded(answer)) return loadFailureMessage(answer).orEmpty()
    Qgc.invoke(ROUTE_DISMISS)
    return null
}

@Composable
internal fun RouteRecordingOverlay(onOpenedInPlan: () -> Unit) {
    val recorder by qgcDerived(ROUTE_RECORDER_VIEW, ::routeRecorder)
    val shown = recorder ?: return
    val flyScreen = LocalFlyScreenState.current
    val scope = rememberCoroutineScope()
    Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(8.dp)) {
        shown.text?.let { text ->
            RecordingPill(text) { scope.launch { withContext(Dispatchers.Default) { toggleRoute(recording = true) }?.let { flyScreen.refusal = it } } }
        }
        shown.message?.takeIf { !shown.recording }?.let { message -> RouteSavedNotice(shown, message, onOpenedInPlan) }
    }
}

@Composable
internal fun RecordingPill(text: String, onStop: () -> Unit) {
    val blink by rememberInfiniteTransition(label = "recording").animateFloat(
        initialValue = 1f,
        targetValue = BLINK_LOW,
        animationSpec = infiniteRepeatable(tween(BLINK_MS), RepeatMode.Reverse),
        label = "dot",
    )
    Surface(shape = CircleShape, color = MaterialTheme.colorScheme.surface.copy(alpha = 0.85f)) {
        Row(Modifier.padding(start = 12.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Box(Modifier.size(DOT_SIZE).alpha(blink).background(Color.Red, CircleShape))
            Text(text, style = MaterialTheme.typography.labelLarge)
            TextButton(onClick = onStop) { Text("Stop") }
        }
    }
}

@Composable
private fun RouteSavedNotice(recorder: RouteRecorder, message: String, onOpenedInPlan: () -> Unit) {
    val scope = rememberCoroutineScope()
    val openPlan = LocalOpenPlan.current
    val focus = LocalFocusManager.current
    var typed by remember(recorder.savedName) { mutableStateOf(recorder.savedName.orEmpty()) }
    var said by remember(recorder.savedFile) { mutableStateOf<String?>(null) }
    var editing by remember { mutableStateOf(false) }
    val rename = {
        val file = recorder.savedFile
        val wanted = typed.trim()
        if (file != null && wanted.isNotEmpty() && wanted != recorder.savedName) {
            scope.launch {
                said = withContext(Dispatchers.Default) { renamed(Qgc.call(ROUTE_RENAME, file, wanted), file) }.message
                typed = recorder.savedName.orEmpty()
            }
        } else {
            typed = recorder.savedName.orEmpty()
        }
    }
    Surface(shape = MaterialTheme.shapes.medium, color = MaterialTheme.colorScheme.surface.copy(alpha = 0.9f), modifier = Modifier.widthIn(max = NOTICE_WIDTH)) {
        Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(said ?: message, style = MaterialTheme.typography.titleSmall)
            recorder.savedFile?.let {
                OutlinedTextField(
                    value = typed,
                    onValueChange = { typed = it },
                    label = { Text("Name") },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                    keyboardActions = KeyboardActions(onDone = { focus.clearFocus() }),
                    modifier = Modifier.onFocusChanged { state ->
                        if (editing && !state.isFocused) rename()
                        editing = state.isFocused
                    },
                )
            }
            Row(Modifier.align(Alignment.End), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                recorder.savedFile?.let { file ->
                    TextButton(onClick = {
                        scope.launch {
                            val failed = withContext(Dispatchers.Default) { openRouteInPlan(file) }
                            if (failed == null) {
                                openPlan.document = null
                                openPlan.name = null
                                onOpenedInPlan()
                            } else {
                                said = failed.ifBlank { null }
                            }
                        }
                    }) { Text("Open in Plan") }
                }
                TextButton(onClick = { scope.launch(Dispatchers.Default) { Qgc.invoke(ROUTE_DISMISS) } }) { Text("Dismiss") }
            }
        }
    }
}
