package one.aircast.android.ui

import android.content.Context
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Add
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SmallFloatingActionButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp

private const val LAYOUT_STORE = "fly-overlay-layout"
private const val HIDDEN_PREFIX = "OverlayRigHidden-"
private const val HIDDEN_ALPHA = 0.35f

internal object OverlayLayout {
    var editing by mutableStateOf(false)
    var hidden by mutableStateOf(emptySet<String>())
}

internal fun hiddenKeys(stored: Map<String, *>): Set<String> =
    stored.filter { (key, value) -> key.startsWith(HIDDEN_PREFIX) && value == true }.keys.map { it.removePrefix(HIDDEN_PREFIX) }.toSet()

internal fun withHidden(hidden: Set<String>, key: String, hide: Boolean): Set<String> = if (hide) hidden + key else hidden - key

private fun store(context: Context) = context.getSharedPreferences(LAYOUT_STORE, Context.MODE_PRIVATE)

private fun setHidden(context: Context, key: String, hide: Boolean) {
    store(context).edit().putBoolean(HIDDEN_PREFIX + key, hide).apply()
    OverlayLayout.hidden = withHidden(OverlayLayout.hidden, key, hide)
}

private fun resetLayout(context: Context) {
    store(context).edit().clear().apply()
    OverlayLayout.hidden = emptySet()
}

private fun Modifier.onHold(action: () -> Unit): Modifier = pointerInput(Unit) {
    awaitEachGesture {
        awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
        val released = withTimeoutOrNull(viewConfiguration.longPressTimeoutMillis) {
            waitForUpOrCancellation(PointerEventPass.Initial)
            true
        }
        if (released == null) action()
    }
}

@Composable
internal fun Hideable(key: String, content: @Composable () -> Unit) {
    val context = LocalContext.current
    LaunchedEffect(Unit) { OverlayLayout.hidden = hiddenKeys(store(context).all) }
    val hidden = key in OverlayLayout.hidden
    if (hidden && !OverlayLayout.editing) return
    Box(Modifier.onHold { OverlayLayout.editing = true }.alpha(if (hidden) HIDDEN_ALPHA else 1f)) {
        content()
        if (OverlayLayout.editing) {
            SmallFloatingActionButton(onClick = { setHidden(context, key, !hidden) }, modifier = Modifier.align(Alignment.TopEnd).size(28.dp)) {
                Icon(if (hidden) Icons.Default.Add else Icons.Default.Close, contentDescription = if (hidden) "Show" else "Hide", modifier = Modifier.size(16.dp))
            }
        }
    }
}

@Composable
internal fun OverlayEditBar(modifier: Modifier = Modifier) {
    if (!OverlayLayout.editing) return
    val context = LocalContext.current
    Surface(modifier, shape = MaterialTheme.shapes.medium, color = MaterialTheme.colorScheme.surfaceContainerHigh, tonalElevation = 3.dp) {
        Row(Modifier.padding(horizontal = 12.dp), verticalAlignment = Alignment.CenterVertically) {
            Text("Hide or show widgets", style = MaterialTheme.typography.labelMedium)
            TextButton(onClick = { resetLayout(context) }) { Text("Reset layout") }
            TextButton(onClick = { OverlayLayout.editing = false }) { Text("Done") }
        }
    }
}
