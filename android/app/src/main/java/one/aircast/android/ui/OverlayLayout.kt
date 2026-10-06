package one.aircast.android.ui

import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import android.content.Context
import android.content.SharedPreferences
import androidx.activity.compose.BackHandler
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.layout.positionInRoot
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.layout.findRootCoordinates
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.toSize
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay

private const val LAYOUT_STORE = "fly-overlay-layout"
private const val HIDDEN_PREFIX = "OverlayRigHidden-"
private const val OFFSET_PREFIX = "OverlayRigOffset-"
private const val LANDSCAPE_SUFFIX = "@landscape"
private const val HIDDEN_ALPHA = 0.35f
private const val JIGGLE_DEGREES = 1.2f
private const val JIGGLE_MILLIS = 120
private const val JIGGLE_SPREAD = 0x1f
private val BADGE_SIZE = 24.dp
internal const val RESET_ARM_MILLIS = 4000L

internal data class ResetTap(val reset: Boolean, val armed: Boolean)

internal fun resetTap(armed: Boolean): ResetTap = ResetTap(reset = armed, armed = !armed)

internal fun resetPillText(armed: Boolean): String = if (armed) "Tap again to reset" else "Reset layout"

private const val INDICATOR_ORDER = "FlyViewIndicatorOrder"

internal class OverlayLayoutState(private val prefs: SharedPreferences) {
    private val stored = prefs.all
    var editing by mutableStateOf(false)
    var locked by mutableStateOf(false)
        private set
    var hidden by mutableStateOf(hiddenKeys(stored))
        private set
    var indicatorOrder by mutableStateOf(storedIndicatorOrder(stored))
        private set
    var valueSize by mutableStateOf(ValueSize.Default)
    var offsets by mutableStateOf(storedOffsets(stored))
        private set

    fun startEditing() {
        if (!locked) editing = true
    }

    fun lockWhileArmed(armed: Boolean) {
        locked = armed
        if (armed) editing = false
    }

    fun setHidden(key: String, hide: Boolean) {
        prefs.edit().putBoolean(HIDDEN_PREFIX + key, hide).apply()
        hidden = withHidden(hidden, key, hide)
    }

    fun saveIndicatorOrder(keys: List<String>) {
        prefs.edit().putString(INDICATOR_ORDER, keys.joinToString(",")).apply()
        indicatorOrder = keys
    }

    fun nudge(key: String, dx: Float, dy: Float) {
        val current = offsets[key] ?: (0f to 0f)
        offsets = offsets + (key to (current.first + dx to current.second + dy))
    }

    fun saveOffset(key: String) {
        offsets[key]?.let { (x, y) -> prefs.edit().putString(OFFSET_PREFIX + key, "$x,$y").apply() }
    }

    fun reset() {
        prefs.edit().clear().apply()
        hidden = emptySet()
        indicatorOrder = emptyList()
        offsets = emptyMap()
    }
}

internal fun overlayLayoutStore(context: Context): SharedPreferences = context.getSharedPreferences(LAYOUT_STORE, Context.MODE_PRIVATE)

internal fun storedIndicatorOrder(stored: Map<String, *>): List<String> =
    (stored[INDICATOR_ORDER] as? String).orEmpty().split(",").filter { it.isNotBlank() }


internal fun onScreenCorrection(bounds: Rect, root: Size, offset: Pair<Float, Float>): Pair<Float, Float> {
    val (dx, dy) = clampedDrag(bounds.left, bounds.top, bounds.right, bounds.bottom, root.width, root.height, 0f, 0f)
    return pulledBack(if (bounds.width > root.width) 0f else dx, offset.first) to pulledBack(if (bounds.height > root.height) 0f else dy, offset.second)
}

internal fun pulledBack(correction: Float, offset: Float): Float = when {
    offset > 0f -> correction.coerceIn(-offset, 0f)
    offset < 0f -> correction.coerceIn(0f, -offset)
    else -> 0f
}

internal fun orientedKey(key: String, landscape: Boolean): String = if (landscape) "$key$LANDSCAPE_SUFFIX" else key

internal fun orderedKeys(available: List<String>, order: List<String>): List<String> =
    order.filter { it in available } + available.filter { it !in order }

internal fun movedKey(keys: List<String>, key: String, delta: Int): List<String>? {
    val from = keys.indexOf(key)
    val to = from + delta
    return if (from < 0 || to !in keys.indices) null else keys.filterIndexed { i, _ -> i != from }.let { rest -> rest.take(to) + key + rest.drop(to) }
}



