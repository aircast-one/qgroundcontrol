package one.aircast.android.ui

import android.content.Context
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
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
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

internal object OverlayLayout {
    var editing by mutableStateOf(false)
    var hidden by mutableStateOf(emptySet<String>())
    var indicatorOrder by mutableStateOf(emptyList<String>())
    var valueSize by mutableStateOf(ValueSize.Default)
    var offsets by mutableStateOf(emptyMap<String, Pair<Float, Float>>())
    var loaded = false
}

internal fun loadLayoutOnce(stored: () -> Map<String, *>) {
    if (OverlayLayout.loaded) return
    val read = stored()
    OverlayLayout.hidden = hiddenKeys(read)
    OverlayLayout.offsets = storedOffsets(read)
    OverlayLayout.loaded = true
}

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
    OverlayLayout.offsets = emptyMap()
}

internal fun storedOffsets(stored: Map<String, *>): Map<String, Pair<Float, Float>> =
    stored.filterKeys { it.startsWith(OFFSET_PREFIX) }.mapNotNull { (key, value) ->
        (value as? String)?.split(",")?.mapNotNull { it.toFloatOrNull() }?.takeIf { it.size == 2 }?.let { key.removePrefix(OFFSET_PREFIX) to (it[0] to it[1]) }
    }.toMap()

private fun saveOffset(context: Context, key: String) {
    OverlayLayout.offsets[key]?.let { (x, y) -> store(context).edit().putString(OFFSET_PREFIX + key, "$x,$y").apply() }
}

private fun nudge(key: String, dx: Float, dy: Float) {
    val current = OverlayLayout.offsets[key] ?: (0f to 0f)
    OverlayLayout.offsets += key to (current.first + dx to current.second + dy)
}

private fun Modifier.onRootBounds(seen: (Rect, Size) -> Unit): Modifier = onGloballyPositioned {
    seen(Rect(it.positionInRoot(), it.size.toSize()), it.findRootCoordinates().size.toSize())
}

@Composable
private fun layoutKey(key: String): String = orientedKey(key, !flyIsPortrait())

@Composable
private fun keptOnScreen(key: String): Modifier {
    val context = LocalContext.current
    val density = LocalDensity.current.density
    return Modifier.onRootBounds { bounds, root ->
        val (x, y) = OverlayLayout.offsets[key] ?: (0f to 0f)
        val (dx, dy) = onScreenCorrection(bounds, root, x * density to y * density)
        if (dx != 0f || dy != 0f) {
            nudge(key, dx / density, dy / density)
            saveOffset(context, key)
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
    val context = LocalContext.current
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
                detectDragGestures(onDragEnd = { saveOffset(context, placedKey) }, onDragCancel = { saveOffset(context, placedKey) }) { change, drag ->
                    change.consume()
                    val (dx, dy) = clampedDrag(bounds.left, bounds.top, bounds.right, bounds.bottom, root.width, root.height, drag.x, drag.y)
                    nudge(placedKey, dx / density, dy / density)
                }
            },
    )
}

@Composable
internal fun layoutPlacement(key: String, keepOnScreen: Boolean): Modifier {
    val context = LocalContext.current
    loadLayoutOnce { store(context).all }
    val placedKey = layoutKey(key)
    val moved = OverlayLayout.offsets[placedKey] ?: (0f to 0f)
    val angle = if (OverlayLayout.editing) jiggleAngle(key) else 0f
    val clamped = if (keepOnScreen) keptOnScreen(placedKey) else Modifier
    return Modifier.offset(moved.first.dp, moved.second.dp).then(clamped).graphicsLayer { rotationZ = angle }
}

@Composable
internal fun LayoutPipEditor(key: String, shape: Shape, modifier: Modifier = Modifier) {
    if (!OverlayLayout.editing) return
    LayoutDragArea(key, modifier.border(2.dp, MaterialTheme.colorScheme.primary, shape))
}

@Composable
internal fun LayoutWidget(key: String, movable: Boolean = true, hideable: Boolean = true, content: @Composable () -> Unit) {
    val context = LocalContext.current
    loadLayoutOnce { store(context).all }
    val editing = OverlayLayout.editing
    val hidden = key in OverlayLayout.hidden
    if (hidden && !editing) return
    val placedKey = layoutKey(key)
    val moved = OverlayLayout.offsets[placedKey] ?: (0f to 0f)
    val angle = if (editing) jiggleAngle(key) else 0f
    var shown by remember { mutableStateOf(false) }
    val decorated = editing && shown
    Box(
        Modifier
            .offset(moved.first.dp, moved.second.dp)
            .then(if (movable) keptOnScreen(placedKey) else Modifier)
            .then(if (hideable && !editing) Modifier.onHold { OverlayLayout.editing = true } else Modifier)
            .graphicsLayer { rotationZ = if (decorated) angle else 0f },
    ) {
        Box(
            Modifier
                .onSizeChanged { shown = it.width > 0 && it.height > 0 }
                .then(if (decorated) Modifier.border(1.dp, MaterialTheme.colorScheme.outline, MaterialTheme.shapes.medium) else Modifier)
                .alpha(if (hidden) HIDDEN_ALPHA else 1f),
        ) { content() }
        if (decorated && movable) LayoutDragArea(key, Modifier.matchParentSize())
        if (decorated && hideable) Surface(
            onClick = { setHidden(context, key, !hidden) },
            modifier = Modifier.align(Alignment.TopEnd).size(BADGE_SIZE),
            shape = CircleShape,
            color = if (hidden) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.inverseSurface,
            contentColor = if (hidden) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.inverseOnSurface,
            shadowElevation = 2.dp,
        ) {
            Icon(if (hidden) Icons.Default.Add else Icons.Default.Close, contentDescription = if (hidden) "Show" else "Hide", modifier = Modifier.padding(4.dp))
        }
    }
}

@Composable
internal fun OverlayEditBar(modifier: Modifier = Modifier) {
    if (!OverlayLayout.editing) return
    BackHandler { OverlayLayout.editing = false }
    val context = LocalContext.current
    val classView by one.aircast.android.bridge.qgcPath(INSTRUMENTS_VIEW)
    val vehicleClass = instrumentVehicleClass(classView)
    Surface(modifier.fillMaxWidth(), shape = MaterialTheme.shapes.large, color = MaterialTheme.colorScheme.surfaceContainerHigh, shadowElevation = 3.dp) {
        Row(Modifier.padding(horizontal = 8.dp, vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
            TextButton(onClick = {
                OverlayLayout.valueSize = nextValueSize(OverlayLayout.valueSize)
                writeValueSize(context, vehicleClass, OverlayLayout.valueSize)
            }) { Text(valueSizePillText(OverlayLayout.valueSize)) }
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
                    if (tap.reset) resetLayout(context)
                    armed = tap.armed
                },
                colors = if (armed) ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error) else ButtonDefaults.textButtonColors(),
            ) { Text(resetPillText(armed)) }
            Spacer(Modifier.weight(1f))
            Button(onClick = { OverlayLayout.editing = false }) { Text("Done") }
        }
    }
}
