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
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay

private const val LAYOUT_STORE = "fly-overlay-layout"
private const val HIDDEN_PREFIX = "OverlayRigHidden-"
private const val HIDDEN_ALPHA = 0.35f
internal const val RESET_ARM_MILLIS = 4000L

internal data class ResetTap(val reset: Boolean, val armed: Boolean)

internal fun resetTap(armed: Boolean): ResetTap = ResetTap(reset = armed, armed = !armed)

internal fun resetPillText(armed: Boolean): String = if (armed) "Tap again to reset" else "Reset Layout"

private const val INDICATOR_ORDER = "FlyViewIndicatorOrder"

internal object OverlayLayout {
    var editing by mutableStateOf(false)
    var hidden by mutableStateOf(emptySet<String>())
    var indicatorOrder by mutableStateOf(emptyList<String>())
}

internal fun orderedKeys(available: List<String>, order: List<String>): List<String> =
    order.filter { it in available } + available.filter { it !in order }

internal fun movedKey(keys: List<String>, key: String, delta: Int): List<String>? {
    val from = keys.indexOf(key)
    val to = from + delta
    return if (from < 0 || to !in keys.indices) null else keys.filterIndexed { i, _ -> i != from }.let { rest -> rest.take(to) + key + rest.drop(to) }
}

internal fun loadIndicatorOrder(context: Context) {
    OverlayLayout.indicatorOrder = store(context).getString(INDICATOR_ORDER, "").orEmpty().split(",").filter { it.isNotBlank() }
}

internal fun saveIndicatorOrder(context: Context, keys: List<String>) {
    store(context).edit().putString(INDICATOR_ORDER, keys.joinToString(",")).apply()
    OverlayLayout.indicatorOrder = keys
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
    OverlayLayout.indicatorOrder = emptyList()
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
            var armed by remember { mutableStateOf(false) }
            LaunchedEffect(armed) {
                if (armed) {
                    delay(RESET_ARM_MILLIS)
                    armed = false
                }
            }
            TextButton(onClick = {
                val tap = resetTap(armed)
                if (tap.reset) resetLayout(context)
                armed = tap.armed
            }) { Text(resetPillText(armed)) }
            TextButton(onClick = { OverlayLayout.editing = false }) { Text("Done") }
        }
    }
}