internal fun hiddenKeys(stored: Map<String, *>): Set<String> =
    stored.filter { (key, value) -> key.startsWith(HIDDEN_PREFIX) && value == true }.keys.map { it.removePrefix(HIDDEN_PREFIX) }.toSet()

internal fun withHidden(hidden: Set<String>, key: String, hide: Boolean): Set<String> = if (hide) hidden + key else hidden - key



internal fun storedOffsets(stored: Map<String, *>): Map<String, Pair<Float, Float>> =
    stored.filterKeys { it.startsWith(OFFSET_PREFIX) }.mapNotNull { (key, value) ->
        (value as? String)?.split(",")?.mapNotNull { it.toFloatOrNull() }?.takeIf { it.size == 2 }?.let { key.removePrefix(OFFSET_PREFIX) to (it[0] to it[1]) }
    }.toMap()



private fun Modifier.onRootBounds(seen: (Rect, Size) -> Unit): Modifier = onGloballyPositioned {
    seen(Rect(it.positionInRoot(), it.size.toSize()), it.findRootCoordinates().size.toSize())
}

@Composable
private fun layoutKey(key: String): String = orientedKey(key, !flyIsPortrait())

@Composable
private fun keptOnScreen(key: String): Modifier {
    val layout = LocalFlyScreenState.current.layout
    val density = LocalDensity.current.density
    return Modifier.onRootBounds { bounds, root ->
        val (x, y) = layout.offsets[key] ?: (0f to 0f)
        val (dx, dy) = onScreenCorrection(bounds, root, x * density to y * density)
        if (dx != 0f || dy != 0f) {
            layout.nudge(key, dx / density, dy / density)
            layout.saveOffset(key)
        }
    }
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
internal fun Modifier.holdToEditLayout(): Modifier {
    val layout = LocalFlyScreenState.current.layout
    return if (layout.editing || layout.locked) this else onHold { layout.startEditing() }
}

@Composable
private fun jiggleAngle(key: String): Float {
    val swing by rememberInfiniteTransition(label = "jiggle").animateFloat(
        initialValue = -JIGGLE_DEGREES,
        targetValue = JIGGLE_DEGREES,
        animationSpec = infiniteRepeatable(tween(JIGGLE_MILLIS + (key.hashCode() and JIGGLE_SPREAD)), RepeatMode.Reverse),
        label = "jiggle",
    )
    return swing
}

internal fun clampedDrag(left: Float, top: Float, right: Float, bottom: Float, width: Float, height: Float, dx: Float, dy: Float): Pair<Float, Float> =
    dx.coerceIn(-left, (width - right).coerceAtLeast(-left)) to dy.coerceIn(-top, (height - bottom).coerceAtLeast(-top))

@Composable
internal fun LayoutDragArea(key: String, modifier: Modifier = Modifier) {
    val layout = LocalFlyScreenState.current.layout
    val density = LocalDensity.current.density
    val placedKey = layoutKey(key)
    var bounds by remember { mutableStateOf(Rect.Zero) }
    var root by remember { mutableStateOf(Size.Zero) }
    Box(
        modifier
            .onRootBounds { seen, size ->
                bounds = seen
                root = size
            }
            .pointerInput(placedKey) {
                detectDragGestures(onDragEnd = { layout.saveOffset(placedKey) }, onDragCancel = { layout.saveOffset(placedKey) }) { change, drag ->
                    change.consume()
                    val (dx, dy) = clampedDrag(bounds.left, bounds.top, bounds.right, bounds.bottom, root.width, root.height, drag.x, drag.y)
                    layout.nudge(placedKey, dx / density, dy / density)
                }
            },
    )
}

@Composable
internal fun layoutPlacement(key: String, keepOnScreen: Boolean): Modifier {
    val layout = LocalFlyScreenState.current.layout
    val placedKey = layoutKey(key)
    val moved = layout.offsets[placedKey] ?: (0f to 0f)
    val angle = if (layout.editing) jiggleAngle(key) else 0f
    val clamped = if (keepOnScreen) keptOnScreen(placedKey) else Modifier
    return Modifier.offset(moved.first.dp, moved.second.dp).then(clamped).graphicsLayer { rotationZ = angle }
}

@Composable
internal fun LayoutPipEditor(key: String, shape: Shape, modifier: Modifier = Modifier) {
    if (!LocalFlyScreenState.current.layout.editing) return
    LayoutDragArea(key, modifier.border(2.dp, MaterialTheme.colorScheme.primary, shape))
}

@Composable
internal fun LayoutWidget(key: String, movable: Boolean = true, hideable: Boolean = true, content: @Composable () -> Unit) {
    val flyScreen = LocalFlyScreenState.current
    val layout = flyScreen.layout
    val editing = layout.editing
    val hidden = key in layout.hidden
    if (hidden && !editing) return
    val avoided = LocalAvoidedByVideoMessage.current
    val placedKey = layoutKey(key)
    val moved = layout.offsets[placedKey] ?: (0f to 0f)
    val angle = if (editing) jiggleAngle(key) else 0f
    var shown by remember { mutableStateOf(false) }
    val decorated = editing && shown
    Box(
        Modifier
            .offset(moved.first.dp, moved.second.dp)
            .then(if (movable) keptOnScreen(placedKey) else Modifier)
            .holdToEditLayout()
            .graphicsLayer { rotationZ = if (decorated) angle else 0f }
            .then(if (avoided) Modifier.avoidedByVideoMessage(key) else Modifier),
    ) {
        Box(
            Modifier
                .onSizeChanged { shown = it.width > 0 && it.height > 0 }
                .then(if (decorated) Modifier.border(1.dp, MaterialTheme.colorScheme.outline, MaterialTheme.shapes.medium) else Modifier)
                .alpha(if (hidden) HIDDEN_ALPHA else 1f)
                .then(if (LocalFlyOsd.current) Modifier.osdShadow() else Modifier),
        ) { content() }
        if (decorated && movable) LayoutDragArea(key, Modifier.matchParentSize())
        if (decorated && hideable) Surface(
            onClick = { layout.setHidden(key, !hidden) },
            modifier = Modifier.align(Alignment.TopEnd).size(BADGE_SIZE),
            shape = CircleShape,
            color = if (hidden) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.inverseSurface,
            contentColor = if (hidden) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.inverseOnSurface,
            shadowElevation = 2.dp,
        ) {
            Icon(if (hidden) Icons.Default.Add else Icons.Default.Close, contentDescription = if (hidden) "Show" else "Hide", modifier = Modifier.padding(4.dp))
        }
        if (decorated && !hideable) Surface(
            modifier = Modifier.align(Alignment.TopEnd).size(BADGE_SIZE),
            shape = CircleShape,
            color = MaterialTheme.colorScheme.surfaceContainerHighest,
            contentColor = MaterialTheme.colorScheme.onSurfaceVariant,
            shadowElevation = 2.dp,
        ) {
            Icon(Icons.Default.Lock, contentDescription = "Always shown", modifier = Modifier.padding(5.dp))
        }
    }
}

@Composable
internal fun OverlayEditBar(modifier: Modifier = Modifier) {
    val layout = LocalFlyScreenState.current.layout
    if (!layout.editing) return
    BackHandler { layout.editing = false }
    val context = LocalContext.current
    val classView by one.aircast.android.bridge.qgcPath(INSTRUMENTS_VIEW)
    val vehicleClass = instrumentVehicleClass(classView)
    Surface(modifier.fillMaxWidth(), shape = MaterialTheme.shapes.large, color = MaterialTheme.colorScheme.surfaceContainerHigh, shadowElevation = 3.dp) {
        Row(Modifier.padding(horizontal = 8.dp, vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
            TextButton(onClick = {
                layout.valueSize = nextValueSize(layout.valueSize)
                writeValueSize(context, vehicleClass, layout.valueSize)
            }) { Text(valueSizePillText(layout.valueSize)) }
            var armed by remember { mutableStateOf(false) }
            LaunchedEffect(armed) {
                if (armed) {
                    delay(RESET_ARM_MILLIS)
                    armed = false
                }
            }
            TextButton(
                onClick = {
                    val tap = resetTap(armed)
                    if (tap.reset) layout.reset()
                    armed = tap.armed
                },
                colors = if (armed) ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error) else ButtonDefaults.textButtonColors(),
            ) { Text(resetPillText(armed)) }
            Spacer(Modifier.weight(1f))
            Button(onClick = { layout.editing = false }) { Text("Done") }
        }
    }
}
